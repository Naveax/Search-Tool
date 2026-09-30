use crate::index::flags;
use crate::index_lock::IndexPublishGuard;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const STORE_MAGIC: [u8; 8] = *b"STIDX\0\0\0";
const STORE_VERSION: u16 = 1;
const STORE_HEADER_SIZE: u64 = 64;
const RECORD_SIZE: u16 = 40;

const NAME_MAGIC: [u8; 8] = *b"STNAME\0\0";
const NAME_ENTRY_SIZE: u16 = 48;
const NAME_HEADER_SIZE: u64 = 32;
const NAME_KEY_BYTES: usize = 32;

const ID_MAGIC: [u8; 8] = *b"STID\0\0\0\0";
const ID_ENTRY_SIZE: u16 = 16;
const ID_HEADER_SIZE: u64 = 32;

const NAME_CHECKPOINT_MAGIC: [u8; 8] = *b"STNCP\0\0\0";
const ID_CHECKPOINT_MAGIC: [u8; 8] = *b"STICP\0\0\0";
const NAME_CHECKPOINT_SIZE: u16 = 40;
const ID_CHECKPOINT_SIZE: u16 = 16;
const NAME_CHECKPOINT_STRIDE: u64 = 2048;
const ID_CHECKPOINT_STRIDE: u64 = 256;
const ID_LOOKUP_CACHE_LIMIT: usize = 256;

pub const FLAG_DIRECTORY: u16 = flags::DIRECTORY;
pub const FLAG_HIDDEN: u16 = flags::HIDDEN;
pub const FLAG_SYSTEM: u16 = flags::SYSTEM;
pub const FLAG_REPARSE_POINT: u16 = flags::REPARSE_POINT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreRecord {
    pub file_id: u64,
    pub parent_id: u64,
    pub size_bytes: u64,
    pub name_offset: u64,
    pub name_len: u32,
    pub flags: u16,
}

impl StoreRecord {
    pub const ENCODED_SIZE: usize = RECORD_SIZE as usize;

    fn write_to<W: Write>(&self, out: &mut W) -> io::Result<()> {
        out.write_all(&self.file_id.to_le_bytes())?;
        out.write_all(&self.parent_id.to_le_bytes())?;
        out.write_all(&self.size_bytes.to_le_bytes())?;
        out.write_all(&self.name_offset.to_le_bytes())?;
        out.write_all(&self.name_len.to_le_bytes())?;
        out.write_all(&self.flags.to_le_bytes())?;
        out.write_all(&0_u16.to_le_bytes())?;
        Ok(())
    }

    fn read_from<R: Read>(input: &mut R) -> io::Result<Self> {
        Ok(Self {
            file_id: read_u64(input)?,
            parent_id: read_u64(input)?,
            size_bytes: read_u64(input)?,
            name_offset: read_u64(input)?,
            name_len: read_u32(input)?,
            flags: read_u16(input)?,
            // reserved
        })
        .and_then(|record| {
            let _ = read_u16(input)?;
            Ok(record)
        })
    }

    pub const fn is_directory(self) -> bool {
        self.flags & FLAG_DIRECTORY != 0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InputRecord<'a> {
    pub file_id: u64,
    pub parent_id: u64,
    pub size_bytes: u64,
    pub flags: u16,
    pub name: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreStats {
    pub records: u64,
    pub string_pool_bytes: u64,
    pub index_bytes: u64,
    pub name_index_bytes: u64,
    pub id_index_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct BuildOptions {
    /// Maximum fixed-size sidecar entries retained before flushing one sorted chunk.
    /// 131_072 name entries are ~6 MiB before Vec overhead.
    pub sort_chunk_entries: usize,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            sort_chunk_entries: 131_072,
        }
    }
}

#[derive(Debug)]
pub struct IndexBuilder {
    final_path: PathBuf,
    records_path: PathBuf,
    strings_path: PathBuf,
    lock_path: PathBuf,
    lock_file: Option<File>,
    records: BufWriter<File>,
    strings: BufWriter<File>,
    record_count: u64,
    string_pool_bytes: u64,
    names: ChunkedNameSorter,
    ids: ChunkedIdSorter,
    finished: bool,
}

impl IndexBuilder {
    pub fn create(path: impl AsRef<Path>, options: BuildOptions) -> io::Result<Self> {
        let final_path = path.as_ref().to_path_buf();
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let records_path = temp_path(&final_path, "records.tmp");
        let strings_path = temp_path(&final_path, "strings.tmp");
        let lock_path = temp_path(&final_path, "lock");

        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;
        lock_file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => {
                io::Error::new(io::ErrorKind::WouldBlock, "index build already in progress")
            }
            std::fs::TryLockError::Error(error) => error,
        })?;

        // Lock before removing stale temp files. A concurrent builder must never
        // be able to delete the active builder's records/string/chunk staging.
        cleanup_temp_family(&final_path)?;

        let records = BufWriter::with_capacity(256 * 1024, File::create(&records_path)?);
        let strings = BufWriter::with_capacity(256 * 1024, File::create(&strings_path)?);
        let chunk_entries = options.sort_chunk_entries.max(1024);

