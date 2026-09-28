use crate::delta::{delta_path, delta_record_count, for_each_delta_record, DeltaOp, DeltaRecord};
use crate::index_lock::IndexMutationGuard;
use crate::store::{
    id_checkpoints_path, ids_path, name_checkpoints_path, names_path, BuildOptions, IndexBuilder,
    InputRecord, SearchStore, StoreStats,
};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const DELTA_SORT_CHUNK_ENTRIES: usize = 16_384;
const DELTA_SORT_CHUNK_BYTES: usize = 4 * 1024 * 1024;
const MAX_OVERRIDE_BITMAP_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifyReport {
    pub records: u64,
    pub memory_hint_bytes: usize,
    pub delta_entries: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeepVerifyReport {
    pub records: u64,
    pub names_checked: u64,
    pub ids_checked: u64,
    pub parent_links_missing: u64,
}

pub fn verify_index(index_path: impl AsRef<Path>) -> io::Result<VerifyReport> {
    let index_path = index_path.as_ref();
    recover_compaction(index_path)?;
    let store = SearchStore::open(index_path)?;
    let delta_records = delta_record_count(delta_path(index_path))?;
    Ok(VerifyReport {
        records: store.record_count(),
        memory_hint_bytes: store.memory_hint_bytes(),
        delta_entries: usize::try_from(delta_records).unwrap_or(usize::MAX),
    })
}

pub fn verify_index_deep(index_path: impl AsRef<Path>) -> io::Result<DeepVerifyReport> {
    let index_path = index_path.as_ref();
    recover_compaction(index_path)?;
    let mut store = SearchStore::open(index_path)?;
    let record_count_u64 = store.record_count();
    let record_count = usize::try_from(record_count_u64)
        .map_err(|_| io::Error::new(io::ErrorKind::OutOfMemory, "index is too large to verify"))?;

    // Deep verification is an explicit maintenance operation, so spend a bounded
    // amount of RAM to avoid millions of random sidecar seeks. At 10M records
    // these two u64 vectors plus Vec<bool> stay well below the normal 4 GB target.
    let mut record_ids = vec![0_u64; record_count];
    let mut record_seen = vec![false; record_count];
    let mut sorted_ids = Vec::with_capacity(record_count);
    let mut last_file_id = None;

    store.for_each_id_entry(|file_id, record_index| {
        let index = usize::try_from(record_index)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "record index overflow"))?;
        if index >= record_count {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file-id sidecar points outside the record table",
            ));
        }
        if record_seen[index] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file-id sidecar contains a duplicate record index",
            ));
        }
        if last_file_id.is_some_and(|previous| file_id <= previous) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file-id sidecar is not strictly sorted",
            ));
        }

        record_seen[index] = true;
        record_ids[index] = file_id;
        sorted_ids.push(file_id);
        last_file_id = Some(file_id);
        Ok(())
    })?;

    if sorted_ids.len() != record_count || record_seen.iter().any(|seen| !seen) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file-id sidecar is missing a record",
        ));
    }

    let mut report = DeepVerifyReport {
        records: record_count_u64,
        names_checked: 0,
        ids_checked: 0,
        parent_links_missing: 0,
    };
    for record_index in 0..record_count_u64 {
        let index = record_index as usize;
        let record = store.read_record(record_index)?;
        let _ = store.read_name(record)?;
        report.names_checked = report.names_checked.saturating_add(1);

        if record.file_id != record_ids[index] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file-id sidecar points at the wrong record",
            ));
        }
        report.ids_checked = report.ids_checked.saturating_add(1);

        if record.parent_id != 0
            && record.parent_id != record.file_id
            && sorted_ids.binary_search(&record.parent_id).is_err()
        {
            report.parent_links_missing = report.parent_links_missing.saturating_add(1);
        }
    }
    Ok(report)
}

