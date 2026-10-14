use crate::attributes::AttributeIndex;
use crate::delta::{delta_path, load_latest_delta, DeltaOp, DeltaRecord};
use crate::filters::{matches_filters, ParsedSearchQuery};
use crate::index_lock::IndexMutationReadGuard;
use crate::query::{fuzzy_distance, fuzzy_seed, relevance_score};
use crate::relationship::relation_for_query;
use crate::store::{normalize_name, SearchStore, FLAG_DIRECTORY};
use std::collections::{BTreeMap, HashMap, HashSet};
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
    flags: u16,
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

        // SearchStore::open owns the base-family publication snapshot. The
        // delta overlay is append-only and independently tolerant of a partial
        // final record, so live search does not need a broader publish lock.
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
        let _guard = IndexMutationReadGuard::try_acquire(&self.index_path)?;
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
        // Merge the live overlay and base candidates before applying the
        // final cap. A full base page must never hide a fresher delta hit.
        // The ordered top-K map retains at most `limit` hits in memory.
        let mut best = BTreeMap::<(String, u64), LiveSearchHit>::new();
        let base_hits = self
            .base
            .search_prefix(query, limit.saturating_mul(4).max(limit))?;
        for hit in base_hits {
            if self.delta.contains_key(&hit.record.file_id) {
                continue;
            }
            let key = (normalize_name(&hit.name), hit.record.file_id);
            best.insert(
                key,
                LiveSearchHit {
                    file_id: hit.record.file_id,
                    parent_id: hit.record.parent_id,
                    size_bytes: hit.record.size_bytes,
                    flags: hit.record.flags,
                    name: hit.name,
                },
            );
            if best.len() > limit {
                best.pop_last();
            }
        }
        for record in self.delta.values() {
            if record.op != DeltaOp::Upsert {
                continue;
            }
            let name_key = normalize_name(&record.name);
            if !name_key.starts_with(&normalized) {
                continue;
            }
            let key = (name_key, record.file_id);
            if best.len() == limit
                && best
                    .last_key_value()
                    .is_some_and(|(worst, _)| key >= *worst)
            {
                continue;
            }
            best.insert(
                key,
                LiveSearchHit {
                    file_id: record.file_id,
                    parent_id: record.parent_id,
                    size_bytes: record.size_bytes,
                    flags: record.flags,
                    name: record.name.clone(),
                },
            );
            if best.len() > limit {
                best.pop_last();
            }
        }
        Ok(best.into_values().collect())
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
        attributes: Option<&mut AttributeIndex>,
        limit: usize,
        scan_budget: usize,
    ) -> io::Result<Vec<LiveSearchHit>> {
        self.search_filtered_with_root_prefix(parsed, attributes, limit, scan_budget, None)
    }

    // Multi-volume filters with a drive prefix must match from that volume's
    // root, not at an arbitrary position in a relative indexed path. Apply
    // this restriction during candidate filtering, before the result limit.
    pub(crate) fn search_filtered_with_root_prefix(
        &mut self,
        parsed: &ParsedSearchQuery,
        mut attributes: Option<&mut AttributeIndex>,
        limit: usize,
        scan_budget: usize,
        root_prefix: Option<&str>,
    ) -> io::Result<Vec<LiveSearchHit>> {
        self.refresh_if_due()?;
        if limit == 0 {
            return Ok(Vec::new());
        }
        if parsed.filters.is_empty() && root_prefix.is_none() && !parsed.text.is_empty() {
            return self.search_ranked(&parsed.text, limit);
        }
        let root_prefix = root_prefix.map(normalize_name);
        let candidate_limit = limit.saturating_mul(4).clamp(128, 2048);
        let mut candidates = if parsed.text.is_empty() {
            self.scan_live_candidates(scan_budget.max(limit), None, root_prefix.as_deref())?
        } else {
            self.search_ranked(&parsed.text, candidate_limit)?
        };
        if parsed.filters.is_empty() && root_prefix.is_none() {
            candidates.truncate(limit);
            return Ok(candidates);
        }

        let mut results = Vec::with_capacity(limit.min(64));
        let mut matched_ids = HashSet::new();
        for hit in candidates {
            if self.filtered_candidate_matches(
                &hit,
                parsed,
                &mut attributes,
                root_prefix.as_deref(),
            )? {
                matched_ids.insert(hit.file_id);
                results.push(hit);
                if results.len() >= limit {
                    break;
                }
            }
        }

        // A scoped query can have hundreds of highly ranked *global* name
        // matches before the first in-folder match. Never equate the bounded
        // global top-N with the bounded set of scoped results. When the fast
        // name-index lookup did not fill the request, scan at most the caller's
        // explicit candidate budget and apply the identical path/attribute
        // checks to each matching name before counting it toward the limit.
        if root_prefix.is_some() && !parsed.text.is_empty() && results.len() < limit {
            let normalized_text = normalize_name(&parsed.text);
            for hit in self.scan_live_candidates(
                scan_budget.max(limit),
                Some(&normalized_text),
                root_prefix.as_deref(),
            )? {
                if !normalize_name(&hit.name).starts_with(&normalized_text)
                    || matched_ids.contains(&hit.file_id)
                {
                    continue;
                }
                if self.filtered_candidate_matches(
                    &hit,
                    parsed,
                    &mut attributes,
                    root_prefix.as_deref(),
                )? {
                    matched_ids.insert(hit.file_id);
                    results.push(hit);
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }
        Ok(results)
    }

    fn filtered_candidate_matches(
        &mut self,
        hit: &LiveSearchHit,
        parsed: &ParsedSearchQuery,
        attributes: &mut Option<&mut AttributeIndex>,
        root_prefix: Option<&str>,
    ) -> io::Result<bool> {
        let path = if parsed.filters.needs_path() || root_prefix.is_some() {
            // A broken parent chain must never be turned into a plausible
            // root-level hit by reconstructing a partial name.
            let Ok(path) = self.reconstruct_path(hit, 256) else {
                return Ok(false);
            };
            path
        } else {
            String::new()
        };
        let attribute = match attributes.as_deref_mut() {
            Some(index) => index.get(hit.file_id)?,
            None => None,
        };
        Ok(
            matches_filters(&parsed.filters, &hit.name, &path, hit.flags, attribute)
                && root_prefix.is_none_or(|prefix| Self::path_within_root(&path, prefix)),
        )
    }

    fn path_within_root(path: &str, normalized_root: &str) -> bool {
        // Apply the identical full-component containment rule during early
        // candidate admission and final result authorization.
        normalize_name(path)
            .strip_prefix(normalized_root)
            .is_some_and(|tail| {
                normalized_root.is_empty()
                    || tail.is_empty()
                    || normalized_root.ends_with('\\')
                    || tail.starts_with('\\')
            })
    }

    fn candidate_inside_root(
        &mut self,
        hit: &LiveSearchHit,
        normalized_root: &str,
        parent_paths: &mut HashMap<u64, Option<String>>,
    ) -> bool {
        // Root/parentless entries need the canonical validation path.
        if hit.parent_id == 0 || hit.parent_id == hit.file_id {
            return self
                .reconstruct_path(hit, 256)
                .is_ok_and(|path| Self::path_within_root(&path, normalized_root));
        }
        // Thousands of live children often share the same parent. Verify
        // that directory's complete chain only once per scan rather than
        // reconstructing the identical ancestry for every sibling.
        if !parent_paths.contains_key(&hit.parent_id) {
            let verified = self
                .path_node(hit.parent_id)
                .ok()
                .flatten()
                .and_then(|node| {
                    if node.flags & FLAG_DIRECTORY == 0 {
                        return None;
                    }
                    let parent = LiveSearchHit {
                        file_id: hit.parent_id,
                        parent_id: node.parent_id,
                        size_bytes: 0,
                        flags: node.flags,
                        name: node.name,
                    };
                    // A candidate can use up to 256 nodes, so its parent has
                    // at most 255. A malformed, cyclic or partial chain stays
                    // fail-closed and is never cached as a valid path.
                    self.reconstruct_path(&parent, 255).ok()
                });
            // Large indexes can have many distinct parent directories.
            // Keep the short-lived memo bounded like the underlying node
            // cache; every eviction only triggers fresh verification.
            if parent_paths.len() >= PATH_NODE_CACHE_LIMIT {
                parent_paths.clear();
            }
            parent_paths.insert(hit.parent_id, verified);
        }
        parent_paths
            .get(&hit.parent_id)
            .and_then(Option::as_deref)
            .is_some_and(|parent| {
                let full_path = if parent.is_empty() {
                    hit.name.clone()
                } else {
                    format!("{parent}\\{}", hit.name)
                };
                Self::path_within_root(&full_path, normalized_root)
            })
    }

    fn scan_live_candidates(
        &mut self,
        scan_budget: usize,
        normalized_name_prefix: Option<&str>,
        normalized_root_prefix: Option<&str>,
    ) -> io::Result<Vec<LiveSearchHit>> {
        let budget = scan_budget.max(1);
        let mut hits = Vec::with_capacity(budget.min(4096));
        let mut verified_parents = HashMap::<u64, Option<String>>::new();
        // The delta represents the current state of newly created and moved
        // files, not a low-priority tail of the base index. Under a bounded
        // scan, base-first iteration could consume every candidate slot and
        // silently starve the only in-scope delta entry. Include upserts
        // first, without treating tombstones as live candidates.
        // HashMap iteration is randomized, which previously changed which
        // updates won a small candidate budget between identical searches.
        // Sort references rather than cloning every full delta record.
        let mut upsert_ids: Vec<_> = self
            .delta
            .values()
            .filter(|record| {
                record.op == DeltaOp::Upsert
                    && normalized_name_prefix
                        .is_none_or(|prefix| normalize_name(&record.name).starts_with(prefix))
            })
            .map(|record| record.file_id)
            .collect();
        upsert_ids.sort_unstable();
        for id in upsert_ids {
            let record = &self.delta[&id];
            let hit = LiveSearchHit {
                file_id: record.file_id,
                parent_id: record.parent_id,
                size_bytes: record.size_bytes,
                flags: record.flags,
                name: record.name.clone(),
            };
            // A name match outside the Explorer folder must not consume a
            // scoped candidate slot before the only in-folder match.
            if normalized_root_prefix
                .is_some_and(|root| !self.candidate_inside_root(&hit, root, &mut verified_parents))
            {
                continue;
            }
            hits.push(hit);
            if hits.len() >= budget {
                return Ok(hits);
            }
        }
        let count = self.base.record_count().min(budget as u64);
        for record_index in 0..count {
            let record = self.base.read_record(record_index)?;
            if self.delta.contains_key(&record.file_id) {
                continue;
            }
            let name = self.base.read_name(record)?;
            if normalized_name_prefix
                .is_some_and(|prefix| !normalize_name(&name).starts_with(prefix))
            {
                continue;
            }
            let hit = LiveSearchHit {
                file_id: record.file_id,
                parent_id: record.parent_id,
                size_bytes: record.size_bytes,
                flags: record.flags,
                name,
            };
            if normalized_root_prefix
                .is_some_and(|root| !self.candidate_inside_root(&hit, root, &mut verified_parents))
            {
                continue;
            }
            hits.push(hit);
            if hits.len() >= budget {
                break;
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
                flags: delta.flags,
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
            flags: record.flags,
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
        let mut last_is_directory = hit.flags & FLAG_DIRECTORY != 0;
        let mut visited = HashSet::from([hit.file_id]);
        for _ in 0..max_depth.saturating_sub(1) {
            if parent_id == 0 {
                break;
            }
            if parent_id == last_id {
                // A self-parent terminal record denotes a volume root,
                // never an ordinary file pointing to itself.
                if !last_is_directory {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "indexed file cannot be its own parent",
                    ));
                }
                break;
            }
            if !visited.insert(parent_id) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "indexed path contains a cyclic parent chain",
                ));
            }
            let Some(node) = self.path_node(parent_id)? else {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("indexed path parent {parent_id} is missing"),
                ));
            };
            if node.flags & FLAG_DIRECTORY == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "indexed parent record is not a directory",
                ));
            }
            if !node.name.is_empty() {
                pieces.push(node.name);
            }
            last_id = parent_id;
            parent_id = node.parent_id;
            last_is_directory = node.flags & FLAG_DIRECTORY != 0;
        }
        // Truncated parent chains can produce plausible but false paths.
        // The requested depth is sufficient only after a genuine root or
        // self-parent root record has been reached.
        if parent_id != 0 && parent_id == last_id && !last_is_directory {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "indexed file cannot be its own parent",
            ));
        }
        if parent_id != 0 && parent_id != last_id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "indexed path exceeds verified parent depth",
            ));
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
    use crate::index_lock::IndexPublishGuard;
    use crate::store::{BuildOptions, IndexBuilder, InputRecord, FLAG_DIRECTORY};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    fn prefix_search_should_include_recent_delta_entries() {
        let path = temp("prefix-delta-new-entry");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for offset in 0..8u64 {
            let name = format!("report-a{offset:02}.txt");
            builder
                .push(InputRecord {
                    file_id: 10 + offset,
                    parent_id: 0,
                    size_bytes: 1,
                    flags: 0,
                    name: &name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        let dpath = delta_path(&path);
        let mut delta = DeltaWriter::open(&dpath).unwrap();
        delta
            .append(&DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 500,
                parent_id: 0,
                size_bytes: 1,
                flags: 0,
                name: "report-000-new.txt".into(),
            })
            .unwrap();
        delta
            .append(&DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 11,
                parent_id: 0,
                size_bytes: 1,
                flags: 0,
                name: "notes-renamed.txt".into(),
            })
            .unwrap();
        delta.sync().unwrap();
        drop(delta);
        let mut store = LiveSearchStore::open(&path).unwrap();
        let names: Vec<_> = store
            .search_prefix("report", 3)
            .unwrap()
            .into_iter()
            .map(|h| h.name)
            .collect();
        assert_eq!(
            names,
            ["report-000-new.txt", "report-a00.txt", "report-a02.txt"]
        );
        assert!(
            !store
                .search_prefix("report", 12)
                .unwrap()
                .iter()
                .any(|hit| hit.name == "report-a01.txt"),
            "renamed base file must not reappear under stale filename"
        );
        for file in [
            path.clone(),
            crate::store::names_path(&path),
            crate::store::ids_path(&path),
            dpath,
        ] {
            let _ = fs::remove_file(file);
        }
    }

    #[test]
    fn scoped_filtered_search_recovers_matches_beyond_global_ranked_candidates() {
        use crate::filters::parse_search_query;

        let path = temp("scoped-ranked-starvation");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (file_id, parent_id, name, flags) in [
            (1, 1, "C:", FLAG_DIRECTORY),
            (2, 1, "Outside", FLAG_DIRECTORY),
            (3, 1, "Private", FLAG_DIRECTORY),
        ] {
            builder
                .push(InputRecord {
                    file_id,
                    parent_id,
                    size_bytes: 0,
                    flags,
                    name,
                })
                .unwrap();
        }
        // 350 globally earlier lexical matches consume the ranked candidate
        // window while the sole in-scope report remains within scan budget.
        for offset in 0..350u64 {
            let name = format!("report-a{offset:04}.txt");
            builder
                .push(InputRecord {
                    file_id: 10 + offset,
                    parent_id: 2,
                    size_bytes: 1,
                    flags: 0,
                    name: &name,
                })
                .unwrap();
        }
        builder
            .push(InputRecord {
                file_id: 500,
                parent_id: 3,
                size_bytes: 1,
                flags: 0,
                name: "report-z-private.txt",
            })
            .unwrap();
        builder.finish().unwrap();

        let mut store = LiveSearchStore::open(&path).unwrap();
        let mut parsed = parse_search_query("report");
        parsed.filters.path_contains = Some("private\\".into());
        // The fallback may not walk outside a caller-specified small budget.
        // The in-scope hit is deliberately located beyond the first 128
        // indexed records, as well as beyond the global name candidate cap.
        assert!(
            store
                .search_filtered_with_root_prefix(&parsed, None, 1, 128, Some("C:\\Private"))
                .unwrap()
                .is_empty(),
            "scoped fallback silently ignored the scan budget"
        );
        let hits = store
            .search_filtered_with_root_prefix(&parsed, None, 1, 512, Some("C:\\Private"))
            .unwrap();
        assert_eq!(hits.len(), 1, "in-scope report lost behind outside top-N");
        assert_eq!(hits[0].name, "report-z-private.txt");
        assert_eq!(
            store.reconstruct_path(&hits[0], 32).unwrap(),
            "C:\\Private\\report-z-private.txt"
        );

        for file in [
            path.clone(),
            crate::store::names_path(&path),
            crate::store::ids_path(&path),
        ] {
            let _ = fs::remove_file(file);
        }
    }

    #[test]
    fn scoped_search_does_not_starve_new_or_moved_delta_files_after_large_base() {
        use crate::filters::parse_search_query;

        let path = temp("scoped-delta-after-large-base");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (file_id, parent_id, name, flags) in [
            (1, 1, "C:", FLAG_DIRECTORY),
            (2, 1, "Outside", FLAG_DIRECTORY),
            (3, 1, "Private", FLAG_DIRECTORY),
        ] {
            builder
                .push(InputRecord {
                    file_id,
                    parent_id,
                    size_bytes: 0,
                    flags,
                    name,
                })
                .unwrap();
        }
        for offset in 0..350u64 {
            let name = format!("report-a{offset:04}.txt");
            builder
                .push(InputRecord {
                    file_id: 10 + offset,
                    parent_id: 2,
                    size_bytes: 1,
                    flags: 0,
                    name: &name,
                })
                .unwrap();
        }
        builder.finish().unwrap();

        let dpath = delta_path(&path);
        let mut delta = DeltaWriter::open(&dpath).unwrap();
        for (id, name) in [(500, "report-z-new.txt"), (10, "report-z-moved.txt")] {
            delta
                .append(&DeltaRecord {
                    op: DeltaOp::Upsert,
                    file_id: id,
                    parent_id: 3,
                    size_bytes: 1,
                    flags: 0,
                    name: name.to_string(),
                })
                .unwrap();
        }
        delta.sync().unwrap();
        drop(delta);

        let mut store = LiveSearchStore::open(&path).unwrap();
        let mut parsed = parse_search_query("report");
        parsed.filters.path_contains = Some("private\\".into());
        let hits = store
            .search_filtered_with_root_prefix(&parsed, None, 2, 128, Some("C:\\Private"))
            .unwrap();
        let mut names: Vec<_> = hits.iter().map(|hit| hit.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            ["report-z-moved.txt", "report-z-new.txt"],
            "large base index starved fresh delta entries inside scoped folder"
        );
        for hit in &hits {
            assert!(store
                .reconstruct_path(hit, 32)
                .unwrap()
                .starts_with("C:\\Private\\"));
        }

        for file in [
            path.clone(),
            crate::store::names_path(&path),
            crate::store::ids_path(&path),
            dpath,
        ] {
            let _ = fs::remove_file(file);
        }
    }

    #[test]
    fn bounded_delta_candidates_have_stable_file_id_order() {
        let path = temp("deterministic-delta-order");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        builder
            .push(InputRecord {
                file_id: 1,
                parent_id: 1,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "C:",
            })
            .unwrap();
        builder.finish().unwrap();
        let dpath = delta_path(&path);
        let mut delta = DeltaWriter::open(&dpath).unwrap();
        for file_id in [82_u64, 13, 57, 44, 99, 26, 12, 31, 64, 7, 48, 18] {
            delta
                .append(&DeltaRecord {
                    op: DeltaOp::Upsert,
                    file_id,
                    parent_id: 1,
                    size_bytes: 1,
                    flags: 0,
                    name: format!("file-{file_id}.txt"),
                })
                .unwrap();
        }
        delta.sync().unwrap();
        drop(delta);
        let mut store = LiveSearchStore::open(&path).unwrap();
        let ids: Vec<_> = store
            .scan_live_candidates(5, None, None)
            .unwrap()
            .iter()
            .map(|hit| hit.file_id)
            .collect();
        assert_eq!(
            ids,
            [7, 12, 13, 18, 26],
            "bounded delta selection must not depend on HashMap iteration order"
        );
        for file in [
            path.clone(),
            crate::store::names_path(&path),
            crate::store::ids_path(&path),
            dpath,
        ] {
            let _ = fs::remove_file(file);
        }
    }

    #[test]
    fn scoped_name_search_ignores_unrelated_delta_slots_before_result_cap() {
        use crate::filters::parse_search_query;

        let path = temp("scoped-named-delta-budget");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (file_id, parent_id, name, flags) in [
            (1, 1, "C:", FLAG_DIRECTORY),
            (2, 1, "Outside", FLAG_DIRECTORY),
            (3, 1, "Private", FLAG_DIRECTORY),
        ] {
            builder
                .push(InputRecord {
                    file_id,
                    parent_id,
                    size_bytes: 0,
                    flags,
                    name,
                })
                .unwrap();
        }
        for offset in 0..350u64 {
            let name = format!("report-a{offset:04}.txt");
            builder
                .push(InputRecord {
                    file_id: 10 + offset,
                    parent_id: 2,
                    size_bytes: 1,
                    flags: 0,
                    name: &name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        let dpath = delta_path(&path);
        let mut delta = DeltaWriter::open(&dpath).unwrap();
        for offset in 0..300u64 {
            delta
                .append(&DeltaRecord {
                    op: DeltaOp::Upsert,
                    file_id: 1000 + offset,
                    parent_id: 2,
                    size_bytes: 1,
                    flags: 0,
                    name: format!("unrelated-{offset:04}.txt"),
                })
                .unwrap();
        }
        delta
            .append(&DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 9000,
                parent_id: 3,
                size_bytes: 1,
                flags: 0,
                name: "report-z-private.txt".into(),
            })
            .unwrap();
        delta.sync().unwrap();
        drop(delta);

        let mut store = LiveSearchStore::open(&path).unwrap();
        let mut parsed = parse_search_query("report");
        parsed.filters.path_contains = Some("private\\".into());
        let hits = store
            .search_filtered_with_root_prefix(&parsed, None, 1, 128, Some("C:\\Private"))
            .unwrap();
        assert_eq!(
            hits.len(),
            1,
            "unrelated delta upserts exhausted bounded named scan"
        );
        assert_eq!(hits[0].name, "report-z-private.txt");
        assert_eq!(
            store.reconstruct_path(&hits[0], 32).unwrap(),
            "C:\\Private\\report-z-private.txt"
        );
        for file in [
            path.clone(),
            crate::store::names_path(&path),
            crate::store::ids_path(&path),
            dpath,
        ] {
            let _ = fs::remove_file(file);
        }
    }

    #[test]
    fn scoped_name_search_does_not_lose_private_delta_after_matching_outside_upserts() {
        use crate::filters::parse_search_query;

        let path = temp("matching-delta-scoped-overflow");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (file_id, parent_id, name, flags) in [
            (1, 1, "C:", FLAG_DIRECTORY),
            (2, 1, "Outside", FLAG_DIRECTORY),
            (3, 1, "Private", FLAG_DIRECTORY),
            (4, 1, "Private-old", FLAG_DIRECTORY),
        ] {
            builder
                .push(InputRecord {
                    file_id,
                    parent_id,
                    size_bytes: 0,
                    flags,
                    name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        let dpath = delta_path(&path);
        let mut delta = DeltaWriter::open(&dpath).unwrap();
        for offset in 0..250u64 {
            delta
                .append(&DeltaRecord {
                    op: DeltaOp::Upsert,
                    file_id: 1000 + offset,
                    parent_id: 2,
                    size_bytes: 1,
                    flags: 0,
                    name: format!("report-a{offset:04}.txt"),
                })
                .unwrap();
        }
        for (file_id, parent_id, name) in [
            (8000, 4, "report-z-sibling.txt"),
            (8001, 777_777, "report-z-broken-parent.txt"),
        ] {
            delta
                .append(&DeltaRecord {
                    op: DeltaOp::Upsert,
                    file_id,
                    parent_id,
                    size_bytes: 1,
                    flags: 0,
                    name: name.into(),
                })
                .unwrap();
        }
        delta
            .append(&DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 9000,
                parent_id: 3,
                size_bytes: 1,
                flags: 0,
                name: "report-z-private.txt".into(),
            })
            .unwrap();
        delta.sync().unwrap();
        drop(delta);

        let mut store = LiveSearchStore::open(&path).unwrap();
        let parsed = parse_search_query("report");
        let hits = store
            .search_filtered_with_root_prefix(&parsed, None, 1, 128, Some("C:\\Private"))
            .unwrap();
        assert_eq!(
            hits.len(),
            1,
            "out-of-scope matching delta entries exhausted the candidate budget"
        );
        assert_eq!(hits[0].name, "report-z-private.txt");
        assert_eq!(
            store.reconstruct_path(&hits[0], 32).unwrap(),
            "C:\\Private\\report-z-private.txt"
        );
        for file in [
            path.clone(),
            crate::store::names_path(&path),
            crate::store::ids_path(&path),
            dpath,
        ] {
            let _ = fs::remove_file(file);
        }
    }

    #[test]
    fn cached_parent_scope_matches_canonical_full_path_checks() {
        let path = temp("cached-parent-scope-equivalence");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (file_id, parent_id, name, flags) in [
            (1, 1, "C:", FLAG_DIRECTORY),
            (2, 1, "Private", FLAG_DIRECTORY),
            (3, 1, "Private-old", FLAG_DIRECTORY),
            (4, 2, "Nested", FLAG_DIRECTORY),
            (5, 1, "not-a-directory.txt", 0),
        ] {
            builder
                .push(InputRecord {
                    file_id,
                    parent_id,
                    size_bytes: 0,
                    flags,
                    name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        let dpath = delta_path(&path);
        let mut delta = DeltaWriter::open(&dpath).unwrap();
        for (file_id, parent_id, flags, name) in [
            (100, 2, 0, "report-a.txt"),
            (101, 2, 0, "report-b.txt"),
            (102, 3, 0, "report-sibling.txt"),
            (103, 4, 0, "report-nested.txt"),
            (104, 99_999, 0, "report-broken-parent.txt"),
            (105, 5, 0, "report-file-parent.txt"),
            (106, 107, FLAG_DIRECTORY, "cyclic-a"),
            (107, 106, FLAG_DIRECTORY, "cyclic-b"),
        ] {
            delta
                .append(&DeltaRecord {
                    op: DeltaOp::Upsert,
                    file_id,
                    parent_id,
                    size_bytes: 1,
                    flags,
                    name: name.into(),
                })
                .unwrap();
        }
        delta.sync().unwrap();
        drop(delta);
        let mut store = LiveSearchStore::open(&path).unwrap();
        for root in ["C:\\Private", "C:\\Private-old", "C:"] {
            let normalized_root = normalize_name(root);
            let mut cache = HashMap::new();
            for file_id in [1, 2, 3, 4, 5, 100, 101, 102, 103, 104, 105, 106, 107] {
                let hit = store.get_by_file_id(file_id).unwrap().unwrap();
                let canonical = store
                    .reconstruct_path(&hit, 256)
                    .is_ok_and(|path| LiveSearchStore::path_within_root(&path, &normalized_root));
                let cached = store.candidate_inside_root(&hit, &normalized_root, &mut cache);
                assert_eq!(
                    cached, canonical,
                    "cached parent path changed scope decision for root {root}, id {file_id}"
                );
            }
        }
        for file in [
            path.clone(),
            crate::store::names_path(&path),
            crate::store::ids_path(&path),
            dpath,
        ] {
            let _ = fs::remove_file(file);
        }
    }

    // Manual scale benchmark, excluded from routine tests/CI. Uses only an
    // isolated synthetic on-disk index. No production Windows Search state.
    // Run: cargo test -p search-core --release bench_scoped_large_delta -- --ignored --nocapture
    #[test]
    #[ignore = "manual synthetic large-delta latency benchmark"]
    fn bench_scoped_large_delta() {
        use crate::filters::parse_search_query;

        for delta_count in [1_024_u64, 8_192_u64, 16_384_u64] {
            let path = temp(&format!("bench-scoped-delta-{delta_count}"));
            let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
            for (file_id, parent_id, name) in [(1, 1, "C:"), (2, 1, "Outside"), (3, 1, "Private")] {
                builder
                    .push(InputRecord {
                        file_id,
                        parent_id,
                        size_bytes: 0,
                        flags: FLAG_DIRECTORY,
                        name,
                    })
                    .unwrap();
            }
            builder.finish().unwrap();
            let dpath = delta_path(&path);
            let mut delta = DeltaWriter::open(&dpath).unwrap();
            for offset in 0..delta_count {
                delta
                    .append(&DeltaRecord {
                        op: DeltaOp::Upsert,
                        file_id: 1_000 + offset,
                        parent_id: 2,
                        size_bytes: 1,
                        flags: 0,
                        name: format!("report-a{offset:05}.txt"),
                    })
                    .unwrap();
            }
            delta
                .append(&DeltaRecord {
                    op: DeltaOp::Upsert,
                    file_id: 100_000,
                    parent_id: 3,
                    size_bytes: 1,
                    flags: 0,
                    name: "report-z-private.txt".into(),
                })
                .unwrap();
            delta.sync().unwrap();
            drop(delta);

            let mut store = LiveSearchStore::open(&path).unwrap();
            let parsed = parse_search_query("report");
            let mut latencies = Vec::new();
            for iteration in 0..9 {
                let start = Instant::now();
                let hits = store
                    .search_filtered_with_root_prefix(&parsed, None, 1, 128, Some("C:\\Private"))
                    .unwrap();
                let elapsed = start.elapsed().as_micros();
                assert_eq!(hits.len(), 1);
                assert_eq!(hits[0].name, "report-z-private.txt");
                if iteration > 0 {
                    latencies.push(elapsed);
                }
            }
            latencies.sort_unstable();
            println!(
                "SYNTHETIC_SCOPED_DELTA size={} median_us={} p95_us={} samples={} hits=1",
                delta_count,
                latencies[latencies.len() / 2],
                latencies[latencies.len() - 1],
                latencies.len(),
            );
            for file in [
                path.clone(),
                crate::store::names_path(&path),
                crate::store::ids_path(&path),
                dpath,
            ] {
                let _ = fs::remove_file(file);
            }
        }
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
    fn initial_open_waits_for_compaction_publish_snapshot() {
        let path = temp("publish-snapshot");
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
                size_bytes: 1,
                flags: 0,
                name: "stable.txt",
            })
            .unwrap();
        builder.finish().unwrap();

        let original = fs::read(&path).unwrap();
        let publisher = IndexPublishGuard::write(&path).unwrap();
        fs::write(&path, &original[..8]).unwrap();

        let open_path = path.clone();
        let opener = std::thread::spawn(move || LiveSearchStore::open(open_path).unwrap());
        std::thread::sleep(Duration::from_millis(25));
        assert!(
            !opener.is_finished(),
            "fresh reader must wait while a family publication is in progress"
        );

        fs::write(&path, &original).unwrap();
        drop(publisher);

        let mut live = opener.join().unwrap();
        assert_eq!(live.search_exact("stable.txt", 4).unwrap().len(), 1);

        for suffix in ["", ".names", ".ids", ".ncp", ".icp", ".publish.lock"] {
            let mut value = path.as_os_str().to_os_string();
            value.push(suffix);
            let _ = fs::remove_file(PathBuf::from(value));
        }
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

        // refresh_now() is intentionally non-blocking with respect to writers and may
        // transiently return WouldBlock while the mutation lock handoff settles.
        // Compaction has completed synchronously here, so retry for a bounded interval
        // instead of assuming a zero-delay lock reacquire.
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        let outcome = loop {
            match live.refresh_now() {
                Ok(outcome) => break outcome,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("refresh after completed compaction failed: {error}"),
            }
        };
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
