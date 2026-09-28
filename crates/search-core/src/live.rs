use crate::attributes::AttributeIndex;
use crate::delta::{delta_path, load_latest_delta, DeltaOp, DeltaRecord};
use crate::filters::{matches_filters, ParsedSearchQuery};
use crate::index_lock::IndexMutationGuard;
use crate::query::{fuzzy_distance, fuzzy_seed, relevance_score};
use crate::relationship::relation_for_query;
use crate::store::{normalize_name, SearchStore};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

pub const DEFAULT_MAX_DELTA_ENTRIES: usize = 65_536;
const LIVE_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
const PATH_NODE_CACHE_LIMIT: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveSearchHit {
    pub file_id: u64,
    pub parent_id: u64,
    pub size_bytes: u64,
    pub flags: u16,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RefreshOutcome {
    pub base_changed: bool,
    pub delta_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone)]
struct PathNode {
    parent_id: u64,
    name: String,
}

#[derive(Debug)]
pub struct LiveSearchStore {
    index_path: PathBuf,
    base: SearchStore,
    delta: HashMap<u64, DeltaRecord>,
    max_delta_entries: usize,
    base_stamp: Option<FileStamp>,
    delta_stamp: Option<FileStamp>,
    last_refresh_check: Instant,
    path_node_cache: HashMap<u64, PathNode>,
}

impl LiveSearchStore {
    pub fn open(index_path: impl AsRef<Path>) -> io::Result<Self> {
        Self::open_with_limit(index_path, DEFAULT_MAX_DELTA_ENTRIES)
    }

    pub fn open_with_limit(
        index_path: impl AsRef<Path>,
        max_delta_entries: usize,
    ) -> io::Result<Self> {
        let index_path = index_path.as_ref().to_path_buf();
        let delta_file = delta_path(&index_path);
        Ok(Self {
            base: SearchStore::open(&index_path)?,
            delta: load_latest_delta(&delta_file, max_delta_entries)?,
            max_delta_entries,
            base_stamp: file_stamp(&index_path)?,
            delta_stamp: file_stamp(&delta_file)?,
            index_path,
            last_refresh_check: Instant::now(),
            path_node_cache: HashMap::new(),
        })
    }

    pub fn refresh_now(&mut self) -> io::Result<RefreshOutcome> {
        let delta_file = delta_path(&self.index_path);
        let next_base_stamp = file_stamp(&self.index_path)?;
        let next_delta_stamp = file_stamp(&delta_file)?;
        let base_changed = next_base_stamp != self.base_stamp;
        let delta_changed = next_delta_stamp != self.delta_stamp;
        if !base_changed && !delta_changed {
            self.last_refresh_check = Instant::now();
            return Ok(RefreshOutcome::default());
        }

        // Writers hold this lock across sync/compaction/family publication. If a
        // change is observed while publication is still in progress, keep the old
        // consistent view and let the next refresh retry instead of opening a mixed
        // generation.
        let _guard = IndexMutationGuard::try_acquire(&self.index_path)?;
        let stable_base_stamp = file_stamp(&self.index_path)?;
        let stable_delta_stamp = file_stamp(&delta_file)?;
        let stable_base_changed = stable_base_stamp != self.base_stamp;
        let stable_delta_changed = stable_delta_stamp != self.delta_stamp;

        if stable_base_changed {
            let new_base = SearchStore::open(&self.index_path)?;
            let new_delta = load_latest_delta(&delta_file, self.max_delta_entries)?;
            self.base = new_base;
            self.delta = new_delta;
        } else if stable_delta_changed {
            self.delta = load_latest_delta(&delta_file, self.max_delta_entries)?;
        }
        if stable_base_changed || stable_delta_changed {
            self.path_node_cache.clear();
        }
        self.base_stamp = stable_base_stamp;
        self.delta_stamp = stable_delta_stamp;
        self.last_refresh_check = Instant::now();
        Ok(RefreshOutcome {
            base_changed: stable_base_changed,
            delta_changed: stable_delta_changed,
        })
    }

