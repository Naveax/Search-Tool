//! Optional out-of-core parent-to-child adjacency for very large base stores.
//! Never modifies the authoritative index family; an invalid cache rebuilds.
use crate::parent_cache::fingerprint;
use crate::store::SearchStore;
use sha2::{Digest, Sha256};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC: [u8; 8] = *b"STPAGED1";
const VERSION: u32 = 1;
const HEADER_SIZE: u64 = 8 + 4 + 8 + 32 + 32;
const BODY_HASH_OFFSET: u64 = 8 + 4 + 8 + 32;
const PAIR_BYTES: u64 = 16;
const RUN_CAPACITY: usize = 262_144;
// Deliberately finite: on-disk sidecar could reach 320 MB plus temp runs.
pub(crate) const MAX_PAGED_PARENT_RECORDS: u64 = 20_000_000;

#[derive(Debug)]
pub(crate) struct PagedParentIndex {
    source: Mutex<File>,
    count: u64,
}

impl PagedParentIndex {
    pub(crate) fn record_count(&self) -> u64 {
        self.count
    }

    // This routine is only called on the asynchronous worker. Full main-index
    // SHA-256 and sidecar verification never run on the search/UI thread.
    pub(crate) fn load_or_build(path: &Path, base: &mut SearchStore) -> io::Result<Self> {
        let count = base.record_count();
        if count > MAX_PAGED_PARENT_RECORDS {
            return Err(invalid("paged parent adjacency exceeds guarded disk limit"));
        }
        let base_hash = fingerprint(path)?;
        if let Ok(existing) = Self::open_verified(&paged_path(path), count, &base_hash) {
            return Ok(existing);
        }
        Self::build_external(path, base, &base_hash, RUN_CAPACITY)?;
        Self::open_verified(&paged_path(path), count, &base_hash)
    }

    fn open_verified(path: &Path, count: u64, base_hash: &[u8; 32]) -> io::Result<Self> {
        if count > MAX_PAGED_PARENT_RECORDS {
            return Err(invalid("paged sidecar record count exceeds limit"));
        }
        let mut file = File::open(path)?;
        let expected = HEADER_SIZE
            .checked_add(
                count
                    .checked_mul(PAIR_BYTES)
                    .ok_or_else(|| invalid("size overflow"))?,
            )
            .ok_or_else(|| invalid("size overflow"))?;
        if file.metadata()?.len() != expected {
            return Err(invalid("paged sidecar length mismatch"));
        }
        let mut header = [0u8; HEADER_SIZE as usize];
        file.read_exact(&mut header)?;
        if header[..8] != MAGIC
            || u32::from_le_bytes(header[8..12].try_into().unwrap()) != VERSION
            || u64::from_le_bytes(header[12..20].try_into().unwrap()) != count
            || header[20..52] != *base_hash
        {
            return Err(invalid("paged sidecar base generation/format mismatch"));
        }
        let mut expected_hash = [0u8; 32];
        expected_hash.copy_from_slice(&header[52..84]);
        let mut hasher = Sha256::new();
        // One bit per base row catches duplicate/out-of-bounds indices even
        // when the serialized payload has a consistent checksum.
        let mut seen = vec![0u8; count.div_ceil(8) as usize];
        let mut previous = None;
        let mut reader = BufReader::with_capacity(512 * 1024, &mut file);
        for _ in 0..count {
            let mut bytes = [0u8; 16];
            reader.read_exact(&mut bytes)?;
            hasher.update(bytes);
            let pair = (
                u64::from_le_bytes(bytes[..8].try_into().unwrap()),
                u64::from_le_bytes(bytes[8..].try_into().unwrap()),
            );
            if pair.1 >= count || previous.is_some_and(|last| last > pair) {
                return Err(invalid("paged sidecar row out of bounds/unsorted"));
            }
            let byte = &mut seen[(pair.1 / 8) as usize];
            let mask = 1u8 << (pair.1 % 8);
            if *byte & mask != 0 {
                return Err(invalid("paged sidecar duplicate row"));
            }
            *byte |= mask;
            previous = Some(pair);
        }
        if hasher.finalize().as_slice() != expected_hash {
            return Err(invalid("paged sidecar body digest mismatch"));
        }
        drop(reader);
        Ok(Self {
            source: Mutex::new(file),
            count,
        })
    }