pub fn repair_index_sidecars(index_path: impl AsRef<Path>) -> io::Result<()> {
    let index_path = index_path.as_ref();
    let _guard = IndexMutationGuard::try_acquire(index_path)?;
    recover_compaction_unlocked(index_path)?;
    crate::store::repair_sidecars(index_path)
}

pub fn compact_index(index_path: impl AsRef<Path>) -> io::Result<StoreStats> {
    compact_index_impl(
        index_path.as_ref(),
        DELTA_SORT_CHUNK_ENTRIES,
        DELTA_SORT_CHUNK_BYTES,
    )
}

fn compact_index_impl(
    index_path: &Path,
    sort_chunk_entries: usize,
    sort_chunk_bytes: usize,
) -> io::Result<StoreStats> {
    let _guard = IndexMutationGuard::try_acquire(index_path)?;
    recover_compaction_unlocked(index_path)?;
    cleanup_delta_sort_temps(index_path);
    let delta_file = delta_path(index_path);
    if delta_record_count(&delta_file)? == 0 {
        let store = SearchStore::open(index_path)?;
        return Ok(StoreStats {
            records: store.record_count(),
            string_pool_bytes: 0,
            index_bytes: fs::metadata(index_path)?.len(),
            name_index_bytes: fs::metadata(names_path(index_path))?.len(),
            id_index_bytes: fs::metadata(ids_path(index_path))?.len(),
        });
    }

    let sorted = sort_latest_delta(
        &delta_file,
        index_path,
        sort_chunk_entries.max(1),
        sort_chunk_bytes.max(1024),
    )?;

    let staging = suffix(index_path, ".compact.new");
    cleanup_family(&staging);
    let mut builder = IndexBuilder::create(&staging, BuildOptions::default())?;
    let mut base = SearchStore::open(index_path)?;
    let mut overridden = OverrideBitmap::new(base.record_count())?;
    mark_overridden_records(&mut base, &sorted.merged, &mut overridden)?;

    for record_index in 0..base.record_count() {
        if overridden.get(record_index) {
            continue;
        }
        let record = base.read_record(record_index)?;
        let name = base.read_name(record)?;
        builder.push(InputRecord {
            file_id: record.file_id,
            parent_id: record.parent_id,
            size_bytes: record.size_bytes,
            flags: record.flags,
            name: &name,
        })?;
    }
    drop(base);

    let mut delta = SortedDeltaReader::open(&sorted.merged)?;
    while let Some(entry) = delta.next_entry()? {
        if entry.record.op != DeltaOp::Upsert {
            continue;
        }
        builder.push(InputRecord {
            file_id: entry.record.file_id,
            parent_id: entry.record.parent_id,
            size_bytes: entry.record.size_bytes,
            flags: entry.record.flags,
            name: &entry.record.name,
        })?;
    }

    let stats = builder.finish()?;
    commit_compaction(index_path, &staging)?;
    if delta_file.exists() {
        fs::remove_file(delta_file)?;
    }
    Ok(stats)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SequencedDelta {
    sequence: u64,
    record: DeltaRecord,
}

impl SequencedDelta {
    fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>().saturating_add(self.record.name.len())
    }
}

#[derive(Debug)]
struct SortedDeltaFiles {
    merged: PathBuf,
    cleanup: Vec<PathBuf>,
}

impl Drop for SortedDeltaFiles {
    fn drop(&mut self) {
        for path in &self.cleanup {
            let _ = fs::remove_file(path);
        }
    }
}