    pub fn refresh_if_due(&mut self) -> io::Result<RefreshOutcome> {
        if self.last_refresh_check.elapsed() < LIVE_REFRESH_INTERVAL {
            return Ok(RefreshOutcome::default());
        }
        match self.refresh_now() {
            Ok(outcome) => Ok(outcome),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                self.last_refresh_check = Instant::now();
                Ok(RefreshOutcome::default())
            }
            Err(error) => Err(error),
        }
    }

    pub fn delta_entries(&self) -> usize {
        self.delta.len()
    }

    pub const fn base_record_count(&self) -> u64 {
        self.base.record_count()
    }

    pub fn memory_hint_bytes(&self) -> usize {
        self.base.memory_hint_bytes()
            + self.delta.capacity() * std::mem::size_of::<(u64, DeltaRecord)>()
            + self
                .delta
                .values()
                .map(|record| record.name.capacity())
                .sum::<usize>()
            + self.path_node_cache.capacity() * std::mem::size_of::<(u64, PathNode)>()
            + self
                .path_node_cache
                .values()
                .map(|node| node.name.capacity())
                .sum::<usize>()
    }

    pub fn search_prefix(&mut self, query: &str, limit: usize) -> io::Result<Vec<LiveSearchHit>> {
        self.refresh_if_due()?;
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let normalized = normalize_name(query);
        let mut hits = Vec::with_capacity(limit.min(64));
        let base_hits = self
            .base
            .search_prefix(query, limit.saturating_mul(4).max(limit))?;
        for hit in base_hits {
            if self.delta.contains_key(&hit.record.file_id) {
                continue;
            }
            hits.push(LiveSearchHit {
                file_id: hit.record.file_id,
                parent_id: hit.record.parent_id,
                size_bytes: hit.record.size_bytes,
                flags: hit.record.flags,
                name: hit.name,
            });
            if hits.len() >= limit {
                break;
            }
        }
        if hits.len() < limit {
            for record in self.delta.values() {
                if record.op != DeltaOp::Upsert
                    || !normalize_name(&record.name).starts_with(&normalized)
                {
                    continue;
                }
                hits.push(LiveSearchHit {
                    file_id: record.file_id,
                    parent_id: record.parent_id,
                    size_bytes: record.size_bytes,
                    flags: record.flags,
                    name: record.name.clone(),
                });
                if hits.len() >= limit {
                    break;
                }
            }
        }
        hits.sort_unstable_by(|a, b| {
            normalize_name(&a.name)
                .cmp(&normalize_name(&b.name))
                .then(a.file_id.cmp(&b.file_id))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn search_exact(&mut self, query: &str, limit: usize) -> io::Result<Vec<LiveSearchHit>> {
        let normalized = normalize_name(query);
        Ok(self
            .search_prefix(query, limit.saturating_mul(8).max(limit))?
            .into_iter()
            .filter(|hit| normalize_name(&hit.name) == normalized)
            .take(limit)
            .collect())
    }

    pub fn get_by_file_id(&mut self, file_id: u64) -> io::Result<Option<LiveSearchHit>> {
        self.refresh_if_due()?;
        if let Some(record) = self.delta.get(&file_id) {
            if record.op == DeltaOp::Delete {
                return Ok(None);
            }
            return Ok(Some(LiveSearchHit {
                file_id: record.file_id,
                parent_id: record.parent_id,
                size_bytes: record.size_bytes,
                flags: record.flags,
                name: record.name.clone(),
            }));
        }
        Ok(self.base.get_by_file_id(file_id)?.map(|hit| LiveSearchHit {
            file_id: hit.record.file_id,
            parent_id: hit.record.parent_id,
            size_bytes: hit.record.size_bytes,
            flags: hit.record.flags,
            name: hit.name,
        }))
    }

    pub fn search_ranked(&mut self, query: &str, limit: usize) -> io::Result<Vec<LiveSearchHit>> {
        let mut hits = self.search_prefix(query, limit.saturating_mul(2).max(limit))?;
        hits.sort_unstable_by(|a, b| {
            relevance_score(query, &b.name, b.flags)
                .cmp(&relevance_score(query, &a.name, a.flags))
                .then(a.name.len().cmp(&b.name.len()))
                .then(a.file_id.cmp(&b.file_id))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn search_fuzzy(
        &mut self,
        query: &str,
        max_distance: usize,
        limit: usize,
    ) -> io::Result<Vec<LiveSearchHit>> {
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let seed = fuzzy_seed(query);
        let candidates = self.search_prefix(&seed, 4096)?;
        let mut scored = Vec::with_capacity(candidates.len());
        for hit in candidates {
            let Some(distance) = fuzzy_distance(query, &hit.name, max_distance) else {
                continue;
            };
            let relevance = relevance_score(query, &hit.name, hit.flags);
            scored.push((distance, relevance, hit));
        }
        scored.sort_unstable_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.2.file_id.cmp(&b.2.file_id))
        });
        scored.truncate(limit);
        Ok(scored.into_iter().map(|(_, _, hit)| hit).collect())
    }

    pub fn search_related(&mut self, query: &str, limit: usize) -> io::Result<Vec<LiveSearchHit>> {
        let Some(relation) = relation_for_query(query) else {
            return self.search_ranked(query, limit);
        };
        let mut by_id = HashMap::<u64, LiveSearchHit>::new();
        let per_alias = (limit.saturating_mul(2) / relation.aliases.len().max(1)).max(8);
        for alias in relation.aliases {
            for hit in self.search_prefix(alias, per_alias)? {
                by_id.entry(hit.file_id).or_insert(hit);
            }
        }
        let mut hits: Vec<_> = by_id.into_values().collect();
        hits.sort_unstable_by(|a, b| {
            relevance_score(query, &b.name, b.flags)
                .cmp(&relevance_score(query, &a.name, a.flags))
                .then(a.file_id.cmp(&b.file_id))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn search_filtered(
        &mut self,
        parsed: &ParsedSearchQuery,
        mut attributes: Option<&mut AttributeIndex>,
        limit: usize,
        scan_budget: usize,
    ) -> io::Result<Vec<LiveSearchHit>> {
        self.refresh_if_due()?;
        if limit == 0 {
            return Ok(Vec::new());
        }
        if parsed.filters.is_empty() && !parsed.text.is_empty() {
            return self.search_ranked(&parsed.text, limit);
        }
        let candidate_limit = limit.saturating_mul(4).clamp(128, 2048);
        let mut candidates = if parsed.text.is_empty() {
            self.scan_live_candidates(scan_budget.max(limit))?
        } else {
            self.search_ranked(&parsed.text, candidate_limit)?
        };
        if parsed.filters.is_empty() {
            candidates.truncate(limit);
            return Ok(candidates);
        }

        let mut results = Vec::with_capacity(limit.min(64));
        let needs_path = parsed.filters.needs_path();
        for hit in candidates {
            let path = if needs_path {
                self.reconstruct_path(&hit, 256)
                    .unwrap_or_else(|_| hit.name.clone())
            } else {
                String::new()
            };
            let attribute = match attributes.as_deref_mut() {
                Some(index) => index.get(hit.file_id)?,
                None => None,
            };
            if matches_filters(&parsed.filters, &hit.name, &path, hit.flags, attribute) {
                results.push(hit);
                if results.len() >= limit {
                    break;
                }
            }
        }
        Ok(results)
    }

    fn scan_live_candidates(&mut self, scan_budget: usize) -> io::Result<Vec<LiveSearchHit>> {
        let budget = scan_budget.max(1);
        let mut hits = Vec::with_capacity(budget.min(4096));
        let count = self.base.record_count().min(budget as u64);
        for record_index in 0..count {
            let record = self.base.read_record(record_index)?;
            if self.delta.contains_key(&record.file_id) {
                continue;
            }
            let name = self.base.read_name(record)?;
            hits.push(LiveSearchHit {
                file_id: record.file_id,
                parent_id: record.parent_id,
                size_bytes: record.size_bytes,
                flags: record.flags,
                name,
            });
        }
        if hits.len() < budget {
            for record in self.delta.values() {
                if record.op != DeltaOp::Upsert {
                    continue;
                }
                hits.push(LiveSearchHit {
                    file_id: record.file_id,
                    parent_id: record.parent_id,
                    size_bytes: record.size_bytes,
                    flags: record.flags,
                    name: record.name.clone(),
                });
                if hits.len() >= budget {
                    break;
                }
            }
        }
        Ok(hits)
    }

    fn path_node(&mut self, file_id: u64) -> io::Result<Option<PathNode>> {
        if let Some(delta) = self.delta.get(&file_id) {
            if delta.op == DeltaOp::Delete {
                return Ok(None);
            }
            return Ok(Some(PathNode {
                parent_id: delta.parent_id,
                name: delta.name.clone(),
            }));
        }
        if let Some(node) = self.path_node_cache.get(&file_id) {
            return Ok(Some(node.clone()));
        }
        let Some(record_index) = self.base.lookup_file_id(file_id)? else {
            return Ok(None);
        };
        let record = self.base.read_record(record_index)?;
        let node = PathNode {
            parent_id: record.parent_id,
            name: self.base.read_name(record)?,
        };
        if self.path_node_cache.len() >= PATH_NODE_CACHE_LIMIT {
            self.path_node_cache.clear();
        }
        self.path_node_cache.insert(file_id, node.clone());
        Ok(Some(node))
    }

    pub fn reconstruct_path(
        &mut self,
        hit: &LiveSearchHit,
        max_depth: usize,
    ) -> io::Result<String> {
        let mut pieces = vec![hit.name.clone()];
        let mut parent_id = hit.parent_id;
        let mut last_id = hit.file_id;
        for _ in 0..max_depth.saturating_sub(1) {
            if parent_id == 0 || parent_id == last_id {
                break;
            }
            let Some(node) = self.path_node(parent_id)? else {
                break;
            };
            if !node.name.is_empty() {
                pieces.push(node.name);
            }
            last_id = parent_id;
            parent_id = node.parent_id;
        }
        pieces.reverse();
        Ok(pieces.join("\\"))
    }
}

fn file_stamp(path: &Path) -> io::Result<Option<FileStamp>> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(FileStamp {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        })),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{DeltaRecord, DeltaWriter};
    use crate::store::{BuildOptions, IndexBuilder, InputRecord, FLAG_DIRECTORY};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("search-tool-live-{name}-{n}.stidx"))
    }

    #[test]
    fn delta_rename_and_delete_override_base() {
        let path = temp("overlay");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        builder
            .push(InputRecord {
                file_id: 5,
                parent_id: 5,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "C:",
            })
            .unwrap();
        builder
            .push(InputRecord {
                file_id: 10,
                parent_id: 5,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "work",
            })
            .unwrap();
        builder
            .push(InputRecord {
                file_id: 20,
                parent_id: 10,
                size_bytes: 1,
                flags: 0,
                name: "old.txt",
            })
            .unwrap();
        builder
            .push(InputRecord {
                file_id: 21,
                parent_id: 10,
                size_bytes: 1,
                flags: 0,
                name: "gone.txt",
            })
            .unwrap();
        builder.finish().unwrap();
        let dpath = delta_path(&path);
        let mut delta = DeltaWriter::open(&dpath).unwrap();
        delta
            .append(&DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 20,
                parent_id: 10,
                size_bytes: 1,
                flags: 0,
                name: "new.txt".into(),
            })
            .unwrap();
        delta
            .append(&DeltaRecord {
                op: DeltaOp::Delete,
                file_id: 21,
                parent_id: 10,
                size_bytes: 0,
                flags: 0,
                name: String::new(),
            })
            .unwrap();
        delta.sync().unwrap();
        drop(delta);

        let mut store = LiveSearchStore::open(&path).unwrap();
        assert!(store.search_prefix("old", 10).unwrap().is_empty());
        assert!(store.search_prefix("gone", 10).unwrap().is_empty());
        let hits = store.search_prefix("new", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            store.reconstruct_path(&hits[0], 32).unwrap(),
            "C:\\work\\new.txt"
        );

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(crate::store::names_path(&path));
        let _ = fs::remove_file(crate::store::ids_path(&path));
        let _ = fs::remove_file(dpath);
    }

    #[test]
    fn filtered_search_uses_attribute_sidecar() {
        use crate::attributes::{attribute_index_path, AttributeEntry, AttributeIndexBuilder};
        use crate::filters::parse_search_query;

        let path = temp("filtered");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (id, name) in [(10, "node.exe"), (11, "node.txt")] {
            builder
                .push(InputRecord {
                    file_id: id,
                    parent_id: 0,
                    size_bytes: 0,
                    flags: 0,
                    name,
                })
                .unwrap();
        }
        builder.finish().unwrap();

        let attrs_path = attribute_index_path(&path);
        let mut attrs_builder = AttributeIndexBuilder::create(&attrs_path, 1024).unwrap();
        attrs_builder
            .push(AttributeEntry {
                file_id: 10,
                size_bytes: 2 * 1024 * 1024,
                modified_unix_secs: 1_700_000_000,
                platform_attributes: 0,
            })
            .unwrap();
        attrs_builder
            .push(AttributeEntry {
                file_id: 11,
                size_bytes: 512,
                modified_unix_secs: 1_700_000_000,
                platform_attributes: 0,
            })
            .unwrap();
        attrs_builder.finish().unwrap();

        let mut store = LiveSearchStore::open(&path).unwrap();
        let mut attrs = AttributeIndex::open(&attrs_path).unwrap();
        let parsed = parse_search_query("node ext:exe size:>1mb");
        let hits = store
            .search_filtered(&parsed, Some(&mut attrs), 10, 8192)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "node.exe");

        for candidate in [
            path.clone(),
            crate::store::names_path(&path),
            crate::store::ids_path(&path),
            attrs_path.clone(),
            crate::attributes::attribute_checkpoints_path(&attrs_path),
        ] {
            let _ = fs::remove_file(candidate);
        }
    }
    #[test]
    fn fuzzy_and_related_find_expected_candidates() {
        let path = temp("fuzzy-related");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (id, name) in [
            (1, "notepad.exe"),
            (2, "node.exe"),
            (3, "npm"),
            (4, "package.json"),
            (5, "node_modules"),
        ] {
            builder
                .push(InputRecord {
                    file_id: id,
                    parent_id: 0,
                    size_bytes: 0,
                    flags: 0,
                    name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        let mut store = LiveSearchStore::open(&path).unwrap();
        let fuzzy = store.search_fuzzy("notpad", 2, 5).unwrap();
        assert_eq!(fuzzy[0].name, "notepad.exe");
        let related = store.search_related("node js", 10).unwrap();
        assert!(related.iter().any(|h| h.name == "package.json"));
        assert!(related.iter().any(|h| h.name == "npm"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(crate::store::names_path(&path));
        let _ = fs::remove_file(crate::store::ids_path(&path));
    }
    #[test]
    fn refresh_now_observes_delta_written_after_open() {
        let path = temp("live-refresh");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        builder
            .push(InputRecord {
                file_id: 1,
                parent_id: 1,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "root",
            })
            .unwrap();
        builder.finish().unwrap();

        let mut live = LiveSearchStore::open(&path).unwrap();
        assert!(live.search_exact("fresh.txt", 4).unwrap().is_empty());

        let mut delta = DeltaWriter::open(delta_path(&path)).unwrap();
        delta
            .append(&DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 2,
                parent_id: 1,
                size_bytes: 0,
                flags: 0,
                name: "fresh.txt".into(),
            })
            .unwrap();
        delta.sync().unwrap();
        drop(delta);

        let outcome = live.refresh_now().unwrap();
        assert!(outcome.delta_changed);
        assert_eq!(live.search_exact("fresh.txt", 4).unwrap().len(), 1);

        for suffix in [
            "",
            ".names",
            ".ids",
            ".ncp",
            ".icp",
            ".delta",
            ".mutation.lock",
        ] {
            let mut value = path.as_os_str().to_os_string();
            value.push(suffix);
            let _ = fs::remove_file(PathBuf::from(value));
        }
    }
    #[test]
    fn refresh_now_reopens_base_after_compaction() {
        let path = temp("base-refresh");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        builder
            .push(InputRecord {
                file_id: 1,
                parent_id: 1,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "root",
            })
            .unwrap();
        builder
            .push(InputRecord {
                file_id: 2,
                parent_id: 1,
                size_bytes: 0,
                flags: 0,
                name: "before.txt",
            })
            .unwrap();
        builder.finish().unwrap();

        let mut live = LiveSearchStore::open(&path).unwrap();
        let mut delta = DeltaWriter::open(delta_path(&path)).unwrap();
        delta
            .append(&DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 2,
                parent_id: 1,
                size_bytes: 0,
                flags: 0,
                name: "after.txt".into(),
            })
            .unwrap();
        delta.sync().unwrap();
        drop(delta);
        crate::maintenance::compact_index(&path).unwrap();

        let outcome = live.refresh_now().unwrap();
        assert!(outcome.base_changed);
        assert!(live.search_exact("before.txt", 4).unwrap().is_empty());
        assert_eq!(live.search_exact("after.txt", 4).unwrap().len(), 1);

        for suffix in [
            "",
            ".names",
            ".ids",
            ".ncp",
            ".icp",
            ".delta",
            ".mutation.lock",
            ".compact.pending",
        ] {
            let mut value = path.as_os_str().to_os_string();
            value.push(suffix);
            let _ = fs::remove_file(PathBuf::from(value));
        }
    }
}
