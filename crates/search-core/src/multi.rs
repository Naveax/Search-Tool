use crate::attributes::{attribute_index_path, AttributeIndex};
use crate::content::{content_path, ContentIndex};
use crate::filters::ParsedSearchQuery;
use crate::freshness::sidecar_is_fresh;
use crate::live::{LiveSearchHit, LiveSearchStore};
use crate::query::relevance_score;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeSearchHit {
    pub volume: char,
    pub hit: LiveSearchHit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SidecarStamp {
    len: u64,
    modified: Option<SystemTime>,
}

#[derive(Debug)]
struct VolumeStore {
    volume: char,
    index_path: PathBuf,
    store: LiveSearchStore,
    attributes: Option<AttributeIndex>,
    attribute_stamp: Option<SidecarStamp>,
}

impl VolumeStore {
    fn refresh_attributes(&mut self) -> io::Result<()> {
        let path = attribute_index_path(&self.index_path);
        if !sidecar_is_fresh(&self.index_path, &path).unwrap_or(false) {
            self.attributes = None;
            self.attribute_stamp = None;
            return Ok(());
        }
        let stamp = sidecar_stamp(&path)?;
        if self.attributes.is_none() || stamp != self.attribute_stamp {
            self.attributes = Some(AttributeIndex::open(&path)?);
            self.attribute_stamp = stamp;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MultiLiveSearchStore {
    volumes: Vec<VolumeStore>,
}

impl MultiLiveSearchStore {
    pub fn open_index(index_path: impl AsRef<Path>) -> io::Result<Self> {
        let index_path = index_path.as_ref();
        let volume = volume_from_index_path(index_path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "index filename must be a single drive letter such as C.stidx",
            )
        })?;
        Self::open_volume(volume, index_path)
    }

    pub fn open_volume(volume: char, index_path: impl AsRef<Path>) -> io::Result<Self> {
        let volume = normalize_volume(volume)?;
        let index_path = index_path.as_ref().to_path_buf();
        let store = LiveSearchStore::open(&index_path)?;
        let attribute_path = attribute_index_path(&index_path);
        let attributes = if sidecar_is_fresh(&index_path, &attribute_path).unwrap_or(false) {
            AttributeIndex::open(&attribute_path).ok()
        } else {
            None
        };
        let attribute_stamp = if attributes.is_some() {
            sidecar_stamp(&attribute_path)?
        } else {
            None
        };
        Ok(Self {
            volumes: vec![VolumeStore {
                volume,
                index_path,
                store,
                attributes,
                attribute_stamp,
            }],
        })
    }

    pub fn open_index_directory(index_dir: impl AsRef<Path>) -> io::Result<Self> {
        let paths = discover_volume_indexes(index_dir)?;
        if paths.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no volume indexes were found",
            ));
        }
        let mut volumes = Vec::with_capacity(paths.len());
        for (volume, index_path) in paths {
            let store = LiveSearchStore::open(&index_path)?;
            let attribute_path = attribute_index_path(&index_path);
            let attributes = if sidecar_is_fresh(&index_path, &attribute_path).unwrap_or(false) {
                AttributeIndex::open(&attribute_path).ok()
            } else {
                None
            };
            let attribute_stamp = if attributes.is_some() {
                sidecar_stamp(&attribute_path)?
            } else {
                None
            };
            volumes.push(VolumeStore {
                volume,
                index_path,
                store,
                attributes,
                attribute_stamp,
            });
        }
        Ok(Self { volumes })
    }

    pub fn volume_count(&self) -> usize {
        self.volumes.len()
    }

    pub fn volumes(&self) -> impl Iterator<Item = char> + '_ {
        self.volumes.iter().map(|volume| volume.volume)
    }

    pub fn memory_hint_bytes(&self) -> usize {
        self.volumes
            .iter()
            .map(|volume| volume.store.memory_hint_bytes())
            .sum()
    }

    pub fn search_prefix(&mut self, query: &str, limit: usize) -> io::Result<Vec<VolumeSearchHit>> {
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let per_volume = per_volume_limit(limit, self.volumes.len());
        let mut hits = Vec::with_capacity(limit.saturating_mul(2).min(512));
        for volume in &mut self.volumes {
            for hit in volume.store.search_prefix(query, per_volume)? {
                hits.push(VolumeSearchHit {
                    volume: volume.volume,
                    hit,
                });
            }
        }
        sort_ranked(query, &mut hits);
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn search_ranked(&mut self, query: &str, limit: usize) -> io::Result<Vec<VolumeSearchHit>> {
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let per_volume = per_volume_limit(limit, self.volumes.len());
        let mut hits = Vec::with_capacity(limit.saturating_mul(2).min(512));
        for volume in &mut self.volumes {
            for hit in volume.store.search_ranked(query, per_volume)? {
                hits.push(VolumeSearchHit {
                    volume: volume.volume,
                    hit,
                });
            }
        }
        sort_ranked(query, &mut hits);
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn search_fuzzy(
        &mut self,
        query: &str,
        max_distance: usize,
        limit: usize,
    ) -> io::Result<Vec<VolumeSearchHit>> {
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let per_volume = per_volume_limit(limit, self.volumes.len());
        let mut hits = Vec::with_capacity(limit.saturating_mul(2).min(512));
        for volume in &mut self.volumes {
            for hit in volume.store.search_fuzzy(query, max_distance, per_volume)? {
                hits.push(VolumeSearchHit {
                    volume: volume.volume,
                    hit,
                });
            }
        }
        hits.sort_unstable_by(|a, b| {
            crate::query::fuzzy_distance(query, &a.hit.name, max_distance)
                .unwrap_or(usize::MAX)
                .cmp(
                    &crate::query::fuzzy_distance(query, &b.hit.name, max_distance)
                        .unwrap_or(usize::MAX),
                )
                .then_with(|| {
                    relevance_score(query, &b.hit.name, b.hit.flags).cmp(&relevance_score(
                        query,
                        &a.hit.name,
                        a.hit.flags,
                    ))
                })
                .then(a.volume.cmp(&b.volume))
                .then(a.hit.file_id.cmp(&b.hit.file_id))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn search_related(
        &mut self,
        query: &str,
        limit: usize,
    ) -> io::Result<Vec<VolumeSearchHit>> {
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let per_volume = per_volume_limit(limit, self.volumes.len());
        let mut hits = Vec::with_capacity(limit.saturating_mul(2).min(512));
        for volume in &mut self.volumes {
            for hit in volume.store.search_related(query, per_volume)? {
                hits.push(VolumeSearchHit {
                    volume: volume.volume,
                    hit,
                });
            }
        }
        sort_ranked(query, &mut hits);
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn search_filtered(
        &mut self,
        parsed: &ParsedSearchQuery,
        limit: usize,
        scan_budget_per_volume: usize,
    ) -> io::Result<Vec<VolumeSearchHit>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let per_volume = per_volume_limit(limit, self.volumes.len());
        let mut hits = Vec::with_capacity(limit.saturating_mul(2).min(512));
        for volume in &mut self.volumes {
            // LiveSearchStore reconstructs paths relative to its volume,
            // while callers of MultiLiveSearchStore filter C:\\... paths.
            // Strip a matching drive prefix before local filtering and skip
            // other drives rather than dropping every drive-scoped hit.
            let mut local_parsed = parsed.clone();
            let mut volume_limit = per_volume;
            let mut root_prefix = None;
            if let Some(needle) = &parsed.filters.path_contains {
                let Some((local_needle, drive_qualified)) =
                    local_path_filter(needle, volume.volume)
                else {
                    continue;
                };
                if drive_qualified {
                    // An absolute C:\\ needle must match from the volume
                    // root, not a similarly named file deep in a folder.
                    root_prefix = Some(local_needle.clone());
                    // Only one volume is eligible; do not throttle its hits
                    // based on unrelated volume count.
                    volume_limit = limit;
                }
                local_parsed.filters.path_contains = Some(local_needle);
            }
            let _ = volume.store.refresh_if_due()?;
            volume.refresh_attributes()?;
            let volume_hits = volume.store.search_filtered_with_root_prefix(
                &local_parsed,
                volume.attributes.as_mut(),
                volume_limit,
                scan_budget_per_volume,
                root_prefix.as_deref(),
            )?;
            for hit in volume_hits {
                hits.push(VolumeSearchHit {
                    volume: volume.volume,
                    hit,
                });
            }
        }
        sort_ranked(&parsed.text, &mut hits);
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn search_content(
        &mut self,
        query: &str,
        limit: usize,
    ) -> io::Result<Vec<VolumeSearchHit>> {
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let per_volume = per_volume_limit(limit, self.volumes.len());
        let mut hits = Vec::with_capacity(limit.saturating_mul(2).min(512));
        for volume in &mut self.volumes {
            let content_path = content_path(&volume.index_path);
            let mut content = match ContentIndex::open(content_path) {
                Ok(index) => index,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            for file_id in content.search(query, per_volume)? {
                if let Some(hit) = volume.store.get_by_file_id(file_id)? {
                    hits.push(VolumeSearchHit {
                        volume: volume.volume,
                        hit,
                    });
                }
            }
        }
        sort_ranked(query, &mut hits);
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn stale_attribute_volumes(&self) -> Vec<char> {
        self.volumes
            .iter()
            .filter(|volume| {
                let path = attribute_index_path(&volume.index_path);
                !sidecar_is_fresh(&volume.index_path, &path).unwrap_or(false)
            })
            .map(|volume| volume.volume)
            .collect()
    }

    pub fn stale_content_volumes(&self) -> Vec<char> {
        self.volumes
            .iter()
            .filter(|volume| {
                let path = content_path(&volume.index_path);
                !sidecar_is_fresh(&volume.index_path, &path).unwrap_or(false)
            })
            .map(|volume| volume.volume)
            .collect()
    }

    pub fn reconstruct_path(
        &mut self,
        hit: &VolumeSearchHit,
        max_depth: usize,
    ) -> io::Result<String> {
        let volume = self
            .volumes
            .iter_mut()
            .find(|volume| volume.volume == hit.volume)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "volume index is not open"))?;
        let path = volume.store.reconstruct_path(&hit.hit, max_depth)?;
        Ok(qualify_volume_path(hit.volume, &path))
    }

    pub fn index_paths(&self) -> impl Iterator<Item = (char, &Path)> {
        self.volumes
            .iter()
            .map(|volume| (volume.volume, volume.index_path.as_path()))
    }
}

fn sidecar_stamp(path: &Path) -> io::Result<Option<SidecarStamp>> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(SidecarStamp {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        })),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn volume_from_index_path(path: impl AsRef<Path>) -> Option<char> {
    let stem = path.as_ref().file_stem()?.to_str()?;
    if stem.len() != 1 {
        return None;
    }
    let volume = stem.chars().next()?.to_ascii_uppercase();
    volume.is_ascii_alphabetic().then_some(volume)
}

pub fn discover_volume_indexes(index_dir: impl AsRef<Path>) -> io::Result<Vec<(char, PathBuf)>> {
    let mut indexes = Vec::new();
    for entry in fs::read_dir(index_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file()
            || path.extension().and_then(|ext| ext.to_str()) != Some("stidx")
        {
            continue;
        }
        if let Some(volume) = volume_from_index_path(&path) {
            indexes.push((volume, path));
        }
    }
    indexes.sort_unstable_by_key(|(volume, _)| *volume);
    indexes.dedup_by_key(|(volume, _)| *volume);
    Ok(indexes)
}

fn normalize_volume(volume: char) -> io::Result<char> {
    let volume = volume.to_ascii_uppercase();
    if volume.is_ascii_alphabetic() {
        Ok(volume)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "volume must be an ASCII drive letter",
        ))
    }
}

fn per_volume_limit(limit: usize, volume_count: usize) -> usize {
    if volume_count <= 1 {
        return limit;
    }
    limit
        .saturating_mul(2)
        .div_ceil(volume_count)
        .clamp(8, limit.max(8))
}

fn sort_ranked(query: &str, hits: &mut [VolumeSearchHit]) {
    hits.sort_unstable_by(|a, b| {
        relevance_score(query, &b.hit.name, b.hit.flags)
            .cmp(&relevance_score(query, &a.hit.name, a.hit.flags))
            .then(a.hit.name.len().cmp(&b.hit.name.len()))
            .then(a.volume.cmp(&b.volume))
            .then(a.hit.file_id.cmp(&b.hit.file_id))
    });
}

// Match fully qualified paths (C:\\ or C:/) against the relative paths
// stored in each volume's native index. None means the filter explicitly
// requests a different drive. Normalize separators in generic fragments too:
// a user may type docs/report.txt when the index stores docs\\report.txt.
fn local_path_filter(needle: &str, volume: char) -> Option<(String, bool)> {
    let bytes = needle.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
    {
        if bytes[0].to_ascii_uppercase() != volume.to_ascii_uppercase() as u8 {
            return None;
        }
        return Some((needle[3..].replace('/', "\\"), true));
    }
    Some((needle.replace('/', "\\"), false))
}

fn qualify_volume_path(volume: char, path: &str) -> String {
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return path.to_string();
    }
    if path.starts_with('\\') {
        format!("{volume}:{path}")
    } else if path.is_empty() {
        format!("{volume}:\\")
    } else {
        format!("{volume}:\\{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{AttributeEntry, AttributeIndexBuilder};
    use crate::store::{BuildOptions, IndexBuilder, InputRecord, FLAG_DIRECTORY};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("search-tool-multi-{name}-{n}"))
    }

    fn build_volume(dir: &Path, volume: char, files: &[(u64, &str)]) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(format!("{volume}.stidx"));
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        builder
            .push(InputRecord {
                file_id: 5,
                parent_id: 5,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "",
            })
            .unwrap();
        for (file_id, name) in files {
            builder
                .push(InputRecord {
                    file_id: *file_id,
                    parent_id: 5,
                    size_bytes: 0,
                    flags: 0,
                    name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        path
    }

    #[test]
    fn discovers_and_searches_multiple_volume_indexes() {
        let dir = temp_dir("search");
        build_volume(&dir, 'C', &[(10, "node.exe"), (11, "notepad.exe")]);
        build_volume(&dir, 'D', &[(10, "node-project.txt"), (12, "notes.txt")]);

        let mut store = MultiLiveSearchStore::open_index_directory(&dir).unwrap();
        assert_eq!(store.volume_count(), 2);
        assert_eq!(store.volumes().collect::<Vec<_>>(), vec!['C', 'D']);
        let hits = store.search_ranked("node", 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits
            .iter()
            .any(|hit| hit.volume == 'C' && hit.hit.name == "node.exe"));
        assert!(hits
            .iter()
            .any(|hit| hit.volume == 'D' && hit.hit.name == "node-project.txt"));
        let d_hit = hits.iter().find(|hit| hit.volume == 'D').unwrap().clone();
        assert_eq!(
            store.reconstruct_path(&d_hit, 32).unwrap(),
            r"D:\node-project.txt"
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn filtered_search_matches_drive_qualified_paths_on_correct_volume() {
        let dir = temp_dir("qualified-filter");
        build_volume(&dir, 'C', &[(10, "report.txt"), (11, "notes.txt")]);
        build_volume(&dir, 'D', &[(10, "report.txt")]);
        let mut store = MultiLiveSearchStore::open_index_directory(&dir).unwrap();
        for query in ["report path:C:/report.txt", r"report path:C:\report.txt"] {
            let parsed = crate::filters::parse_search_query(query);
            let hits = store.search_filtered(&parsed, 10, 4096).unwrap();
            assert_eq!(hits.len(), 1, "{query}");
            assert_eq!(hits[0].volume, 'C');
            assert_eq!(
                store.reconstruct_path(&hits[0], 32).unwrap(),
                r"C:\report.txt"
            );
        }
        let parsed = crate::filters::parse_search_query("report path:D:/report.txt");
        let hits = store.search_filtered(&parsed, 10, 4096).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].volume, 'D');
        let parsed = crate::filters::parse_search_query("report path:E:/report.txt");
        assert!(store.search_filtered(&parsed, 10, 4096).unwrap().is_empty());
        // Relative substrings continue to match on all volumes.
        let parsed = crate::filters::parse_search_query("report path:report");
        assert_eq!(store.search_filtered(&parsed, 10, 4096).unwrap().len(), 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn absolute_path_filter_does_not_match_same_filename_in_sibling_directory() {
        let dir = temp_dir("absolute-rooted-filter");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("C.stidx");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for record in [
            InputRecord {
                file_id: 5,
                parent_id: 5,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "",
            },
            InputRecord {
                file_id: 10,
                parent_id: 5,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "Other",
            },
            InputRecord {
                file_id: 12,
                parent_id: 5,
                size_bytes: 0,
                flags: 0,
                name: "report.txt",
            },
            InputRecord {
                file_id: 11,
                parent_id: 10,
                size_bytes: 0,
                flags: 0,
                name: "report.txt",
            },
            InputRecord {
                file_id: 13,
                parent_id: 5,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "Other-old",
            },
            InputRecord {
                file_id: 14,
                parent_id: 13,
                size_bytes: 0,
                flags: 0,
                name: "report.txt",
            },
            InputRecord {
                file_id: 15,
                parent_id: 5,
                size_bytes: 0,
                flags: 0,
                name: "report.txt-old",
            },
        ] {
            builder.push(record).unwrap();
        }
        builder.finish().unwrap();
        let mut store = MultiLiveSearchStore::open_index_directory(&dir).unwrap();
        for (query, expected_path) in [
            (r"report path:C:\report.txt", r"C:\report.txt"),
            (r"report path:C:\Other\report.txt", r"C:\Other\report.txt"),
            (r"report path:C:\Other", r"C:\Other\report.txt"),
            (r"report path:C:\Other\", r"C:\Other\report.txt"),
            (r"report path:c:/other", r"C:\Other\report.txt"),
        ] {
            let parsed = crate::filters::parse_search_query(query);
            let hits = store.search_filtered(&parsed, 10, 4096).unwrap();
            assert_eq!(hits.len(), 1, "{query}");
            assert_eq!(store.reconstruct_path(&hits[0], 32).unwrap(), expected_path);
            // Even at a one-result limit, an unrelated same-name hit must
            // not crowd out the true rooted match before filtering.
            let limited = store.search_filtered(&parsed, 1, 4096).unwrap();
            assert_eq!(limited.len(), 1, "limited {query}");
            assert_eq!(
                store.reconstruct_path(&limited[0], 32).unwrap(),
                expected_path
            );
        }
        let root = crate::filters::parse_search_query(r"report path:C:\");
        assert_eq!(store.search_filtered(&root, 10, 4096).unwrap().len(), 4);
        let parsed = crate::filters::parse_search_query("report path:report.txt");
        assert_eq!(store.search_filtered(&parsed, 10, 4096).unwrap().len(), 4);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn orphaned_index_path_must_not_fabricate_a_root_level_result() {
        let dir = temp_dir("orphaned-parent");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("C.stidx");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for record in [
            InputRecord {
                file_id: 5,
                parent_id: 5,
                size_bytes: 0,
                flags: FLAG_DIRECTORY,
                name: "",
            },
            InputRecord {
                file_id: 17,
                parent_id: 999,
                size_bytes: 0,
                flags: 0,
                name: "report.txt",
            },
        ] {
            builder.push(record).unwrap();
        }
        builder.finish().unwrap();
        let mut store = MultiLiveSearchStore::open_index_directory(&dir).unwrap();
        let hits = store.search_ranked("report", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(store.reconstruct_path(&hits[0], 256).is_err());
        let parsed = crate::filters::parse_search_query(r"report path:C:\report.txt");
        assert!(store.search_filtered(&parsed, 10, 4096).unwrap().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn relative_path_filters_accept_forward_slashes_across_volumes() {
        let dir = temp_dir("relative-filter-slashes");
        fs::create_dir_all(&dir).unwrap();
        for volume in ['C', 'D'] {
            let path = dir.join(format!("{volume}.stidx"));
            let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
            for record in [
                InputRecord {
                    file_id: 5,
                    parent_id: 5,
                    size_bytes: 0,
                    flags: FLAG_DIRECTORY,
                    name: "",
                },
                InputRecord {
                    file_id: 10,
                    parent_id: 5,
                    size_bytes: 0,
                    flags: FLAG_DIRECTORY,
                    name: "docs",
                },
                InputRecord {
                    file_id: 11,
                    parent_id: 10,
                    size_bytes: 0,
                    flags: 0,
                    name: "report.txt",
                },
            ] {
                builder.push(record).unwrap();
            }
            builder.finish().unwrap();
        }
        let mut store = MultiLiveSearchStore::open_index_directory(&dir).unwrap();
        for query in [
            "report path:docs/report.txt",
            "report in:docs/report.txt",
            r"report path:docs\report.txt",
        ] {
            let parsed = crate::filters::parse_search_query(query);
            let hits = store.search_filtered(&parsed, 10, 4096).unwrap();
            assert_eq!(hits.len(), 2, "{query}");
            for volume in ['C', 'D'] {
                assert!(hits.iter().any(|hit| hit.volume == volume));
            }
        }
        let parsed = crate::filters::parse_search_query("report path:docs/missing.txt");
        assert!(store.search_filtered(&parsed, 10, 4096).unwrap().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn filtered_search_keeps_attribute_ids_scoped_to_volume() {
        let dir = temp_dir("filtered");
        let c = build_volume(&dir, 'C', &[(10, "node.exe")]);
        let d = build_volume(&dir, 'D', &[(10, "node.exe")]);

        let mut c_attrs = AttributeIndexBuilder::create(attribute_index_path(&c), 1024).unwrap();
        c_attrs
            .push(AttributeEntry {
                file_id: 10,
                size_bytes: 2 * 1024 * 1024,
                modified_unix_secs: 1_700_000_000,
                platform_attributes: 0,
            })
            .unwrap();
        c_attrs.finish().unwrap();
        crate::freshness::write_sidecar_generation(
            attribute_index_path(&c),
            crate::freshness::capture_index_generation(&c).unwrap(),
        )
        .unwrap();

        let mut d_attrs = AttributeIndexBuilder::create(attribute_index_path(&d), 1024).unwrap();
        d_attrs
            .push(AttributeEntry {
                file_id: 10,
                size_bytes: 512,
                modified_unix_secs: 1_700_000_000,
                platform_attributes: 0,
            })
            .unwrap();
        d_attrs.finish().unwrap();
        crate::freshness::write_sidecar_generation(
            attribute_index_path(&d),
            crate::freshness::capture_index_generation(&d).unwrap(),
        )
        .unwrap();

        let parsed = crate::filters::parse_search_query("node ext:exe size:>1mb");
        let mut store = MultiLiveSearchStore::open_index_directory(&dir).unwrap();
        let hits = store.search_filtered(&parsed, 10, 4096).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].volume, 'C');

        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn filtered_search_reloads_rebuilt_attribute_sidecar() {
        let dir = temp_dir("attribute-refresh");
        let c = build_volume(&dir, 'C', &[(10, "node.exe")]);
        let attrs_path = attribute_index_path(&c);

        let write_attrs = |size_bytes: u64| {
            let mut attrs = AttributeIndexBuilder::create(&attrs_path, 1024).unwrap();
            attrs
                .push(AttributeEntry {
                    file_id: 10,
                    size_bytes,
                    modified_unix_secs: 1_700_000_000,
                    platform_attributes: 0,
                })
                .unwrap();
            attrs.finish().unwrap();
            crate::freshness::write_sidecar_generation(
                &attrs_path,
                crate::freshness::capture_index_generation(&c).unwrap(),
            )
            .unwrap();
        };

        write_attrs(2 * 1024 * 1024);
        let parsed = crate::filters::parse_search_query("node ext:exe size:>1mb");
        let mut store = MultiLiveSearchStore::open_index_directory(&dir).unwrap();
        assert_eq!(store.search_filtered(&parsed, 10, 4096).unwrap().len(), 1);

        // Keep the same record count so the sidecar has the same byte length.
        // The live store must still notice the atomically replaced file.
        std::thread::sleep(std::time::Duration::from_millis(2));
        write_attrs(512);
        assert!(store.search_filtered(&parsed, 10, 4096).unwrap().is_empty());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn content_search_is_volume_scoped() {
        let dir = temp_dir("content");
        let c = build_volume(&dir, 'C', &[(10, "c-note.txt")]);
        let d = build_volume(&dir, 'D', &[(10, "d-note.txt")]);

        let mut c_content =
            crate::content::ContentIndexBuilder::create(crate::content::content_path(&c)).unwrap();
        c_content.add_text(10, "websocket alpha", 128).unwrap();
        c_content.finish().unwrap();
        crate::freshness::write_sidecar_generation(
            crate::content::content_path(&c),
            crate::freshness::capture_index_generation(&c).unwrap(),
        )
        .unwrap();

        let mut d_content =
            crate::content::ContentIndexBuilder::create(crate::content::content_path(&d)).unwrap();
        d_content.add_text(10, "database beta", 128).unwrap();
        d_content.finish().unwrap();
        crate::freshness::write_sidecar_generation(
            crate::content::content_path(&d),
            crate::freshness::capture_index_generation(&d).unwrap(),
        )
        .unwrap();

        let mut store = MultiLiveSearchStore::open_index_directory(&dir).unwrap();
        let hits = store.search_content("websocket", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].volume, 'C');
        assert_eq!(hits[0].hit.name, "c-note.txt");

        let _ = fs::remove_dir_all(dir);
    }
}
