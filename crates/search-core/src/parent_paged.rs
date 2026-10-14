//! Optional out-of-core parent-to-child adjacency for very large base stores.
//! Never modifies the authoritative index family; an invalid cache rebuilds.
use crate::store::SearchStore;
use sha2::{Digest, Sha256};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAGIC: [u8; 8] = *b"STPAGED1";
const VERSION: u32 = 1;
const HEADER_SIZE: u64 = 8 + 4 + 8 + 32 + 32;
const BODY_HASH_OFFSET: u64 = 8 + 4 + 8 + 32;
const PAIR_BYTES: u64 = 16;
const RUN_CAPACITY: usize = 262_144;
const BUILD_LOCK_WAIT: Duration = Duration::from_secs(300);
const BUILD_LOCK_RETRY: Duration = Duration::from_millis(50);
const ORPHAN_MIN_AGE: Duration = Duration::from_secs(60 * 60);
const MAX_ORPHAN_CLEANUPS: usize = 512;
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
    #[cfg(test)]
    pub(crate) fn load_or_build(path: &Path, base: &mut SearchStore) -> io::Result<Self> {
        Self::load_or_build_cancellable(path, base, &AtomicBool::new(false))
    }

    pub(crate) fn load_or_build_cancellable(
        path: &Path,
        base: &mut SearchStore,
        cancel: &AtomicBool,
    ) -> io::Result<Self> {
        let count = base.record_count();
        if count > MAX_PAGED_PARENT_RECORDS {
            return Err(invalid("paged parent adjacency exceeds guarded disk limit"));
        }
        let base_hash = fingerprint_cancellable(path, cancel)?;
        if let Ok(existing) =
            Self::open_verified_cancellable(&paged_path(path), count, &base_hash, cancel)
        {
            return Ok(existing);
        }
        // A per-index OS lock prevents live writers from racing orphan
        // scavenging or publishing two independent optional cache files.
        let _build_lock = acquire_build_lock(path, cancel)?;
        check_cancel(cancel)?;
        if let Ok(existing) =
            Self::open_verified_cancellable(&paged_path(path), count, &base_hash, cancel)
        {
            return Ok(existing);
        }
        let _ = cleanup_orphan_scratch(path);
        Self::build_external_cancellable(path, base, &base_hash, RUN_CAPACITY, cancel)?;
        check_cancel(cancel)?;
        Self::open_verified_cancellable(&paged_path(path), count, &base_hash, cancel)
    }

    #[cfg(test)]
    fn open_verified(path: &Path, count: u64, base_hash: &[u8; 32]) -> io::Result<Self> {
        Self::open_verified_cancellable(path, count, base_hash, &AtomicBool::new(false))
    }

    fn open_verified_cancellable(
        path: &Path,
        count: u64,
        base_hash: &[u8; 32],
        cancel: &AtomicBool,
    ) -> io::Result<Self> {
        check_cancel(cancel)?;
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
        for index in 0..count {
            if index.is_multiple_of(8192) {
                check_cancel(cancel)?;
            }
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

    #[cfg(test)]
    fn build_external(
        path: &Path,
        base: &mut SearchStore,
        base_hash: &[u8; 32],
        capacity: usize,
    ) -> io::Result<()> {
        Self::build_external_cancellable(path, base, base_hash, capacity, &AtomicBool::new(false))
    }

    fn build_external_cancellable(
        path: &Path,
        base: &mut SearchStore,
        base_hash: &[u8; 32],
        capacity: usize,
        cancel: &AtomicBool,
    ) -> io::Result<()> {
        check_cancel(cancel)?;
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
            check_cancel(cancel)?;
            let length = ((count - first).min(capacity as u64)) as usize;
            let mut rows = Vec::with_capacity(length);
            for index in first..first + length as u64 {
                if index.is_multiple_of(8192) {
                    check_cancel(cancel)?;
                }
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
            if written.is_multiple_of(8192) {
                check_cancel(cancel)?;
            }
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
        check_cancel(cancel)?;
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

fn check_cancel(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "out-of-core parent build cancelled for stale generation",
        ))
    } else {
        Ok(())
    }
}

fn fingerprint_cancellable(path: &Path, cancel: &AtomicBool) -> io::Result<[u8; 32]> {
    let mut reader = BufReader::with_capacity(512 * 1024, File::open(path)?);
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        check_cancel(cancel)?;
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest.finalize().into())
}

// An OS-held lock, not the presence of a file, is the authority: a crashed
// worker leaves a lock file but its kernel lock is automatically released.
fn acquire_build_lock(index_path: &Path, cancel: &AtomicBool) -> io::Result<File> {
    let path = with_suffix(&paged_path(index_path), ".build.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let start = Instant::now();
    loop {
        check_cancel(cancel)?;
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) if start.elapsed() < BUILD_LOCK_WAIT => {
                thread::sleep(BUILD_LOCK_RETRY);
            }
            Err(TryLockError::WouldBlock) => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "out-of-core parent build lock timeout",
                ));
            }
            Err(TryLockError::Error(error)) => return Err(error),
        }
    }
}