fn sort_latest_delta(
    delta_file: &Path,
    index_path: &Path,
    chunk_entry_limit: usize,
    chunk_byte_limit: usize,
) -> io::Result<SortedDeltaFiles> {
    let mut chunks = Vec::new();
    let mut current = Vec::<SequencedDelta>::with_capacity(chunk_entry_limit.min(16_384));
    let mut current_bytes = 0usize;
    let mut chunk_number = 0usize;

    for_each_delta_record(delta_file, |sequence, record| {
        let entry = SequencedDelta { sequence, record };
        current_bytes = current_bytes.saturating_add(entry.estimated_bytes());
        current.push(entry);
        if current.len() >= chunk_entry_limit || current_bytes >= chunk_byte_limit {
            let path = delta_sort_chunk_path(index_path, chunk_number);
            flush_delta_chunk(&path, &mut current)?;
            chunks.push(path);
            chunk_number = chunk_number.saturating_add(1);
            current_bytes = 0;
        }
        Ok(())
    })?;
    if !current.is_empty() {
        let path = delta_sort_chunk_path(index_path, chunk_number);
        flush_delta_chunk(&path, &mut current)?;
        chunks.push(path);
    }

    let merged = suffix(index_path, ".delta-sort.merged.tmp");
    let _ = fs::remove_file(&merged);
    merge_delta_chunks(&chunks, &merged)?;
    let mut cleanup = chunks;
    cleanup.push(merged.clone());
    Ok(SortedDeltaFiles { merged, cleanup })
}

fn cleanup_delta_sort_temps(index_path: &Path) {
    let Some(parent) = index_path.parent() else {
        return;
    };
    let Some(base_name) = index_path.file_name().map(|name| name.to_string_lossy()) else {
        return;
    };
    let prefix = format!("{base_name}.delta-sort.");
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&prefix) && name.ends_with(".tmp") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn delta_sort_chunk_path(index_path: &Path, chunk_number: usize) -> PathBuf {
    suffix(index_path, &format!(".delta-sort.{chunk_number}.tmp"))
}

fn flush_delta_chunk(path: &Path, records: &mut Vec<SequencedDelta>) -> io::Result<()> {
    records.sort_unstable_by(|a, b| {
        a.record
            .file_id
            .cmp(&b.record.file_id)
            .then(a.sequence.cmp(&b.sequence))
    });
    let mut out = BufWriter::with_capacity(128 * 1024, File::create(path)?);
    let mut position = 0usize;
    while position < records.len() {
        let file_id = records[position].record.file_id;
        let mut end = position + 1;
        while end < records.len() && records[end].record.file_id == file_id {
            end += 1;
        }
        write_sorted_delta(&mut out, &records[end - 1])?;
        position = end;
    }
    out.flush()?;
    out.get_ref().sync_all()?;
    records.clear();
    Ok(())
}

#[derive(Debug)]
struct SortedDeltaReader {
    input: BufReader<File>,
}

impl SortedDeltaReader {
    fn open(path: &Path) -> io::Result<Self> {
        Ok(Self {
            input: BufReader::with_capacity(128 * 1024, File::open(path)?),
        })
    }

    fn next_entry(&mut self) -> io::Result<Option<SequencedDelta>> {
        read_sorted_delta(&mut self.input)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct DeltaHeapItem {
    entry: SequencedDelta,
    source: usize,
}

impl Ord for DeltaHeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .entry
            .record
            .file_id
            .cmp(&self.entry.record.file_id)
            .then_with(|| other.source.cmp(&self.source))
    }
}

impl PartialOrd for DeltaHeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn merge_delta_chunks(chunks: &[PathBuf], output: &Path) -> io::Result<()> {
    let mut out = BufWriter::with_capacity(128 * 1024, File::create(output)?);
    if chunks.is_empty() {
        out.flush()?;
        return Ok(());
    }

    let mut readers: Vec<_> = chunks
        .iter()
        .map(|path| SortedDeltaReader::open(path))
        .collect::<io::Result<_>>()?;
    let mut heap = BinaryHeap::new();
    for (source, reader) in readers.iter_mut().enumerate() {
        if let Some(entry) = reader.next_entry()? {
            heap.push(DeltaHeapItem { entry, source });
        }
    }

