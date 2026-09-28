use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: [u8; 8] = *b"STATTR\0\0";
const CHECKPOINT_MAGIC: [u8; 8] = *b"STATCP\0\0";
const VERSION: u16 = 1;
const ENTRY_SIZE: u16 = 32;
const HEADER_SIZE: u64 = 32;
const CHECKPOINT_ENTRY_SIZE: u16 = 16;
const CHECKPOINT_STRIDE: u64 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributeEntry {
    pub file_id: u64,
    pub size_bytes: u64,
    pub modified_unix_secs: i64,
    pub platform_attributes: u32,
}

impl AttributeEntry {
    fn write_to<W: Write>(self, out: &mut W) -> io::Result<()> {
        out.write_all(&self.file_id.to_le_bytes())?;
        out.write_all(&self.size_bytes.to_le_bytes())?;
        out.write_all(&self.modified_unix_secs.to_le_bytes())?;
        out.write_all(&self.platform_attributes.to_le_bytes())?;
        out.write_all(&0_u32.to_le_bytes())
    }

    fn read_from<R: Read>(input: &mut R) -> io::Result<Self> {
        let file_id = read_u64(input)?;
        let size_bytes = read_u64(input)?;
        let modified_unix_secs = read_i64(input)?;
        let platform_attributes = read_u32(input)?;
        let _ = read_u32(input)?;
        Ok(Self {
            file_id,
            size_bytes,
            modified_unix_secs,
            platform_attributes,
        })
    }
}

#[derive(Debug)]
pub struct AttributeIndexBuilder {
    final_path: PathBuf,
    chunk_limit: usize,
    entries: Vec<AttributeEntry>,
    chunks: Vec<PathBuf>,
    total: u64,
}

impl AttributeIndexBuilder {
    pub fn create(path: impl AsRef<Path>, chunk_entries: usize) -> io::Result<Self> {
        let final_path = path.as_ref().to_path_buf();
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let chunk_limit = chunk_entries.max(1024);
        Ok(Self {
            final_path,
            chunk_limit,
            entries: Vec::with_capacity(chunk_limit.min(131_072)),
            chunks: Vec::new(),
            total: 0,
        })
    }

    pub fn push(&mut self, entry: AttributeEntry) -> io::Result<()> {
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
        let checkpoint_final = attribute_checkpoints_path(&self.final_path);
        let checkpoint_staging = suffix(&checkpoint_final, ".tmp");
        let mut out = BufWriter::with_capacity(256 * 1024, File::create(&staging)?);
        let mut checkpoints =
            BufWriter::with_capacity(64 * 1024, File::create(&checkpoint_staging)?);
        write_header(&mut out, MAGIC, ENTRY_SIZE, self.total)?;
        let checkpoint_count = if self.total == 0 {
            0
        } else {
            (self.total - 1) / CHECKPOINT_STRIDE + 1
        };
        write_header(
            &mut checkpoints,
            CHECKPOINT_MAGIC,
            CHECKPOINT_ENTRY_SIZE,
            checkpoint_count,
        )?;
        checkpoints.write_all(&self.total.to_le_bytes())?;
        merge_chunks(&self.chunks, &mut out, &mut checkpoints)?;
        out.flush()?;
        out.get_ref().sync_all()?;
        checkpoints.flush()?;
        checkpoints.get_ref().sync_all()?;
        atomic_replace(&staging, &self.final_path)?;
        atomic_replace(&checkpoint_staging, &checkpoint_final)?;
        for chunk in self.chunks {
            let _ = fs::remove_file(chunk);
        }
        Ok(self.total)
    }

    fn flush_chunk(&mut self) -> io::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        self.entries.sort_unstable_by_key(|entry| entry.file_id);
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

#[derive(Debug, Clone, Copy)]
struct Checkpoint {
    file_id: u64,
    entry_index: u64,
}

#[derive(Debug)]
pub struct AttributeIndex {
    input: File,
    count: u64,
    checkpoints: Vec<Checkpoint>,
}

impl AttributeIndex {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let mut input = File::open(path)?;
        let count = read_header(&mut input, MAGIC, ENTRY_SIZE)?;
        let expected = HEADER_SIZE.saturating_add(count.saturating_mul(ENTRY_SIZE as u64));
        if input.metadata()?.len() != expected {
            return Err(invalid("attribute index length mismatch"));
        }
        let checkpoint_path = attribute_checkpoints_path(path);
        let mut cp = BufReader::with_capacity(64 * 1024, File::open(checkpoint_path)?);
        let checkpoint_count = read_header(&mut cp, CHECKPOINT_MAGIC, CHECKPOINT_ENTRY_SIZE)?;
        let source_count = read_u64(&mut cp)?;
        if source_count != count {
            return Err(invalid("attribute checkpoint source count mismatch"));
        }
        let mut checkpoints = Vec::with_capacity(checkpoint_count as usize);
        for _ in 0..checkpoint_count {
            checkpoints.push(Checkpoint {
                file_id: read_u64(&mut cp)?,
                entry_index: read_u64(&mut cp)?,
            });
        }
        Ok(Self {
            input,
            count,
            checkpoints,
        })
    }

    pub const fn count(&self) -> u64 {
        self.count
    }