    fn build_external(
        path: &Path,
        base: &mut SearchStore,
        base_hash: &[u8; 32],
        capacity: usize,
    ) -> io::Result<()> {
        if capacity == 0 || base.record_count() > MAX_PAGED_PARENT_RECORDS {
            return Err(invalid("invalid paged adjacency build capacity/count"));
        }
        let mut artifacts = TemporaryArtifacts::default();
        let target = paged_path(path);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let unique = format!("{}.{}", std::process::id(), nonce);
        let count = base.record_count();
        let mut run_paths = Vec::new();
        let mut first = 0u64;
        while first < count {
            let length = ((count - first).min(capacity as u64)) as usize;
            let mut rows = Vec::with_capacity(length);
            for index in first..first + length as u64 {
                rows.push((base.read_record(index)?.parent_id, index));
            }
            rows.sort_unstable();
            let run = with_suffix(&target, &format!(".{unique}.run{}.tmp", run_paths.len()));
            artifacts.track(run.clone());
            let mut out = BufWriter::with_capacity(64 * 1024, create_new(&run)?);
            for pair in rows {
                out.write_all(&pair.0.to_le_bytes())?;
                out.write_all(&pair.1.to_le_bytes())?;
            }
            out.flush()?;
            // Runs are scratch files, not published; only the merged output
            // needs durability before it can be promoted.
            run_paths.push(run);
            first += length as u64;
        }

        let staged = with_suffix(&target, &format!(".{unique}.publishing.tmp"));
        artifacts.track(staged.clone());
        let mut output = BufWriter::with_capacity(512 * 1024, create_new(&staged)?);
        output.write_all(&MAGIC)?;
        output.write_all(&VERSION.to_le_bytes())?;
        output.write_all(&count.to_le_bytes())?;
        output.write_all(base_hash)?;
        output.write_all(&[0u8; 32])?; // patched with final body digest

        let mut readers = Vec::with_capacity(run_paths.len());
        let mut heap = BinaryHeap::<Reverse<(u64, u64, usize)>>::new();
        for (run_id, run) in run_paths.iter().enumerate() {
            let mut reader = BufReader::with_capacity(64 * 1024, File::open(run)?);
            if let Some((parent, row)) = next_pair(&mut reader)? {
                heap.push(Reverse((parent, row, run_id)));
            }
            readers.push(reader);
        }
        let mut digest = Sha256::new();
        let mut written = 0u64;
        while let Some(Reverse((parent, row, run_id))) = heap.pop() {
            let a = parent.to_le_bytes();
            let b = row.to_le_bytes();
            output.write_all(&a)?;
            output.write_all(&b)?;
            digest.update(a);
            digest.update(b);
            written += 1;
            if let Some((next_parent, next_row)) = next_pair(&mut readers[run_id])? {
                heap.push(Reverse((next_parent, next_row, run_id)));
            }
        }
        if written != count {
            return Err(invalid("external parent merge lost records"));
        }
        output.flush()?;
        let mut output_file = output.into_inner().map_err(|error| error.into_error())?;
        output_file.seek(SeekFrom::Start(BODY_HASH_OFFSET))?;
        output_file.write_all(&digest.finalize())?;
        output_file.sync_all()?;
        drop(output_file);
        // Publishing only this optional file does not alter the authoritative
        // five-file index family; the next worker always validates main hash.
        #[cfg(windows)]
        if target.exists() {
            fs::remove_file(&target)?;
        }
        fs::rename(&staged, &target)?;
        #[cfg(unix)]
        if let Some(dir) = target.parent() {
            File::open(dir)?.sync_all()?;
        }
        // Every run and any leftover staging file is removed by Drop,
        // including on early error.
        Ok(())
    }

    /// Visits only children of this parent via two on-disk binary searches.
    /// No Vec is created for very wide folders. Callback false stops early.
    pub(crate) fn visit_children(
        &self,
        parent: u64,
        mut visit: impl FnMut(u64) -> io::Result<bool>,
    ) -> io::Result<()> {
        let mut file = self
            .source
            .lock()
            .map_err(|_| io::Error::other("paged reader poisoned"))?;
        let begin = partition(&mut file, self.count, parent, false)?;
        let end = partition(&mut file, self.count, parent, true)?;
        file.seek(SeekFrom::Start(HEADER_SIZE + begin * PAIR_BYTES))?;
        for _ in begin..end {
            let pair = read_pair(&mut *file)?;
            if !visit(pair.1)? {
                break;
            }
        }
        Ok(())
    }
}