    while let Some(item) = heap.pop() {
        let file_id = item.entry.record.file_id;
        let mut best = item.entry;
        let source = item.source;
        if let Some(next) = readers[source].next_entry()? {
            heap.push(DeltaHeapItem {
                entry: next,
                source,
            });
        }

        while heap
            .peek()
            .is_some_and(|candidate| candidate.entry.record.file_id == file_id)
        {
            let candidate = heap.pop().expect("heap peek guaranteed an item");
            if candidate.entry.sequence > best.sequence {
                best = candidate.entry;
            }
            let source = candidate.source;
            if let Some(next) = readers[source].next_entry()? {
                heap.push(DeltaHeapItem {
                    entry: next,
                    source,
                });
            }
        }
        write_sorted_delta(&mut out, &best)?;
    }
    out.flush()?;
    out.get_ref().sync_all()
}

fn write_sorted_delta<W: Write>(out: &mut W, entry: &SequencedDelta) -> io::Result<()> {
    let name = entry.record.name.as_bytes();
    if name.len() > u32::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "sorted delta filename too large",
        ));
    }
    out.write_all(&entry.sequence.to_le_bytes())?;
    out.write_all(&[entry.record.op as u8])?;
    out.write_all(&[0_u8])?;
    out.write_all(&entry.record.flags.to_le_bytes())?;
    out.write_all(&entry.record.file_id.to_le_bytes())?;
    out.write_all(&entry.record.parent_id.to_le_bytes())?;
    out.write_all(&entry.record.size_bytes.to_le_bytes())?;
    out.write_all(&(name.len() as u32).to_le_bytes())?;
    out.write_all(name)
}

fn read_sorted_delta<R: Read>(input: &mut R) -> io::Result<Option<SequencedDelta>> {
    let mut sequence = [0_u8; 8];
    match input.read_exact(&mut sequence) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let mut op = [0_u8; 1];
    input.read_exact(&mut op)?;
    let mut padding = [0_u8; 1];
    input.read_exact(&mut padding)?;
    let flags = read_u16(input)?;
    let file_id = read_u64(input)?;
    let parent_id = read_u64(input)?;
    let size_bytes = read_u64(input)?;
    let name_len = read_u32(input)? as usize;
    if name_len > 64 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "sorted delta filename exceeds safety limit",
        ));
    }
    let mut name = vec![0_u8; name_len];
    input.read_exact(&mut name)?;
    let name = String::from_utf8(name).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "sorted delta filename is not UTF-8",
        )
    })?;
    let op = match op[0] {
        1 => DeltaOp::Upsert,
        2 => DeltaOp::Delete,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid sorted delta operation",
            ))
        }
    };
    Ok(Some(SequencedDelta {
        sequence: u64::from_le_bytes(sequence),
        record: DeltaRecord {
            op,
            file_id,
            parent_id,
            size_bytes,
            flags,
            name,
        },
    }))
}

fn read_u16<R: Read>(input: &mut R) -> io::Result<u16> {
    let mut bytes = [0_u8; 2];
    input.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32<R: Read>(input: &mut R) -> io::Result<u32> {
    let mut bytes = [0_u8; 4];
    input.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64<R: Read>(input: &mut R) -> io::Result<u64> {
    let mut bytes = [0_u8; 8];
    input.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[derive(Debug)]
struct OverrideBitmap {
    bytes: Vec<u8>,
}

impl OverrideBitmap {
    fn new(record_count: u64) -> io::Result<Self> {
        let byte_count_u64 = record_count.saturating_add(7) / 8;
        let byte_count = usize::try_from(byte_count_u64).map_err(|_| {
            io::Error::new(io::ErrorKind::OutOfMemory, "override bitmap is too large")
        })?;
        if byte_count > MAX_OVERRIDE_BITMAP_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "base index is too large for bounded compaction bitmap; rebuild the index",
            ));
        }
        Ok(Self {
            bytes: vec![0_u8; byte_count],
        })
    }

    fn set(&mut self, record_index: u64) -> io::Result<()> {
        let byte = usize::try_from(record_index / 8).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "record index exceeds address space",
            )
        })?;
        let Some(slot) = self.bytes.get_mut(byte) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "record index exceeds override bitmap",
            ));
        };
        *slot |= 1_u8 << (record_index % 8);
        Ok(())
    }

    fn get(&self, record_index: u64) -> bool {
        let Ok(byte) = usize::try_from(record_index / 8) else {
            return false;
        };
        self.bytes
            .get(byte)
            .is_some_and(|slot| slot & (1_u8 << (record_index % 8)) != 0)
    }
}

