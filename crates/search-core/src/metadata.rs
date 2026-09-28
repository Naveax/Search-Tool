use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const SIZE_MAGIC: [u8; 8] = *b"STSIZE\0\0";
const VERSION: u16 = 1;
const ENTRY_SIZE: u16 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeEntry {
    pub size_bytes: u64,
    pub file_id: u64,
}

impl SizeEntry {
    fn write_to<W: Write>(self, out: &mut W) -> io::Result<()> {
        out.write_all(&self.size_bytes.to_le_bytes())?;
        out.write_all(&self.file_id.to_le_bytes())
    }

    fn read_from<R: Read>(input: &mut R) -> io::Result<Self> {
        Ok(Self {
            size_bytes: read_u64(input)?,
            file_id: read_u64(input)?,
        })
    }
}

#[derive(Debug)]
pub struct SizeIndexBuilder {
    final_path: PathBuf,
    chunk_limit: usize,
    entries: Vec<SizeEntry>,
    chunks: Vec<PathBuf>,
    total: u64,
}

impl SizeIndexBuilder {
    pub fn create(path: impl AsRef<Path>, chunk_entries: usize) -> io::Result<Self> {
        let final_path = path.as_ref().to_path_buf();
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self {
            final_path,
            chunk_limit: chunk_entries.max(1024),
            entries: Vec::with_capacity(chunk_entries.clamp(1024, 131_072)),
            chunks: Vec::new(),
            total: 0,
        })
    }

    pub fn push(&mut self, entry: SizeEntry) -> io::Result<()> {
        self.entries.push(entry);
        self.total = self.total.saturating_add(1);
        if self.entries.len() >= self.chunk_limit {
            self.flush_chunk()?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<u64> {
        self.flush_chunk()?;
        let staging = suffix(&self.final_path, ".tmp");
        let mut out = BufWriter::with_capacity(256 * 1024, File::create(&staging)?);
        write_header(&mut out, self.total)?;
        merge_chunks(&self.chunks, &mut out)?;
        out.flush()?;
        out.get_ref().sync_all()?;
        if self.final_path.exists() {
            fs::remove_file(&self.final_path)?;
        }
        fs::rename(&staging, &self.final_path)?;
        for chunk in self.chunks {
            let _ = fs::remove_file(chunk);
        }
        Ok(self.total)
    }

    fn flush_chunk(&mut self) -> io::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        self.entries
            .sort_unstable_by_key(|entry| (entry.size_bytes, entry.file_id));
        let path = suffix(&self.final_path, &format!(".chunk.{}", self.chunks.len()));
        let mut out = BufWriter::with_capacity(256 * 1024, File::create(&path)?);
        for entry in self.entries.drain(..) {
            entry.write_to(&mut out)?;
        }
        out.flush()?;
        self.chunks.push(path);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroupScanStats {
    pub candidate_groups: u64,
    pub candidate_files: u64,
    pub oversized_groups_skipped: u64,
}

#[derive(Debug)]
pub struct SizeIndex {
    input: BufReader<File>,
    count: u64,
}

impl SizeIndex {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut input = BufReader::with_capacity(256 * 1024, File::open(path)?);
        let count = read_header(&mut input)?;
        Ok(Self { input, count })
    }

    pub const fn count(&self) -> u64 {
        self.count
    }

    pub fn next_entry(&mut self) -> io::Result<Option<SizeEntry>> {
        read_optional_entry(&mut self.input)
    }

    pub fn for_each_candidate_group<F>(
        &mut self,
        min_size: u64,
        max_group_entries: usize,
        mut callback: F,
    ) -> io::Result<GroupScanStats>
    where
        F: FnMut(u64, &[u64]) -> io::Result<()>,
    {
        let max_group_entries = max_group_entries.max(2);
        let mut stats = GroupScanStats::default();
        let mut current_size = None;
        let mut group = Vec::new();
        let mut overflow = false;

        for _ in 0..self.count {
            let entry = SizeEntry::read_from(&mut self.input)?;
            if current_size != Some(entry.size_bytes) {
                flush_group(
                    current_size,
                    &mut group,
                    overflow,
                    min_size,
                    &mut stats,
                    &mut callback,
                )?;
                current_size = Some(entry.size_bytes);
                overflow = false;
            }
            if group.len() < max_group_entries {
                group.push(entry.file_id);
            } else {
                overflow = true;
            }
        }
        flush_group(
            current_size,
            &mut group,
            overflow,
            min_size,
            &mut stats,
            &mut callback,
        )?;
        Ok(stats)
    }
}

/// Streams multiple sorted size indexes as one logical sequence and emits only
/// candidate groups that contain at least two files with the same size.
///
/// The callback receives `(source_index, file_id)` pairs so file IDs remain
/// scoped to their volume/source. Memory is bounded by `max_group_entries` plus
/// one heap item per source. Oversized groups are skipped instead of growing
/// without bound.
pub fn for_each_merged_candidate_group<F>(
    indexes: &mut [SizeIndex],
    min_size: u64,
    max_group_entries: usize,
    mut callback: F,
) -> io::Result<GroupScanStats>
where
    F: FnMut(u64, &[(usize, u64)]) -> io::Result<()>,
{
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    struct MergedHeapItem {
        entry: SizeEntry,
        source: usize,
    }

    impl Ord for MergedHeapItem {
        fn cmp(&self, other: &Self) -> Ordering {
            (other.entry.size_bytes, other.source, other.entry.file_id).cmp(&(
                self.entry.size_bytes,
                self.source,
                self.entry.file_id,
            ))
        }
    }

    impl PartialOrd for MergedHeapItem {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let max_group_entries = max_group_entries.max(2);
    let mut heap = BinaryHeap::with_capacity(indexes.len());
    for (source, index) in indexes.iter_mut().enumerate() {
        if let Some(entry) = index.next_entry()? {
            heap.push(MergedHeapItem { entry, source });
        }
    }

    let mut stats = GroupScanStats::default();
    while let Some(item) = heap.pop() {
        let size = item.entry.size_bytes;
        let mut group = Vec::with_capacity(max_group_entries.min(64));
        let mut overflow = false;

        if size >= min_size {
            group.push((item.source, item.entry.file_id));
        }
        if let Some(next) = indexes[item.source].next_entry()? {
            heap.push(MergedHeapItem {
                entry: next,
                source: item.source,
            });
        }

        while heap
            .peek()
            .is_some_and(|candidate| candidate.entry.size_bytes == size)
        {
            let candidate = heap.pop().expect("peeked heap item must exist");
            if size >= min_size {
                if group.len() < max_group_entries {
                    group.push((candidate.source, candidate.entry.file_id));
                } else {
                    overflow = true;
                }
            }
            if let Some(next) = indexes[candidate.source].next_entry()? {
                heap.push(MergedHeapItem {
                    entry: next,
                    source: candidate.source,
                });
            }
        }

        if size < min_size || group.len() < 2 {
            continue;
        }
        if overflow {
            stats.oversized_groups_skipped = stats.oversized_groups_skipped.saturating_add(1);
            continue;
        }
        stats.candidate_groups = stats.candidate_groups.saturating_add(1);
        stats.candidate_files = stats.candidate_files.saturating_add(group.len() as u64);
        callback(size, &group)?;
    }

    Ok(stats)
}

fn flush_group<F>(
    size: Option<u64>,
    group: &mut Vec<u64>,
    overflow: bool,
    min_size: u64,
    stats: &mut GroupScanStats,
    callback: &mut F,
) -> io::Result<()>
where
    F: FnMut(u64, &[u64]) -> io::Result<()>,
{
    let Some(size) = size else {
        return Ok(());
    };
    if overflow {
        stats.oversized_groups_skipped = stats.oversized_groups_skipped.saturating_add(1);
    } else if size >= min_size && group.len() >= 2 {
        stats.candidate_groups = stats.candidate_groups.saturating_add(1);
        stats.candidate_files = stats.candidate_files.saturating_add(group.len() as u64);
        callback(size, group)?;
    }
    group.clear();
    Ok(())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct HeapItem {
    entry: SizeEntry,
    source: usize,
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        (other.entry.size_bytes, other.entry.file_id, other.source).cmp(&(
            self.entry.size_bytes,
            self.entry.file_id,
            self.source,
        ))
    }
}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn merge_chunks<W: Write>(chunks: &[PathBuf], out: &mut W) -> io::Result<()> {
    let mut readers: Vec<_> = chunks
        .iter()
        .map(|path| File::open(path).map(BufReader::new))
        .collect::<io::Result<_>>()?;
    let mut heap = BinaryHeap::new();
    for (source, reader) in readers.iter_mut().enumerate() {
        if let Some(entry) = read_optional_entry(reader)? {
            heap.push(HeapItem { entry, source });
        }
    }
    while let Some(item) = heap.pop() {
        item.entry.write_to(out)?;
        if let Some(entry) = read_optional_entry(&mut readers[item.source])? {
            heap.push(HeapItem {
                entry,
                source: item.source,
            });
        }
    }
    Ok(())
}

fn read_optional_entry<R: Read>(input: &mut R) -> io::Result<Option<SizeEntry>> {
    let mut first = [0_u8; 1];
    match input.read_exact(&mut first) {
        Ok(()) => {
            let mut rest = [0_u8; ENTRY_SIZE as usize - 1];
            input.read_exact(&mut rest)?;
            let mut bytes = [0_u8; ENTRY_SIZE as usize];
            bytes[0] = first[0];
            bytes[1..].copy_from_slice(&rest);
            SizeEntry::read_from(&mut &bytes[..]).map(Some)
        }
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn size_index_path(index_path: impl AsRef<Path>) -> PathBuf {
    suffix(index_path.as_ref(), ".sizes")
}

fn write_header<W: Write>(out: &mut W, count: u64) -> io::Result<()> {
    out.write_all(&SIZE_MAGIC)?;
    out.write_all(&VERSION.to_le_bytes())?;
    out.write_all(&ENTRY_SIZE.to_le_bytes())?;
    out.write_all(&count.to_le_bytes())?;
    out.write_all(&[0_u8; 12])
}

fn read_header<R: Read>(input: &mut R) -> io::Result<u64> {
    let mut magic = [0_u8; 8];
    input.read_exact(&mut magic)?;
    if magic != SIZE_MAGIC {
        return Err(invalid("invalid size-index magic"));
    }
    if read_u16(input)? != VERSION || read_u16(input)? != ENTRY_SIZE {
        return Err(invalid("unsupported size-index format"));
    }
    let count = read_u64(input)?;
    let mut reserved = [0_u8; 12];
    input.read_exact(&mut reserved)?;
    Ok(count)
}

fn suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn read_u16<R: Read>(input: &mut R) -> io::Result<u16> {
    let mut bytes = [0_u8; 2];
    input.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}
fn read_u64<R: Read>(input: &mut R) -> io::Result<u64> {
    let mut bytes = [0_u8; 8];
    input.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn external_size_index_finds_only_duplicate_size_groups() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("search-tool-size-{n}.idx"));
        let mut builder = SizeIndexBuilder::create(&path, 2).unwrap();
        for entry in [
            SizeEntry {
                size_bytes: 9,
                file_id: 1,
            },
            SizeEntry {
                size_bytes: 5,
                file_id: 2,
            },
            SizeEntry {
                size_bytes: 9,
                file_id: 3,
            },
            SizeEntry {
                size_bytes: 7,
                file_id: 4,
            },
        ] {
            builder.push(entry).unwrap();
        }
        builder.finish().unwrap();
        let mut index = SizeIndex::open(&path).unwrap();
        let mut groups = Vec::new();
        let stats = index
            .for_each_candidate_group(1, 16, |size, ids| {
                groups.push((size, ids.to_vec()));
                Ok(())
            })
            .unwrap();
        assert_eq!(groups, vec![(9, vec![1, 3])]);
        assert_eq!(stats.candidate_groups, 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn merged_size_indexes_find_cross_source_groups_without_id_collisions() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let a = std::env::temp_dir().join(format!("search-tool-size-merge-a-{n}.idx"));
        let b = std::env::temp_dir().join(format!("search-tool-size-merge-b-{n}.idx"));

        let mut a_builder = SizeIndexBuilder::create(&a, 2).unwrap();
        for entry in [
            SizeEntry {
                size_bytes: 5,
                file_id: 10,
            },
            SizeEntry {
                size_bytes: 9,
                file_id: 42,
            },
            SizeEntry {
                size_bytes: 20,
                file_id: 99,
            },
        ] {
            a_builder.push(entry).unwrap();
        }
        a_builder.finish().unwrap();

        let mut b_builder = SizeIndexBuilder::create(&b, 2).unwrap();
        for entry in [
            SizeEntry {
                size_bytes: 7,
                file_id: 11,
            },
            SizeEntry {
                size_bytes: 9,
                file_id: 42,
            },
            SizeEntry {
                size_bytes: 20,
                file_id: 100,
            },
        ] {
            b_builder.push(entry).unwrap();
        }
        b_builder.finish().unwrap();

        let mut indexes = vec![SizeIndex::open(&a).unwrap(), SizeIndex::open(&b).unwrap()];
        let mut groups = Vec::new();
        let stats = for_each_merged_candidate_group(&mut indexes, 8, 16, |size, entries| {
            groups.push((size, entries.to_vec()));
            Ok(())
        })
        .unwrap();

        assert_eq!(
            groups,
            vec![(9, vec![(0, 42), (1, 42)]), (20, vec![(0, 99), (1, 100)]),]
        );
        assert_eq!(stats.candidate_groups, 2);
        assert_eq!(stats.candidate_files, 4);
        assert_eq!(stats.oversized_groups_skipped, 0);

        let _ = fs::remove_file(a);
        let _ = fs::remove_file(b);
    }

    #[test]
    fn merged_size_indexes_bound_oversized_groups() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let a = std::env::temp_dir().join(format!("search-tool-size-overflow-a-{n}.idx"));
        let b = std::env::temp_dir().join(format!("search-tool-size-overflow-b-{n}.idx"));

        let mut a_builder = SizeIndexBuilder::create(&a, 2).unwrap();
        let mut b_builder = SizeIndexBuilder::create(&b, 2).unwrap();
        for file_id in 0..3 {
            a_builder
                .push(SizeEntry {
                    size_bytes: 100,
                    file_id,
                })
                .unwrap();
        }
        for file_id in 3..6 {
            b_builder
                .push(SizeEntry {
                    size_bytes: 100,
                    file_id,
                })
                .unwrap();
        }
        a_builder.finish().unwrap();
        b_builder.finish().unwrap();

        let mut indexes = vec![SizeIndex::open(&a).unwrap(), SizeIndex::open(&b).unwrap()];
        let mut called = false;
        let stats = for_each_merged_candidate_group(&mut indexes, 1, 4, |_size, _entries| {
            called = true;
            Ok(())
        })
        .unwrap();

        assert!(!called);
        assert_eq!(stats.candidate_groups, 0);
        assert_eq!(stats.oversized_groups_skipped, 1);

        let _ = fs::remove_file(a);
        let _ = fs::remove_file(b);
    }
}