    pub fn memory_hint_bytes(&self) -> usize {
        self.checkpoints.len() * std::mem::size_of::<Checkpoint>()
    }

    pub fn get(&mut self, file_id: u64) -> io::Result<Option<AttributeEntry>> {
        if self.count == 0 {
            return Ok(None);
        }
        let start = match self
            .checkpoints
            .binary_search_by_key(&file_id, |checkpoint| checkpoint.file_id)
        {
            Ok(index) => self.checkpoints[index].entry_index,
            Err(0) => 0,
            Err(index) => self.checkpoints[index - 1].entry_index,
        };
        self.input.seek(SeekFrom::Start(
            HEADER_SIZE + start.saturating_mul(ENTRY_SIZE as u64),
        ))?;
        let end = self.count.min(start.saturating_add(CHECKPOINT_STRIDE + 1));
        for _ in start..end {
            let entry = AttributeEntry::read_from(&mut self.input)?;
            match entry.file_id.cmp(&file_id) {
                Ordering::Equal => return Ok(Some(entry)),
                Ordering::Greater => return Ok(None),
                Ordering::Less => {}
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct HeapItem {
    entry: AttributeEntry,
    source: usize,
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        (other.entry.file_id, other.source).cmp(&(self.entry.file_id, self.source))
    }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn merge_chunks<W: Write, C: Write>(
    chunks: &[PathBuf],
    out: &mut W,
    checkpoints: &mut C,
) -> io::Result<()> {
    let mut readers: Vec<_> = chunks
        .iter()
        .map(|path| File::open(path).map(BufReader::new))
        .collect::<io::Result<_>>()?;
    let mut heap = BinaryHeap::new();
    for (source, reader) in readers.iter_mut().enumerate() {
        if let Some(entry) = read_optional(reader)? {
            heap.push(HeapItem { entry, source });
        }
    }
    let mut index = 0_u64;
    while let Some(item) = heap.pop() {
        item.entry.write_to(out)?;
        if index.is_multiple_of(CHECKPOINT_STRIDE) {
            checkpoints.write_all(&item.entry.file_id.to_le_bytes())?;
            checkpoints.write_all(&index.to_le_bytes())?;
        }
        index = index.saturating_add(1);
        if let Some(entry) = read_optional(&mut readers[item.source])? {
            heap.push(HeapItem {
                entry,
                source: item.source,
            });
        }
    }
    Ok(())
}

fn read_optional<R: Read>(input: &mut R) -> io::Result<Option<AttributeEntry>> {
    let mut first = [0_u8; 1];
    match input.read_exact(&mut first) {
        Ok(()) => {
            let mut rest = [0_u8; ENTRY_SIZE as usize - 1];
            input.read_exact(&mut rest)?;
            let mut bytes = [0_u8; ENTRY_SIZE as usize];
            bytes[0] = first[0];
            bytes[1..].copy_from_slice(&rest);
            AttributeEntry::read_from(&mut &bytes[..]).map(Some)
        }
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn attribute_index_path(index_path: impl AsRef<Path>) -> PathBuf {
    suffix(index_path.as_ref(), ".attrs")
}

pub fn attribute_checkpoints_path(attribute_path: impl AsRef<Path>) -> PathBuf {
    suffix(attribute_path.as_ref(), ".cp")
}

fn write_header<W: Write>(
    out: &mut W,
    magic: [u8; 8],
    entry_size: u16,
    count: u64,
) -> io::Result<()> {
    out.write_all(&magic)?;
    out.write_all(&VERSION.to_le_bytes())?;
    out.write_all(&entry_size.to_le_bytes())?;
    out.write_all(&count.to_le_bytes())?;
    out.write_all(&[0_u8; 12])
}

fn read_header<R: Read>(input: &mut R, magic: [u8; 8], entry_size: u16) -> io::Result<u64> {
    let mut found = [0_u8; 8];
    input.read_exact(&mut found)?;
    if found != magic || read_u16(input)? != VERSION || read_u16(input)? != entry_size {
        return Err(invalid("unsupported attribute sidecar format"));
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

fn atomic_replace(staging: &Path, final_path: &Path) -> io::Result<()> {
    if final_path.exists() {
        fs::remove_file(final_path)?;
    }
    fs::rename(staging, final_path)
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
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
fn read_i64<R: Read>(input: &mut R) -> io::Result<i64> {
    let mut bytes = [0_u8; 8];
    input.read_exact(&mut bytes)?;
    Ok(i64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn attribute_index_is_bounded_and_lookup_is_exact() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("search-tool-attrs-{n}.idx"));
        let mut builder = AttributeIndexBuilder::create(&path, 1024).unwrap();
        for id in (1..=5000_u64).rev() {
            builder
                .push(AttributeEntry {
                    file_id: id,
                    size_bytes: id * 10,
                    modified_unix_secs: 1_700_000_000 + id as i64,
                    platform_attributes: 0,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        let mut index = AttributeIndex::open(&path).unwrap();
        assert_eq!(index.count(), 5000);
        assert!(index.memory_hint_bytes() < 4096);
        assert_eq!(index.get(42).unwrap().unwrap().size_bytes, 420);
        assert!(index.get(6000).unwrap().is_none());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(attribute_checkpoints_path(&path));
    }
}