fn mark_overridden_records(
    base: &mut SearchStore,
    sorted_delta: &Path,
    bitmap: &mut OverrideBitmap,
) -> io::Result<()> {
    let mut delta = SortedDeltaReader::open(sorted_delta)?;
    let mut current = delta.next_entry()?;
    base.for_each_id_entry(|file_id, record_index| {
        while current
            .as_ref()
            .is_some_and(|entry| entry.record.file_id < file_id)
        {
            current = delta.next_entry()?;
        }
        if current
            .as_ref()
            .is_some_and(|entry| entry.record.file_id == file_id)
        {
            bitmap.set(record_index)?;
        }
        Ok(())
    })
}

/// Publishes a fully-built staging index family over an existing complete
/// index using the same crash-recoverable backup/marker protocol as compaction.
/// The staging path must already contain main/.names/.ids/.ncp/.icp files.
pub fn publish_staged_index(
    index_path: impl AsRef<Path>,
    staging_path: impl AsRef<Path>,
) -> io::Result<()> {
    let index_path = index_path.as_ref();
    let staging_path = staging_path.as_ref();
    let _guard = IndexMutationGuard::try_acquire(index_path)?;
    recover_compaction_unlocked(index_path)?;

    if !family(index_path).iter().all(|path| path.exists()) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "existing index family is incomplete; repair or rebuild from scratch",
        ));
    }
    if !family(staging_path).iter().all(|path| path.exists()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "staged index family is incomplete",
        ));
    }
    commit_compaction(index_path, staging_path)
}

pub fn remove_index_family(index_path: impl AsRef<Path>) {
    cleanup_family(index_path.as_ref());
}

pub fn recover_compaction(index_path: impl AsRef<Path>) -> io::Result<()> {
    let index_path = index_path.as_ref();
    let _guard = IndexMutationGuard::try_acquire(index_path)?;
    recover_compaction_unlocked(index_path)
}

fn recover_compaction_unlocked(index_path: &Path) -> io::Result<()> {
    let marker = suffix(index_path, ".compact.pending");
    if !marker.exists() {
        return Ok(());
    }

    let final_family = family(index_path);
    let backups = backup_family(index_path);
    let finals_complete = final_family.iter().all(|path| path.exists());

    if finals_complete {
        for backup in backups {
            let _ = fs::remove_file(backup);
        }
    } else {
        for final_path in &final_family {
            let _ = fs::remove_file(final_path);
        }
        for (backup, final_path) in backups.iter().zip(final_family.iter()) {
            if !backup.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "compaction recovery backup is missing",
                ));
            }
            fs::rename(backup, final_path)?;
        }
    }

    cleanup_family(&suffix(index_path, ".compact.new"));
    let _ = fs::remove_file(marker);
    Ok(())
}