        Ok(Self {
            final_path: final_path.clone(),
            records_path,
            strings_path,
            lock_path,
            lock_file: Some(lock_file),
            records,
            strings,
            record_count: 0,
            string_pool_bytes: 0,
            names: ChunkedNameSorter::new(final_path.clone(), chunk_entries),
            ids: ChunkedIdSorter::new(final_path, chunk_entries),
            finished: false,
        })
    }

    pub fn push(&mut self, input: InputRecord<'_>) -> io::Result<()> {
        let name = input.name.as_bytes();
        if name.len() > u32::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "filename too large",
            ));
        }

        let record_index = self.record_count;
        let record = StoreRecord {
            file_id: input.file_id,
            parent_id: input.parent_id,
            size_bytes: input.size_bytes,
            name_offset: self.string_pool_bytes,
            name_len: name.len() as u32,
            flags: input.flags,
        };
        record.write_to(&mut self.records)?;
        self.strings.write_all(name)?;

        self.names
            .push(NameEntry::from_name(input.name, record_index))?;
        self.ids.push(IdEntry {
            file_id: input.file_id,
            record_index,
        })?;

        self.record_count = self.record_count.saturating_add(1);
        self.string_pool_bytes = self.string_pool_bytes.saturating_add(name.len() as u64);
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<StoreStats> {
        self.records.flush()?;
        self.records.get_ref().sync_all()?;
        self.strings.flush()?;
        self.strings.get_ref().sync_all()?;

        let staging = temp_path(&self.final_path, "build.tmp");
        {
            let mut out = BufWriter::with_capacity(512 * 1024, File::create(&staging)?);
            write_store_header(
                &mut out,
                self.record_count,
                STORE_HEADER_SIZE + self.record_count * RECORD_SIZE as u64,
                self.string_pool_bytes,
            )?;
            copy_file(&self.records_path, &mut out)?;
            copy_file(&self.strings_path, &mut out)?;
            out.flush()?;
            out.get_ref().sync_all()?;
        }

        let names_final = names_path(&self.final_path);
        let ids_final = ids_path(&self.final_path);
        self.names.finish(&names_final, self.record_count)?;
        self.ids.finish(&ids_final, self.record_count)?;
        write_checkpoint_sidecars(&names_final, &ids_final, self.record_count)?;

        atomic_replace(&staging, &self.final_path)?;
        sync_parent(&self.final_path)?;

        let stats = StoreStats {
            records: self.record_count,
            string_pool_bytes: self.string_pool_bytes,
            index_bytes: file_len(&self.final_path)?,
            name_index_bytes: file_len(&names_final)?,
            id_index_bytes: file_len(&ids_final)?,
        };

        self.finished = true;
        let _ = fs::remove_file(&self.records_path);
        let _ = fs::remove_file(&self.strings_path);
        self.lock_file.take();
        let _ = fs::remove_file(&self.lock_path);
        Ok(stats)
    }
}

impl Drop for IndexBuilder {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.records.flush();
            let _ = self.strings.flush();
        }
        self.lock_file.take();
        let _ = fs::remove_file(&self.lock_path);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StoreHeader {
    record_count: u64,
    string_pool_offset: u64,
    string_pool_bytes: u64,
}

fn write_store_header<W: Write>(
    out: &mut W,
    record_count: u64,
    string_pool_offset: u64,
    string_pool_bytes: u64,
) -> io::Result<()> {
    out.write_all(&STORE_MAGIC)?;
    out.write_all(&STORE_VERSION.to_le_bytes())?;
    out.write_all(&RECORD_SIZE.to_le_bytes())?;
    out.write_all(&(STORE_HEADER_SIZE as u16).to_le_bytes())?;
    out.write_all(&0_u16.to_le_bytes())?;
    out.write_all(&record_count.to_le_bytes())?;
    out.write_all(&string_pool_offset.to_le_bytes())?;
    out.write_all(&string_pool_bytes.to_le_bytes())?;
    out.write_all(&[0_u8; 24])?;
    Ok(())
}