fn partition(file: &mut File, count: u64, parent: u64, inclusive: bool) -> io::Result<u64> {
    let (mut low, mut high) = (0, count);
    while low < high {
        let middle = low + (high - low) / 2;
        file.seek(SeekFrom::Start(HEADER_SIZE + middle * PAIR_BYTES))?;
        let candidate = read_pair(file)?.0;
        if candidate < parent || (inclusive && candidate == parent) {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    Ok(low)
}

fn read_pair(input: &mut impl Read) -> io::Result<(u64, u64)> {
    let mut bytes = [0u8; 16];
    input.read_exact(&mut bytes)?;
    Ok((
        u64::from_le_bytes(bytes[..8].try_into().unwrap()),
        u64::from_le_bytes(bytes[8..].try_into().unwrap()),
    ))
}

fn next_pair(input: &mut impl Read) -> io::Result<Option<(u64, u64)>> {
    let mut first = [0u8; 1];
    if input.read(&mut first)? == 0 {
        return Ok(None);
    }
    let mut tail = [0u8; 15];
    input.read_exact(&mut tail)?;
    let mut pair = [0u8; 16];
    pair[0] = first[0];
    pair[1..].copy_from_slice(&tail);
    Ok(Some((
        u64::from_le_bytes(pair[..8].try_into().unwrap()),
        u64::from_le_bytes(pair[8..].try_into().unwrap()),
    )))
}

#[derive(Default)]
struct TemporaryArtifacts(Vec<PathBuf>);
impl TemporaryArtifacts {
    fn track(&mut self, path: PathBuf) {
        self.0.push(path);
    }
}
impl Drop for TemporaryArtifacts {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}
fn create_new(path: &Path) -> io::Result<File> {
    OpenOptions::new().create_new(true).write(true).open(path)
}
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
pub(crate) fn paged_path(path: &Path) -> PathBuf {
    with_suffix(path, ".parents.paged")
}
fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{BuildOptions, IndexBuilder, InputRecord, FLAG_DIRECTORY};

    fn fixture() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "search-paged-parent-{}-{nonce}.stidx",
            std::process::id()
        ));
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        builder
            .push(InputRecord {
                file_id: 1,
                parent_id: 1,
                name: "C:",
                flags: FLAG_DIRECTORY,
                size_bytes: 0,
            })
            .unwrap();
        for row in 0..777u64 {
            let parent_id = if row % 7 == 0 {
                1
            } else if row % 11 == 0 {
                2
            } else {
                3 + row % 17
            };
            builder
                .push(InputRecord {
                    file_id: 100 + row,
                    parent_id,
                    name: "record",
                    flags: 0,
                    size_bytes: 0,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        path
    }

    fn cleanup(path: &Path) {
        for suffix in ["", ".names", ".ids", ".ncp", ".icp", ".parents.paged"] {
            let _ = fs::remove_file(with_suffix(path, suffix));
        }
    }

    #[test]
    fn external_chunks_merge_and_disk_binary_search_cover_all_parent_rows() {
        let path = fixture();
        let mut base = SearchStore::open(&path).unwrap();
        let hash = fingerprint(&path).unwrap();
        PagedParentIndex::build_external(&path, &mut base, &hash, 37).unwrap();
        let paged = PagedParentIndex::open_verified(&paged_path(&path), base.record_count(), &hash)
            .unwrap();
        let mut all = Vec::new();
        for parent in 0..25 {
            let mut found = Vec::new();
            paged
                .visit_children(parent, |row| {
                    found.push(row);
                    Ok(true)
                })
                .unwrap();
            for row in &found {
                assert_eq!(base.read_record(*row).unwrap().parent_id, parent);
            }
            all.extend(found);
        }
        all.sort_unstable();
        assert_eq!(all, (0..base.record_count()).collect::<Vec<_>>());
        let mut first = Vec::new();
        paged
            .visit_children(1, |row| {
                first.push(row);
                Ok(false)
            })
            .unwrap();
        assert_eq!(first.len(), 1);
        drop(paged);
        cleanup(&path);
    }

    #[test]
    fn corrupted_or_stale_paged_cache_is_rebuilt() {
        let path = fixture();
        let mut base = SearchStore::open(&path).unwrap();
        let hash = fingerprint(&path).unwrap();
        PagedParentIndex::build_external(&path, &mut base, &hash, 83).unwrap();
        let sidecar = paged_path(&path);
        assert!(PagedParentIndex::open_verified(&sidecar, base.record_count(), &[0; 32]).is_err());
        let mut bytes = fs::read(&sidecar).unwrap();
        bytes[HEADER_SIZE as usize + 13] ^= 0x50;
        fs::write(&sidecar, &bytes).unwrap();
        assert!(PagedParentIndex::open_verified(&sidecar, base.record_count(), &hash).is_err());
        let regenerated = PagedParentIndex::load_or_build(&path, &mut base).unwrap();
        assert_eq!(regenerated.record_count(), 778);
        drop(regenerated);
        fs::write(&sidecar, b"partial").unwrap();
        let reopened = PagedParentIndex::load_or_build(&path, &mut base).unwrap();
        assert_eq!(reopened.record_count(), 778);
        drop(reopened);
        cleanup(&path);
    }
}