fn commit_compaction(index_path: &Path, staging: &Path) -> io::Result<()> {
    let finals = family(index_path);
    let staged = family(staging);
    let backups = backup_family(index_path);

    for backup in &backups {
        let _ = fs::remove_file(backup);
    }
    for (final_path, backup) in finals.iter().zip(backups.iter()) {
        fs::hard_link(final_path, backup)?;
    }

    let marker = suffix(index_path, ".compact.pending");
    {
        let mut out = File::create(&marker)?;
        out.write_all(b"Search Tool compaction pending\n")?;
        out.sync_all()?;
    }

    let replacement = (|| -> io::Result<()> {
        // Sidecars first, main index last. A surviving main file therefore means
        // the entire new family was installed before a crash.
        for position in [1usize, 2, 3, 4, 0] {
            fs::remove_file(&finals[position])?;
            fs::rename(&staged[position], &finals[position])?;
        }
        Ok(())
    })();

    if let Err(error) = replacement {
        let _ = recover_compaction_unlocked(index_path);
        return Err(error);
    }

    for backup in backups {
        let _ = fs::remove_file(backup);
    }
    let _ = fs::remove_file(marker);
    cleanup_family(staging);
    Ok(())
}

fn family(base: &Path) -> [PathBuf; 5] {
    [
        base.to_path_buf(),
        names_path(base),
        ids_path(base),
        name_checkpoints_path(base),
        id_checkpoints_path(base),
    ]
}

fn backup_family(base: &Path) -> [PathBuf; 5] {
    let backup = suffix(base, ".compact.bak");
    family(&backup)
}

fn cleanup_family(base: &Path) {
    for path in family(base) {
        let _ = fs::remove_file(path);
    }
}