fn read_store_header<R: Read>(input: &mut R) -> io::Result<StoreHeader> {
    let mut magic = [0_u8; 8];
    input.read_exact(&mut magic)?;
    if magic != STORE_MAGIC {
        return Err(invalid_data("invalid Search Tool index magic"));
    }
    if read_u16(input)? != STORE_VERSION {
        return Err(invalid_data("unsupported Search Tool index version"));
    }
    if read_u16(input)? != RECORD_SIZE {
        return Err(invalid_data("unexpected Search Tool record size"));
    }
    if read_u16(input)? as u64 != STORE_HEADER_SIZE {
        return Err(invalid_data("unexpected Search Tool header size"));
    }
    let _ = read_u16(input)?;
    let record_count = read_u64(input)?;
    let string_pool_offset = read_u64(input)?;
    let string_pool_bytes = read_u64(input)?;
    let mut reserved = [0_u8; 24];
    input.read_exact(&mut reserved)?;
    Ok(StoreHeader {
        record_count,
        string_pool_offset,
        string_pool_bytes,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NameEntry {
    key: [u8; NAME_KEY_BYTES],
    key_len: u8,
    record_index: u64,
}

impl NameEntry {
    fn from_name(name: &str, record_index: u64) -> Self {
        let normalized = normalize_name(name);
        let bytes = normalized.as_bytes();
        let mut key = [0_u8; NAME_KEY_BYTES];
        let len = bytes.len().min(NAME_KEY_BYTES);
        key[..len].copy_from_slice(&bytes[..len]);
        Self {
            key,
            key_len: len as u8,
            record_index,
        }
    }

    fn cmp_key(&self, other: &Self) -> Ordering {
        self.key
            .cmp(&other.key)
            .then(self.key_len.cmp(&other.key_len))
            .then(self.record_index.cmp(&other.record_index))
    }

    fn write_to<W: Write>(&self, out: &mut W) -> io::Result<()> {
        out.write_all(&self.key)?;
        out.write_all(&[self.key_len])?;
        out.write_all(&[0_u8; 7])?;
        out.write_all(&self.record_index.to_le_bytes())?;
        Ok(())
    }

    fn read_from<R: Read>(input: &mut R) -> io::Result<Self> {
        let mut key = [0_u8; NAME_KEY_BYTES];
        input.read_exact(&mut key)?;
        let mut len = [0_u8; 1];
        input.read_exact(&mut len)?;
        let mut reserved = [0_u8; 7];
        input.read_exact(&mut reserved)?;
        let record_index = read_u64(input)?;
        Ok(Self {
            key,
            key_len: len[0],
            record_index,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IdEntry {
    file_id: u64,
    record_index: u64,
}

impl IdEntry {
    fn write_to<W: Write>(&self, out: &mut W) -> io::Result<()> {
        out.write_all(&self.file_id.to_le_bytes())?;
        out.write_all(&self.record_index.to_le_bytes())?;
        Ok(())
    }

    fn read_from<R: Read>(input: &mut R) -> io::Result<Self> {
        Ok(Self {
            file_id: read_u64(input)?,
            record_index: read_u64(input)?,
        })
    }
}

#[derive(Debug)]
struct ChunkedNameSorter {
    base: PathBuf,
    max_entries: usize,
    buffer: Vec<NameEntry>,
    chunks: Vec<PathBuf>,
}

impl ChunkedNameSorter {
    fn new(base: PathBuf, max_entries: usize) -> Self {
        Self {
            base,
            max_entries,
            buffer: Vec::with_capacity(max_entries),
            chunks: Vec::new(),
        }
    }

    fn push(&mut self, entry: NameEntry) -> io::Result<()> {
        self.buffer.push(entry);
        if self.buffer.len() >= self.max_entries {
            self.flush_chunk()?;
        }
        Ok(())
    }

    fn flush_chunk(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.buffer.sort_unstable_by(NameEntry::cmp_key);
        let path = temp_path(&self.base, &format!("namechunk{}.tmp", self.chunks.len()));
        let mut out = BufWriter::with_capacity(256 * 1024, File::create(&path)?);
        for entry in &self.buffer {
            entry.write_to(&mut out)?;
        }
        out.flush()?;
        self.buffer.clear();
        self.chunks.push(path);
        Ok(())
    }

    fn finish(&mut self, final_path: &Path, count: u64) -> io::Result<()> {
        self.flush_chunk()?;
        let staging = temp_path(final_path, "build.tmp");
        let mut out = BufWriter::with_capacity(512 * 1024, File::create(&staging)?);
        write_sidecar_header(&mut out, NAME_MAGIC, NAME_ENTRY_SIZE, count)?;
        merge_name_chunks(&self.chunks, &mut out)?;
        out.flush()?;
        out.get_ref().sync_all()?;
        atomic_replace(&staging, final_path)?;
        for chunk in self.chunks.drain(..) {
            let _ = fs::remove_file(chunk);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ChunkedIdSorter {
    base: PathBuf,
    max_entries: usize,
    buffer: Vec<IdEntry>,
    chunks: Vec<PathBuf>,
}

impl ChunkedIdSorter {
    fn new(base: PathBuf, max_entries: usize) -> Self {
        Self {
            base,
            max_entries,
            buffer: Vec::with_capacity(max_entries),
            chunks: Vec::new(),
        }
    }

    fn push(&mut self, entry: IdEntry) -> io::Result<()> {
        self.buffer.push(entry);
        if self.buffer.len() >= self.max_entries {
            self.flush_chunk()?;
        }
        Ok(())
    }

    fn flush_chunk(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.buffer
            .sort_unstable_by_key(|entry| (entry.file_id, entry.record_index));
        let path = temp_path(&self.base, &format!("idchunk{}.tmp", self.chunks.len()));
        let mut out = BufWriter::with_capacity(256 * 1024, File::create(&path)?);
        for entry in &self.buffer {
            entry.write_to(&mut out)?;
        }
        out.flush()?;
        self.buffer.clear();
        self.chunks.push(path);
        Ok(())
    }

    fn finish(&mut self, final_path: &Path, count: u64) -> io::Result<()> {
        self.flush_chunk()?;
        let staging = temp_path(final_path, "build.tmp");
        let mut out = BufWriter::with_capacity(512 * 1024, File::create(&staging)?);
        write_sidecar_header(&mut out, ID_MAGIC, ID_ENTRY_SIZE, count)?;
        merge_id_chunks(&self.chunks, &mut out)?;
        out.flush()?;
        out.get_ref().sync_all()?;
        atomic_replace(&staging, final_path)?;
        for chunk in self.chunks.drain(..) {
            let _ = fs::remove_file(chunk);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct NameHeapItem {
    entry: NameEntry,
    source: usize,
}

impl Ord for NameHeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .entry
            .cmp_key(&self.entry)
            .then_with(|| other.source.cmp(&self.source))
    }
}

impl PartialOrd for NameHeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn merge_name_chunks<W: Write>(chunks: &[PathBuf], out: &mut W) -> io::Result<()> {
    let mut readers: Vec<_> = chunks
        .iter()
        .map(|path| File::open(path).map(BufReader::new))
        .collect::<io::Result<_>>()?;
    let mut heap = BinaryHeap::new();
    for (source, reader) in readers.iter_mut().enumerate() {
        if let Some(entry) = read_optional_name(reader)? {
            heap.push(NameHeapItem { entry, source });
        }
    }
    while let Some(item) = heap.pop() {
        item.entry.write_to(out)?;
        if let Some(entry) = read_optional_name(&mut readers[item.source])? {
            heap.push(NameHeapItem {
                entry,
                source: item.source,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct IdHeapItem {
    entry: IdEntry,
    source: usize,
}

impl Ord for IdHeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .entry
            .file_id
            .cmp(&self.entry.file_id)
            .then_with(|| other.entry.record_index.cmp(&self.entry.record_index))
            .then_with(|| other.source.cmp(&self.source))
    }
}

impl PartialOrd for IdHeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn merge_id_chunks<W: Write>(chunks: &[PathBuf], out: &mut W) -> io::Result<()> {
    let mut readers: Vec<_> = chunks
        .iter()
        .map(|path| File::open(path).map(BufReader::new))
        .collect::<io::Result<_>>()?;
    let mut heap = BinaryHeap::new();
    for (source, reader) in readers.iter_mut().enumerate() {
        if let Some(entry) = read_optional_id(reader)? {
            heap.push(IdHeapItem { entry, source });
        }
    }
    while let Some(item) = heap.pop() {
        item.entry.write_to(out)?;
        if let Some(entry) = read_optional_id(&mut readers[item.source])? {
            heap.push(IdHeapItem {
                entry,
                source: item.source,
            });
        }
    }
    Ok(())
}

fn read_optional_name<R: Read>(input: &mut R) -> io::Result<Option<NameEntry>> {
    let mut first = [0_u8; 1];
    match input.read_exact(&mut first) {
        Ok(()) => {
            let mut rest = [0_u8; NAME_ENTRY_SIZE as usize - 1];
            input.read_exact(&mut rest)?;
            let mut bytes = [0_u8; NAME_ENTRY_SIZE as usize];
            bytes[0] = first[0];
            bytes[1..].copy_from_slice(&rest);
            NameEntry::read_from(&mut &bytes[..]).map(Some)
        }
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error),
    }
}

fn read_optional_id<R: Read>(input: &mut R) -> io::Result<Option<IdEntry>> {
    let mut first = [0_u8; 1];
    match input.read_exact(&mut first) {
        Ok(()) => {
            let mut rest = [0_u8; ID_ENTRY_SIZE as usize - 1];
            input.read_exact(&mut rest)?;
            let mut bytes = [0_u8; ID_ENTRY_SIZE as usize];
            bytes[0] = first[0];
            bytes[1..].copy_from_slice(&rest);
            IdEntry::read_from(&mut &bytes[..]).map(Some)
        }
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error),
    }
}

fn write_sidecar_header<W: Write>(
    out: &mut W,
    magic: [u8; 8],
    entry_size: u16,
    count: u64,
) -> io::Result<()> {
    out.write_all(&magic)?;
    out.write_all(&STORE_VERSION.to_le_bytes())?;
    out.write_all(&entry_size.to_le_bytes())?;
    out.write_all(&count.to_le_bytes())?;
    out.write_all(&[0_u8; 12])?;
    Ok(())
}

fn validate_sidecar_header<R: Read>(
    input: &mut R,
    expected_magic: [u8; 8],
    expected_entry_size: u16,
) -> io::Result<u64> {
    let mut magic = [0_u8; 8];
    input.read_exact(&mut magic)?;
    if magic != expected_magic {
        return Err(invalid_data("invalid sidecar magic"));
    }
    if read_u16(input)? != STORE_VERSION {
        return Err(invalid_data("unsupported sidecar version"));
    }
    if read_u16(input)? != expected_entry_size {
        return Err(invalid_data("unexpected sidecar entry size"));
    }
    let count = read_u64(input)?;
    let mut reserved = [0_u8; 12];
    input.read_exact(&mut reserved)?;
    Ok(count)
}

#[derive(Debug, Clone, Copy)]
struct NameCheckpoint {
    key: [u8; NAME_KEY_BYTES],
    entry_index: u64,
}

#[derive(Debug, Clone, Copy)]
struct IdCheckpoint {
    file_id: u64,
    entry_index: u64,
}

#[derive(Debug)]
pub struct SearchStore {
    main: File,
    strings: File,
    names: File,
    ids: File,
    header: StoreHeader,
    name_count: u64,
    id_count: u64,
    name_checkpoints: Vec<NameCheckpoint>,
    id_checkpoints: Vec<IdCheckpoint>,
    id_lookup_cache: HashMap<u64, Option<u64>>,
}

impl SearchStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        // Every SearchStore consumer, not only the live-search overlay, must
        // observe one coherent base-family generation. Final compaction/recovery
        // publishes main + sidecars under the exclusive side of this short-lived
        // lock. Holding the shared side only while opening/validating the files
        // prevents doctor/metadata/content/bench readers from seeing a torn swap
        // without blocking the expensive compaction rebuild itself.
        let _snapshot = IndexPublishGuard::read(path)?;
        let mut main = File::open(path)?;
        let header = read_store_header(&mut main)?;
        let expected_offset = STORE_HEADER_SIZE + header.record_count * RECORD_SIZE as u64;
        if header.string_pool_offset != expected_offset {
            return Err(invalid_data("corrupt string-pool offset"));
        }
        let main_len = main.metadata()?.len();
        if header
            .string_pool_offset
            .saturating_add(header.string_pool_bytes)
            > main_len
        {
            return Err(invalid_data("string pool exceeds index file"));
        }

        let strings = File::open(path)?;
        let mut names = File::open(names_path(path))?;
        let name_count = validate_sidecar_header(&mut names, NAME_MAGIC, NAME_ENTRY_SIZE)?;
        let mut ids = File::open(ids_path(path))?;
        let id_count = validate_sidecar_header(&mut ids, ID_MAGIC, ID_ENTRY_SIZE)?;
        if name_count != header.record_count || id_count != header.record_count {
            return Err(invalid_data("sidecar count does not match main index"));
        }

        let name_checkpoints = load_name_checkpoints(path, &mut names, name_count)?;
        let id_checkpoints = load_id_checkpoints(path, &mut ids, id_count)?;

        Ok(Self {
            main,
            strings,
            names,
            ids,
            header,
            name_count,
            id_count,
            name_checkpoints,
            id_checkpoints,
            id_lookup_cache: HashMap::new(),
        })
    }

    pub const fn record_count(&self) -> u64 {
        self.header.record_count
    }

    pub fn memory_hint_bytes(&self) -> usize {
        self.name_checkpoints.len() * std::mem::size_of::<NameCheckpoint>()
            + self.id_checkpoints.len() * std::mem::size_of::<IdCheckpoint>()
            + self.id_lookup_cache.capacity() * std::mem::size_of::<(u64, Option<u64>)>()
    }

    pub(crate) fn for_each_id_entry(
        &mut self,
        mut visit: impl FnMut(u64, u64) -> io::Result<()>,
    ) -> io::Result<()> {
        self.ids.seek(SeekFrom::Start(ID_HEADER_SIZE))?;
        for _ in 0..self.id_count {
            let entry = IdEntry::read_from(&mut self.ids)?;
            visit(entry.file_id, entry.record_index)?;
        }
        Ok(())
    }

    pub fn read_record(&mut self, record_index: u64) -> io::Result<StoreRecord> {
        if record_index >= self.header.record_count {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "record index out of range",
            ));
        }
        let offset = STORE_HEADER_SIZE + record_index * RECORD_SIZE as u64;
        self.main.seek(SeekFrom::Start(offset))?;
        StoreRecord::read_from(&mut self.main)
    }

    pub fn read_name(&mut self, record: StoreRecord) -> io::Result<String> {
        let len = record.name_len as usize;
        let end = record.name_offset.saturating_add(record.name_len as u64);
        if end > self.header.string_pool_bytes {
            return Err(invalid_data("record filename exceeds string pool"));
        }
        let mut bytes = vec![0_u8; len];
        self.strings.seek(SeekFrom::Start(
            self.header.string_pool_offset + record.name_offset,
        ))?;
        self.strings.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map_err(|_| invalid_data("filename is not valid UTF-8"))
    }

    pub fn lookup_file_id(&mut self, file_id: u64) -> io::Result<Option<u64>> {
        if let Some(cached) = self.id_lookup_cache.get(&file_id).copied() {
            return Ok(cached);
        }
        let result = self.lookup_file_id_uncached(file_id)?;
        if self.id_lookup_cache.len() >= ID_LOOKUP_CACHE_LIMIT {
            self.id_lookup_cache.clear();
        }
        self.id_lookup_cache.insert(file_id, result);
        Ok(result)
    }

    fn lookup_file_id_uncached(&mut self, file_id: u64) -> io::Result<Option<u64>> {
        if self.id_count == 0 {
            return Ok(None);
        }
        let start = checkpoint_start_id(&self.id_checkpoints, file_id);
        self.ids.seek(SeekFrom::Start(
            ID_HEADER_SIZE + start * ID_ENTRY_SIZE as u64,
        ))?;
        let scan_limit = (start + 257).min(self.id_count);
        for _ in start..scan_limit {
            let entry = IdEntry::read_from(&mut self.ids)?;
            match entry.file_id.cmp(&file_id) {
                Ordering::Less => continue,
                Ordering::Equal => return Ok(Some(entry.record_index)),
                Ordering::Greater => return Ok(None),
            }
        }
        Ok(None)
    }

    pub fn get_by_file_id(&mut self, file_id: u64) -> io::Result<Option<SearchHit>> {
        let Some(record_index) = self.lookup_file_id(file_id)? else {
            return Ok(None);
        };
        let record = self.read_record(record_index)?;
        let name = self.read_name(record)?;
        Ok(Some(SearchHit {
            record_index,
            record,
            name,
        }))
    }

    pub fn reconstruct_path(&mut self, record_index: u64, max_depth: usize) -> io::Result<String> {
        let mut pieces = Vec::with_capacity(8);
        let mut current = record_index;
        let mut last_file_id = None;
        for _ in 0..max_depth.max(1) {
            let record = self.read_record(current)?;
            let name = self.read_name(record)?;
            if !name.is_empty() {
                pieces.push(name);
            }
            if record.parent_id == 0 || record.parent_id == record.file_id {
                break;
            }
            if last_file_id == Some(record.parent_id) {
                break;
            }
            last_file_id = Some(record.file_id);
            match self.lookup_file_id(record.parent_id)? {
                Some(parent_index) => current = parent_index,
                None => break,
            }
        }
        pieces.reverse();
        Ok(pieces.join("\\"))
    }

    pub fn search_prefix(&mut self, query: &str, limit: usize) -> io::Result<Vec<SearchHit>> {
        if query.is_empty() || limit == 0 || self.name_count == 0 {
            return Ok(Vec::new());
        }
        let normalized = normalize_name(query);
        let qbytes = normalized.as_bytes();
        let mut key = [0_u8; NAME_KEY_BYTES];
        let key_len = qbytes.len().min(NAME_KEY_BYTES);
        key[..key_len].copy_from_slice(&qbytes[..key_len]);

        let start = checkpoint_start_name(&self.name_checkpoints, &key);
        self.names.seek(SeekFrom::Start(
            NAME_HEADER_SIZE + start * NAME_ENTRY_SIZE as u64,
        ))?;

        let mut hits = Vec::with_capacity(limit.min(64));
        let mut index = start;
        while index < self.name_count && hits.len() < limit {
            let entry = NameEntry::read_from(&mut self.names)?;
            index += 1;
            if entry.key[..key_len] < key[..key_len] {
                continue;
            }
            if entry.key[..key_len] > key[..key_len] {
                break;
            }
            let record = self.read_record(entry.record_index)?;
            let name = self.read_name(record)?;
            if normalize_name(&name).starts_with(&normalized) {
                hits.push(SearchHit {
                    record_index: entry.record_index,
                    record,
                    name,
                });
            }
        }
        Ok(hits)
    }

    pub fn search_exact(&mut self, query: &str, limit: usize) -> io::Result<Vec<SearchHit>> {
        let normalized = normalize_name(query);
        let candidates = self.search_prefix(query, limit.saturating_mul(8).max(limit))?;
        Ok(candidates
            .into_iter()
            .filter(|hit| normalize_name(&hit.name) == normalized)
            .take(limit)
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub record_index: u64,
    pub record: StoreRecord,
    pub name: String,
}

fn write_checkpoint_sidecars(names: &Path, ids: &Path, source_count: u64) -> io::Result<()> {
    let index_path = strip_known_suffix(names, ".names");
    write_name_checkpoint_file(names, &name_checkpoints_path(&index_path), source_count)?;
    write_id_checkpoint_file(ids, &id_checkpoints_path(&index_path), source_count)
}

fn write_name_checkpoint_file(
    source: &Path,
    final_path: &Path,
    source_count: u64,
) -> io::Result<()> {
    let staging = temp_path(final_path, "build.tmp");
    let mut input = File::open(source)?;
    let count = validate_sidecar_header(&mut input, NAME_MAGIC, NAME_ENTRY_SIZE)?;
    if count != source_count {
        return Err(invalid_data(
            "name sidecar count changed during checkpoint build",
        ));
    }
    let checkpoint_count = if count == 0 {
        0
    } else {
        (count - 1) / NAME_CHECKPOINT_STRIDE + 1
    };
    let mut out = BufWriter::with_capacity(64 * 1024, File::create(&staging)?);
    write_checkpoint_header(
        &mut out,
        NAME_CHECKPOINT_MAGIC,
        NAME_CHECKPOINT_SIZE,
        source_count,
        checkpoint_count,
    )?;
    for index in 0..count {
        let entry = NameEntry::read_from(&mut input)?;
        if index % NAME_CHECKPOINT_STRIDE == 0 {
            out.write_all(&entry.key)?;
            out.write_all(&index.to_le_bytes())?;
        }
    }
    out.flush()?;
    out.get_ref().sync_all()?;
    atomic_replace(&staging, final_path)
}

fn write_id_checkpoint_file(source: &Path, final_path: &Path, source_count: u64) -> io::Result<()> {
    let staging = temp_path(final_path, "build.tmp");
    let mut input = File::open(source)?;
    let count = validate_sidecar_header(&mut input, ID_MAGIC, ID_ENTRY_SIZE)?;
    if count != source_count {
        return Err(invalid_data(
            "id sidecar count changed during checkpoint build",
        ));
    }
    let checkpoint_count = if count == 0 {
        0
    } else {
        (count - 1) / ID_CHECKPOINT_STRIDE + 1
    };
    let mut out = BufWriter::with_capacity(64 * 1024, File::create(&staging)?);
    write_checkpoint_header(
        &mut out,
        ID_CHECKPOINT_MAGIC,
        ID_CHECKPOINT_SIZE,
        source_count,
        checkpoint_count,
    )?;
    for index in 0..count {
        let entry = IdEntry::read_from(&mut input)?;
        if index % ID_CHECKPOINT_STRIDE == 0 {
            out.write_all(&entry.file_id.to_le_bytes())?;
            out.write_all(&index.to_le_bytes())?;
        }
    }
    out.flush()?;
    out.get_ref().sync_all()?;
    atomic_replace(&staging, final_path)
}

fn load_name_checkpoints(
    path: &Path,
    names: &mut File,
    count: u64,
) -> io::Result<Vec<NameCheckpoint>> {
    let checkpoint_path = name_checkpoints_path(path);
    if checkpoint_path.exists() {
        let parsed = (|| -> io::Result<Vec<NameCheckpoint>> {
            let mut input = BufReader::with_capacity(64 * 1024, File::open(&checkpoint_path)?);
            let checkpoint_count = read_checkpoint_header(
                &mut input,
                NAME_CHECKPOINT_MAGIC,
                NAME_CHECKPOINT_SIZE,
                count,
            )?;
            let mut result = Vec::with_capacity(checkpoint_count as usize);
            for _ in 0..checkpoint_count {
                let mut key = [0_u8; NAME_KEY_BYTES];
                input.read_exact(&mut key)?;
                result.push(NameCheckpoint {
                    key,
                    entry_index: read_u64(&mut input)?,
                });
            }
            Ok(result)
        })();
        if let Ok(result) = parsed {
            return Ok(result);
        }
    }
    build_name_checkpoints(names, count, NAME_CHECKPOINT_STRIDE)
}

fn load_id_checkpoints(path: &Path, ids: &mut File, count: u64) -> io::Result<Vec<IdCheckpoint>> {
    let checkpoint_path = id_checkpoints_path(path);
    if checkpoint_path.exists() {
        let parsed = (|| -> io::Result<Vec<IdCheckpoint>> {
            let mut input = BufReader::with_capacity(64 * 1024, File::open(&checkpoint_path)?);
            let checkpoint_count =
                read_checkpoint_header(&mut input, ID_CHECKPOINT_MAGIC, ID_CHECKPOINT_SIZE, count)?;
            let mut result = Vec::with_capacity(checkpoint_count as usize);
            for _ in 0..checkpoint_count {
                result.push(IdCheckpoint {
                    file_id: read_u64(&mut input)?,
                    entry_index: read_u64(&mut input)?,
                });
            }
            Ok(result)
        })();
        if let Ok(result) = parsed {
            return Ok(result);
        }
    }
    build_id_checkpoints(ids, count, ID_CHECKPOINT_STRIDE)
}

pub fn repair_sidecars(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    let mut records = File::open(path)?;
    let header = read_store_header(&mut records)?;
    let expected_offset = STORE_HEADER_SIZE + header.record_count * RECORD_SIZE as u64;
    if header.string_pool_offset != expected_offset {
        return Err(invalid_data("corrupt string-pool offset"));
    }
    let main_len = records.metadata()?.len();
    if header
        .string_pool_offset
        .saturating_add(header.string_pool_bytes)
        > main_len
    {
        return Err(invalid_data("string pool exceeds index file"));
    }

    let mut strings = File::open(path)?;
    records.seek(SeekFrom::Start(STORE_HEADER_SIZE))?;
    let mut names = ChunkedNameSorter::new(path.to_path_buf(), 32_768);
    let mut ids = ChunkedIdSorter::new(path.to_path_buf(), 32_768);

    for record_index in 0..header.record_count {
        let record = StoreRecord::read_from(&mut records)?;
        let end = record.name_offset.saturating_add(record.name_len as u64);
        if end > header.string_pool_bytes {
            return Err(invalid_data("record filename exceeds string pool"));
        }
        let mut bytes = vec![0_u8; record.name_len as usize];
        strings.seek(SeekFrom::Start(
            header.string_pool_offset + record.name_offset,
        ))?;
        strings.read_exact(&mut bytes)?;
        let name =
            String::from_utf8(bytes).map_err(|_| invalid_data("filename is not valid UTF-8"))?;
        names.push(NameEntry::from_name(&name, record_index))?;
        ids.push(IdEntry {
            file_id: record.file_id,
            record_index,
        })?;
    }

    let names_final = names_path(path);
    let ids_final = ids_path(path);
    names.finish(&names_final, header.record_count)?;
    ids.finish(&ids_final, header.record_count)?;
    write_checkpoint_sidecars(&names_final, &ids_final, header.record_count)
}

fn write_checkpoint_header<W: Write>(
    out: &mut W,
    magic: [u8; 8],
    entry_size: u16,
    source_count: u64,
    checkpoint_count: u64,
) -> io::Result<()> {
    out.write_all(&magic)?;
    out.write_all(&STORE_VERSION.to_le_bytes())?;
    out.write_all(&entry_size.to_le_bytes())?;
    out.write_all(&source_count.to_le_bytes())?;
    out.write_all(&checkpoint_count.to_le_bytes())?;
    out.write_all(&[0_u8; 4])
}

fn read_checkpoint_header<R: Read>(
    input: &mut R,
    expected_magic: [u8; 8],
    expected_entry_size: u16,
    expected_source_count: u64,
) -> io::Result<u64> {
    let mut magic = [0_u8; 8];
    input.read_exact(&mut magic)?;
    if magic != expected_magic {
        return Err(invalid_data("invalid checkpoint sidecar magic"));
    }
    if read_u16(input)? != STORE_VERSION || read_u16(input)? != expected_entry_size {
        return Err(invalid_data("unsupported checkpoint sidecar format"));
    }
    if read_u64(input)? != expected_source_count {
        return Err(invalid_data("checkpoint sidecar source count mismatch"));
    }
    let checkpoint_count = read_u64(input)?;
    let mut reserved = [0_u8; 4];
    input.read_exact(&mut reserved)?;
    Ok(checkpoint_count)
}

fn strip_known_suffix(path: &Path, suffix: &str) -> PathBuf {
    let text = path.as_os_str().to_string_lossy();
    if let Some(stripped) = text.strip_suffix(suffix) {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

fn build_name_checkpoints(
    file: &mut File,
    count: u64,
    stride: u64,
) -> io::Result<Vec<NameCheckpoint>> {
    let mut result = Vec::with_capacity((count / stride + 1) as usize);
    file.seek(SeekFrom::Start(NAME_HEADER_SIZE))?;
    for index in 0..count {
        let entry = NameEntry::read_from(file)?;
        if index % stride == 0 {
            result.push(NameCheckpoint {
                key: entry.key,
                entry_index: index,
            });
        }
    }
    Ok(result)
}

fn build_id_checkpoints(file: &mut File, count: u64, stride: u64) -> io::Result<Vec<IdCheckpoint>> {
    let mut result = Vec::with_capacity((count / stride + 1) as usize);
    file.seek(SeekFrom::Start(ID_HEADER_SIZE))?;
    for index in 0..count {
        let entry = IdEntry::read_from(file)?;
        if index % stride == 0 {
            result.push(IdCheckpoint {
                file_id: entry.file_id,
                entry_index: index,
            });
        }
    }
    Ok(result)
}

fn checkpoint_start_name(checkpoints: &[NameCheckpoint], key: &[u8; NAME_KEY_BYTES]) -> u64 {
    match checkpoints.binary_search_by(|checkpoint| checkpoint.key.cmp(key)) {
        Ok(index) => checkpoints[index].entry_index,
        Err(0) => 0,
        Err(index) => checkpoints[index - 1].entry_index,
    }
}

fn checkpoint_start_id(checkpoints: &[IdCheckpoint], file_id: u64) -> u64 {
    match checkpoints.binary_search_by_key(&file_id, |checkpoint| checkpoint.file_id) {
        Ok(index) => checkpoints[index].entry_index,
        Err(0) => 0,
        Err(index) => checkpoints[index - 1].entry_index,
    }
}

pub fn normalize_name(input: &str) -> String {
    input.chars().flat_map(char::to_lowercase).collect()
}

pub fn names_path(index_path: &Path) -> PathBuf {
    append_suffix(index_path, ".names")
}

pub fn ids_path(index_path: &Path) -> PathBuf {
    append_suffix(index_path, ".ids")
}

pub fn name_checkpoints_path(index_path: &Path) -> PathBuf {
    append_suffix(index_path, ".ncp")
}

pub fn id_checkpoints_path(index_path: &Path) -> PathBuf {
    append_suffix(index_path, ".icp")
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn temp_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".");
    value.push(suffix);
    PathBuf::from(value)
}

fn cleanup_temp_family(path: &Path) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(());
    };
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with(name) && file_name.ends_with(".tmp") {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}

fn copy_file<W: Write>(path: &Path, out: &mut W) -> io::Result<u64> {
    let mut input = BufReader::with_capacity(512 * 1024, File::open(path)?);
    io::copy(&mut input, out)
}

fn atomic_replace(staging: &Path, final_path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        if final_path.exists() {
            let backup = temp_path(final_path, "old.tmp");
            let _ = fs::remove_file(&backup);
            fs::rename(final_path, &backup)?;
            match fs::rename(staging, final_path) {
                Ok(()) => {
                    let _ = fs::remove_file(backup);
                    Ok(())
                }
                Err(error) => {
                    let _ = fs::rename(backup, final_path);
                    Err(error)
                }
            }
        } else {
            fs::rename(staging, final_path)
        }
    }
    #[cfg(not(windows))]
    {
        fs::rename(staging, final_path)
    }
}

fn sync_parent(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        if let Some(parent) = path.parent() {
            File::open(parent)?.sync_all()?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn file_len(path: &Path) -> io::Result<u64> {
    Ok(fs::metadata(path)?.len())
}

fn invalid_data(message: &'static str) -> io::Error {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("search-tool-{name}-{nonce}.stidx"))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(names_path(path));
        let _ = fs::remove_file(ids_path(path));
        let _ = fs::remove_file(name_checkpoints_path(path));
        let _ = fs::remove_file(id_checkpoints_path(path));
    }

    #[test]
    fn build_open_search_and_reconstruct_path() {
        let path = test_path("roundtrip");
        let mut builder = IndexBuilder::create(
            &path,
            BuildOptions {
                sort_chunk_entries: 1024,
            },
        )
        .unwrap();
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
                name: "Program Files",
            })
            .unwrap();
        builder
            .push(InputRecord {
                file_id: 20,
                parent_id: 10,
                size_bytes: 1234,
                flags: 0,
                name: "node.exe",
            })
            .unwrap();
        builder
            .push(InputRecord {
                file_id: 21,
                parent_id: 10,
                size_bytes: 456,
                flags: 0,
                name: "notepad.exe",
            })
            .unwrap();
        let stats = builder.finish().unwrap();
        assert_eq!(stats.records, 4);

        let mut store = SearchStore::open(&path).unwrap();
        assert_eq!(store.record_count(), 4);
        let exact = store.search_exact("NODE.EXE", 10).unwrap();
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].name, "node.exe");
        assert_eq!(
            store.reconstruct_path(exact[0].record_index, 64).unwrap(),
            "C:\\Program Files\\node.exe"
        );

        let prefix = store.search_prefix("note", 10).unwrap();
        assert_eq!(prefix.len(), 1);
        assert_eq!(prefix[0].name, "notepad.exe");
        cleanup(&path);
    }

    #[test]
    fn open_waits_for_publish_snapshot_before_reading_family() {
        use crate::index_lock::{publish_lock_path, IndexPublishGuard};
        use std::time::Duration;

        let path = test_path("publish-snapshot");
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
        let opener = std::thread::spawn(move || SearchStore::open(open_path).unwrap());
        std::thread::sleep(Duration::from_millis(25));
        assert!(
            !opener.is_finished(),
            "SearchStore::open must wait while the base family is being published"
        );

        fs::write(&path, &original).unwrap();
        drop(publisher);

        let mut opened = opener.join().unwrap();
        assert_eq!(opened.search_exact("stable.txt", 4).unwrap().len(), 1);

        cleanup(&path);
        let _ = fs::remove_file(publish_lock_path(path));
    }

    #[test]
    fn external_sort_multiple_chunks_stays_correct() {
        let path = test_path("chunks");
        let mut builder = IndexBuilder::create(
            &path,
            BuildOptions {
                sort_chunk_entries: 1024,
            },
        )
        .unwrap();
        for i in (0..5000_u64).rev() {
            let name = format!("file-{i:05}.txt");
            builder
                .push(InputRecord {
                    file_id: i + 100,
                    parent_id: 0,
                    size_bytes: i,
                    flags: 0,
                    name: &name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        let mut store = SearchStore::open(&path).unwrap();
        let hits = store.search_prefix("file-012", 200).unwrap();
        assert_eq!(hits.len(), 100);
        assert!(hits.iter().all(|hit| hit.name.starts_with("file-012")));
        assert!(store.memory_hint_bytes() < 32 * 1024);
        cleanup(&path);
    }

    #[test]
    fn corrupt_checkpoint_falls_back_to_primary_sidecar() {
        let path = test_path("checkpoint-fallback");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        builder
            .push(InputRecord {
                file_id: 1,
                parent_id: 0,
                size_bytes: 0,
                flags: 0,
                name: "needle.txt",
            })
            .unwrap();
        builder.finish().unwrap();
        fs::write(name_checkpoints_path(&path), b"corrupt").unwrap();
        fs::write(id_checkpoints_path(&path), b"corrupt").unwrap();
        let mut store = SearchStore::open(&path).unwrap();
        assert_eq!(store.search_exact("needle.txt", 1).unwrap().len(), 1);
        cleanup(&path);
    }

    #[test]
    fn stale_build_lock_file_does_not_block_new_builder() {
        let path = test_path("stale-builder-lock");
        let lock = temp_path(&path, "lock");
        fs::write(&lock, b"stale").unwrap();

        let builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        drop(builder);

        let _ = fs::remove_file(lock);
        let _ = fs::remove_file(temp_path(&path, "records.tmp"));
        let _ = fs::remove_file(temp_path(&path, "strings.tmp"));
        cleanup(&path);
    }

    #[test]
    fn concurrent_builder_is_blocked_by_os_lock() {
        let path = test_path("builder-lock-exclusive");
        let first = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        let error = IndexBuilder::create(&path, BuildOptions::default()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        drop(first);

        let second = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        drop(second);
        let _ = fs::remove_file(temp_path(&path, "lock"));
        cleanup(&path);
    }

    #[test]
    fn sidecars_can_be_rebuilt_from_main_index() {
        let path = test_path("repair-sidecars");
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (id, name) in [(1, "root"), (2, "node.exe"), (3, "package.json")] {
            builder
                .push(InputRecord {
                    file_id: id,
                    parent_id: if id == 1 { 0 } else { 1 },
                    size_bytes: 0,
                    flags: 0,
                    name,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        fs::remove_file(names_path(&path)).unwrap();
        fs::remove_file(ids_path(&path)).unwrap();
        let _ = fs::remove_file(name_checkpoints_path(&path));
        let _ = fs::remove_file(id_checkpoints_path(&path));
        repair_sidecars(&path).unwrap();
        let mut store = SearchStore::open(&path).unwrap();
        assert_eq!(store.search_exact("node.exe", 1).unwrap().len(), 1);
        assert_eq!(store.lookup_file_id(3).unwrap(), Some(2));
        cleanup(&path);
    }
}