// Remove ONLY our precisely shaped scratch files, older than one hour,
// after exclusive build.lock acquisition. Fresh files are never deleted,
// even if left by a previous process. Never touch .parents.paged itself,
// the authoritative index family, or unrelated similarly prefixed files.
fn cleanup_orphan_scratch(index_path: &Path) -> io::Result<usize> {
    let target = paged_path(index_path);
    let Some(name) = target.file_name().and_then(|name| name.to_str()) else {
        return Ok(0);
    };
    let Some(parent) = target.parent() else {
        return Ok(0);
    };
    let prefix = format!("{name}.");
    let mut removed = 0;
    for entry in fs::read_dir(parent)? {
        if removed >= MAX_ORPHAN_CLEANUPS {
            break;
        }
        let entry = entry?;
        let Some(filename) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(suffix) = filename.strip_prefix(&prefix) else {
            continue;
        };
        let parts: Vec<_> = suffix.split('.').collect();
        let valid_name = match parts.as_slice() {
            [pid, nonce, "publishing", "tmp"] => {
                pid.parse::<u32>().is_ok() && nonce.parse::<u128>().is_ok()
            }
            [pid, nonce, run, "tmp"] => {
                pid.parse::<u32>().is_ok()
                    && nonce.parse::<u128>().is_ok()
                    && run
                        .strip_prefix("run")
                        .is_some_and(|num| num.parse::<usize>().is_ok())
            }
            _ => false,
        };
        if !valid_name {
            continue;
        }
        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(metadata) if metadata.file_type().is_file() => metadata,
            _ => continue,
        };
        let old_enough = metadata
            .modified()
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age >= ORPHAN_MIN_AGE);
        if old_enough && fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
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
    use crate::parent_cache::fingerprint;
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
        for suffix in [
            "",
            ".names",
            ".ids",
            ".ncp",
            ".icp",
            ".parents.paged",
            ".parents.paged.build.lock",
        ] {
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
    fn cancellation_rejects_stale_build_even_before_cache_io() {
        let path = fixture();
        let mut base = SearchStore::open(&path).unwrap();
        let cancel = AtomicBool::new(true);
        let error =
            PagedParentIndex::load_or_build_cancellable(&path, &mut base, &cancel).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(!paged_path(&path).exists());
        let normal = PagedParentIndex::load_or_build(&path, &mut base).unwrap();
        assert_eq!(normal.record_count(), 778);
        drop(normal);
        let error =
            PagedParentIndex::load_or_build_cancellable(&path, &mut base, &cancel).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        cleanup(&path);
    }

    #[test]
    fn orphan_cleanup_requires_lock_and_preserves_fresh_and_unrelated_files() {
        let path = fixture();
        let target = paged_path(&path);
        let old_run = with_suffix(&target, ".123.456.run0.tmp");
        let old_staged = with_suffix(&target, ".123.456.publishing.tmp");
        let recent = with_suffix(&target, ".123.456.run1.tmp");
        let unrelated = with_suffix(&target, ".123.456.unrelated.tmp");
        let unrelated_prefix = with_suffix(&target, ".evil.tmp");
        for file in [
            &old_run,
            &old_staged,
            &recent,
            &unrelated,
            &unrelated_prefix,
            &target,
        ] {
            fs::write(file, b"keep or remove").unwrap();
        }
        let old =
            fs::FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(2 * 60 * 60));
        for file in [&old_run, &old_staged, &unrelated, &unrelated_prefix] {
            OpenOptions::new()
                .write(true)
                .open(file)
                .unwrap()
                .set_times(old)
                .unwrap();
        }
        let cancel = AtomicBool::new(false);
        let guard = acquire_build_lock(&path, &cancel).unwrap();
        let competing = OpenOptions::new()
            .read(true)
            .write(true)
            .open(with_suffix(&target, ".build.lock"))
            .unwrap();
        assert!(matches!(
            competing.try_lock(),
            Err(TryLockError::WouldBlock)
        ));
        assert_eq!(cleanup_orphan_scratch(&path).unwrap(), 2);
        assert!(!old_run.exists());
        assert!(!old_staged.exists());
        assert!(recent.exists());
        assert!(unrelated.exists());
        assert!(unrelated_prefix.exists());
        assert!(target.exists());
        drop(competing);
        drop(guard);
        for file in [
            recent,
            unrelated,
            unrelated_prefix,
            target,
            with_suffix(&paged_path(&path), ".build.lock"),
        ] {
            let _ = fs::remove_file(file);
        }
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