fn suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{DeltaRecord, DeltaWriter};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("search-tool-compact-{n}.stidx"))
    }

    fn build_family(path: &Path, files: &[(u64, u64, &str)]) {
        let mut builder = IndexBuilder::create(path, BuildOptions::default()).unwrap();
        for &(id, parent, name) in files {
            builder
                .push(InputRecord {
                    file_id: id,
                    parent_id: parent,
                    size_bytes: 0,
                    flags: 0,
                    name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
    }

    #[test]
    fn staged_publish_replaces_complete_family() {
        let path = temp();
        let staging = suffix(&path, ".rebuild.test");
        build_family(&path, &[(1, 0, "root"), (2, 1, "old.txt")]);
        build_family(&staging, &[(1, 0, "root"), (2, 1, "new.txt")]);

        publish_staged_index(&path, &staging).unwrap();

        let mut store = SearchStore::open(&path).unwrap();
        assert!(store.search_exact("new.txt", 4).unwrap().len() == 1);
        assert!(store.search_exact("old.txt", 4).unwrap().is_empty());
        assert!(family(&staging).iter().all(|entry| !entry.exists()));
        assert!(!suffix(&path, ".compact.pending").exists());
        for file in family(&path) {
            let _ = fs::remove_file(file);
        }
    }

    #[test]
    fn staged_publish_rejects_incomplete_staging_without_touching_old_family() {
        let path = temp();
        let staging = suffix(&path, ".rebuild.bad");
        build_family(&path, &[(1, 0, "root"), (2, 1, "safe.txt")]);
        build_family(&staging, &[(1, 0, "root"), (2, 1, "new.txt")]);
        fs::remove_file(names_path(&staging)).unwrap();

        assert!(publish_staged_index(&path, &staging).is_err());

        let mut store = SearchStore::open(&path).unwrap();
        assert!(store.search_exact("safe.txt", 4).unwrap().len() == 1);
        assert!(store.search_exact("new.txt", 4).unwrap().is_empty());
        cleanup_family(&path);
        cleanup_family(&staging);
    }

    #[test]
    fn compaction_handles_more_than_live_overlay_limit() {
        let path = temp();
        build_family(&path, &[(1, 0, "root")]);

        let mut delta = DeltaWriter::open(delta_path(&path)).unwrap();
        let changes = crate::live::DEFAULT_MAX_DELTA_ENTRIES as u64 + 512;
        for offset in 0..changes {
            let file_id = offset + 2;
            delta
                .append(&DeltaRecord {
                    op: DeltaOp::Upsert,
                    file_id,
                    parent_id: 1,
                    size_bytes: offset,
                    flags: 0,
                    name: format!("delta-{file_id:06}.txt"),
                })
                .unwrap();
        }
        delta.sync().unwrap();
        drop(delta);

        compact_index(&path).unwrap();
        let mut store = SearchStore::open(&path).unwrap();
        assert_eq!(store.record_count(), changes + 1);
        assert_eq!(store.search_exact("delta-000002.txt", 1).unwrap().len(), 1);
        let last_name = format!("delta-{:06}.txt", changes + 1);
        assert_eq!(store.search_exact(&last_name, 1).unwrap().len(), 1);
        assert!(!delta_path(&path).exists());

        cleanup_family(&path);
    }

    #[test]
    fn external_compaction_keeps_latest_delta_across_chunks() {
        let path = temp();
        build_family(
            &path,
            &[(1, 0, "root"), (2, 1, "base.txt"), (3, 1, "keep.txt")],
        );

        let mut delta = DeltaWriter::open(delta_path(&path)).unwrap();
        for record in [
            DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 2,
                parent_id: 1,
                size_bytes: 1,
                flags: 0,
                name: "early.txt".into(),
            },
            DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 4,
                parent_id: 1,
                size_bytes: 2,
                flags: 0,
                name: "temporary.txt".into(),
            },
            DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 2,
                parent_id: 1,
                size_bytes: 3,
                flags: 0,
                name: "latest.txt".into(),
            },
            DeltaRecord {
                op: DeltaOp::Delete,
                file_id: 4,
                parent_id: 1,
                size_bytes: 0,
                flags: 0,
                name: String::new(),
            },
            DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 5,
                parent_id: 1,
                size_bytes: 5,
                flags: 0,
                name: "new.txt".into(),
            },
        ] {
            delta.append(&record).unwrap();
        }
        delta.sync().unwrap();
        drop(delta);

        // Tiny chunk limits force duplicate file IDs into separate temp chunks.
        compact_index_impl(&path, 2, 1024).unwrap();

        let mut store = SearchStore::open(&path).unwrap();
        assert!(store.search_exact("base.txt", 4).unwrap().is_empty());
        assert!(store.search_exact("early.txt", 4).unwrap().is_empty());
        assert_eq!(store.search_exact("latest.txt", 4).unwrap().len(), 1);
        assert!(store.search_exact("temporary.txt", 4).unwrap().is_empty());
        assert_eq!(store.search_exact("new.txt", 4).unwrap().len(), 1);
        assert_eq!(store.search_exact("keep.txt", 4).unwrap().len(), 1);
        assert!(!delta_path(&path).exists());

        cleanup_family(&path);
    }

    #[test]
    fn compaction_applies_upsert_and_delete() {
        let path = temp();
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (id, parent, name) in [(1, 0, "root"), (2, 1, "old.txt"), (3, 1, "gone.txt")] {
            builder
                .push(InputRecord {
                    file_id: id,
                    parent_id: parent,
                    size_bytes: 0,
                    flags: 0,
                    name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        let mut delta = DeltaWriter::open(delta_path(&path)).unwrap();
        delta
            .append(&crate::delta::DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 2,
                parent_id: 1,
                size_bytes: 7,
                flags: 0,
                name: "new.txt".into(),
            })
            .unwrap();
        delta
            .append(&DeltaRecord {
                op: DeltaOp::Delete,
                file_id: 3,
                parent_id: 1,
                size_bytes: 0,
                flags: 0,
                name: String::new(),
            })
            .unwrap();
        delta.sync().unwrap();
        drop(delta);

        compact_index(&path).unwrap();
        let mut store = SearchStore::open(&path).unwrap();
        assert!(store.search_exact("new.txt", 5).unwrap().len() == 1);
        assert!(store.search_exact("gone.txt", 5).unwrap().is_empty());
        assert!(!delta_path(&path).exists());
        for file in family(&path) {
            let _ = fs::remove_file(file);
        }
    }
}
