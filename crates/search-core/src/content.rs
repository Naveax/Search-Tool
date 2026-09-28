use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: [u8; 8] = *b"STCONT\0\0";
const VERSION: u16 = 1;
const HEADER_SIZE: u64 = 32;
const ENTRY_SIZE: u16 = 16;
const DEFAULT_CHUNK_ENTRIES: usize = 262_144;
const CHECKPOINT_STRIDE: u64 = 2048;
const CHECKPOINT_MAGIC: [u8; 8] = *b"STCNCP\0\0";
const CHECKPOINT_VERSION: u16 = 1;
const CHECKPOINT_ENTRY_SIZE: u16 = 16;
const CHECKPOINT_HEADER_SIZE: u64 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Posting {
    token: u64,
    file_id: u64,
}

impl Posting {
    fn write_to<W: Write>(&self, out: &mut W) -> io::Result<()> {
        out.write_all(&self.token.to_le_bytes())?;
        out.write_all(&self.file_id.to_le_bytes())
    }
    fn read_from<R: Read>(input: &mut R) -> io::Result<Self> {
        Ok(Self {
            token: read_u64(input)?,
            file_id: read_u64(input)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Checkpoint {
    token: u64,
    entry: u64,
}

#[derive(Debug)]
pub struct ContentIndexBuilder {
    final_path: PathBuf,
    _build_lock: File,
    buffer: Vec<Posting>,
    chunks: Vec<PathBuf>,
    max_entries: usize,
    postings: u64,
}

impl ContentIndexBuilder {
    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::create_with_chunk(path, DEFAULT_CHUNK_ENTRIES)
    }

    pub fn create_with_chunk(path: impl AsRef<Path>, max_entries: usize) -> io::Result<Self> {
        let final_path = path.as_ref().to_path_buf();
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let lock_path = suffix(&final_path, ".build.lock");
        let build_lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(lock_path)?;
        build_lock.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => io::Error::new(
                io::ErrorKind::WouldBlock,
                "content sidecar build is already in progress",
            ),
            std::fs::TryLockError::Error(error) => error,
        })?;
        cleanup_stale_build_files(&final_path)?;

        Ok(Self {
            final_path,
            _build_lock: build_lock,
            buffer: Vec::with_capacity(max_entries.max(1024)),
            chunks: Vec::new(),
            max_entries: max_entries.max(1024),
            postings: 0,
        })
    }

    pub fn add_text(
        &mut self,
        file_id: u64,
        text: &str,
        max_tokens_per_file: usize,
    ) -> io::Result<usize> {
        let mut unique = BTreeSet::new();
        for token in tokenize(text) {
            unique.insert(hash_token(&token));
            if unique.len() >= max_tokens_per_file {
                break;
            }
        }
        let count = unique.len();
        for token in unique {
            self.buffer.push(Posting { token, file_id });
            self.postings += 1;
            if self.buffer.len() >= self.max_entries {
                self.flush_chunk()?;
            }
        }
        Ok(count)
    }

    pub fn finish(mut self) -> io::Result<u64> {
        self.flush_chunk()?;
        let staging = suffix(&self.final_path, ".tmp");
        let checkpoints_final = content_checkpoints_path(&self.final_path);
        let checkpoints_staging = suffix(&checkpoints_final, ".tmp");
        let mut out = BufWriter::with_capacity(512 * 1024, File::create(&staging)?);
        let mut checkpoints =
            BufWriter::with_capacity(64 * 1024, File::create(&checkpoints_staging)?);
        write_content_header(&mut out, 0)?;
        write_checkpoint_header(&mut checkpoints, 0, 0)?;
        let (posting_count, checkpoint_count) =
            merge_chunks(&self.chunks, &mut out, &mut checkpoints)?;

        out.flush()?;
        out.seek(SeekFrom::Start(0))?;
        write_content_header(&mut out, posting_count)?;
        out.flush()?;
        out.get_ref().sync_all()?;

        checkpoints.flush()?;
        checkpoints.seek(SeekFrom::Start(0))?;
        write_checkpoint_header(&mut checkpoints, checkpoint_count, posting_count)?;
        checkpoints.flush()?;
        checkpoints.get_ref().sync_all()?;

        replace_file(&staging, &self.final_path)?;
        replace_file(&checkpoints_staging, &checkpoints_final)?;
        for chunk in self.chunks {
            let _ = fs::remove_file(chunk);
        }
        Ok(posting_count)
    }

    fn flush_chunk(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.buffer.sort_unstable_by_key(|p| (p.token, p.file_id));
        self.buffer.dedup();
        let path = suffix(
            &self.final_path,
            &format!(".chunk{}.tmp", self.chunks.len()),
        );
        let mut out = BufWriter::with_capacity(256 * 1024, File::create(&path)?);
        for posting in &self.buffer {
            posting.write_to(&mut out)?;
        }
        out.flush()?;
        self.buffer.clear();
        self.chunks.push(path);
        Ok(())
    }
}

fn cleanup_stale_build_files(final_path: &Path) -> io::Result<()> {
    remove_file_if_exists(&suffix(final_path, ".tmp"))?;
    remove_file_if_exists(&suffix(&content_checkpoints_path(final_path), ".tmp"))?;

    let Some(parent) = final_path.parent() else {
        return Ok(());
    };
    let Some(file_name) = final_path.file_name() else {
        return Ok(());
    };
    let chunk_prefix = format!("{}.chunk", file_name.to_string_lossy());

    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(rest) = name.strip_prefix(&chunk_prefix) else {
            continue;
        };
        let Some(number) = rest.strip_suffix(".tmp") else {
            continue;
        };
        if !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()) {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[derive(Debug)]
pub struct ContentIndex {
    file: File,
    count: u64,
    checkpoints: Vec<Checkpoint>,
}

impl ContentIndex {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let mut magic = [0_u8; 8];
        file.read_exact(&mut magic)?;
        if magic != MAGIC {
            return Err(invalid("invalid content index magic"));
        }
        if read_u16(&mut file)? != VERSION {
            return Err(invalid("unsupported content index version"));
        }
        if read_u16(&mut file)? != ENTRY_SIZE {
            return Err(invalid("unexpected content entry size"));
        }
        let count = read_u64(&mut file)?;
        let mut reserved = [0_u8; 12];
        file.read_exact(&mut reserved)?;
        let expected = HEADER_SIZE.saturating_add(count.saturating_mul(ENTRY_SIZE as u64));
        if file.metadata()?.len() != expected {
            return Err(invalid("content index length mismatch"));
        }
        let checkpoint_path = content_checkpoints_path(path);
        let checkpoints = match load_content_checkpoints(&checkpoint_path, count) {
            Ok(checkpoints) => checkpoints,
            Err(_) => {
                rebuild_content_checkpoints(path, count)?;
                load_content_checkpoints(&checkpoint_path, count)?
            }
        };
        Ok(Self {
            file,
            count,
            checkpoints,
        })
    }

    pub fn memory_hint_bytes(&self) -> usize {
        self.checkpoints.capacity() * std::mem::size_of::<Checkpoint>()
    }

    pub fn search(&mut self, query: &str, limit: usize) -> io::Result<Vec<u64>> {
        let tokens: Vec<u64> = tokenize(query)
            .into_iter()
            .map(|t| hash_token(&t))
            .collect();
        if tokens.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let mut current: Option<Vec<u64>> = None;
        for token in tokens {
            let postings = self.postings_for(token, limit.saturating_mul(64).max(4096))?;
            current = Some(match current {
                None => postings,
                Some(left) => {
                    intersect_sorted(&left, &postings, limit.saturating_mul(64).max(4096))
                }
            });
            if current.as_ref().is_some_and(Vec::is_empty) {
                break;
            }
        }
        let mut result = current.unwrap_or_default();
        result.truncate(limit);
        Ok(result)
    }

    fn postings_for(&mut self, token: u64, cap: usize) -> io::Result<Vec<u64>> {
        let start = checkpoint_start(&self.checkpoints, token);
        self.file
            .seek(SeekFrom::Start(HEADER_SIZE + start * ENTRY_SIZE as u64))?;
        let mut result = Vec::new();
        for _ in start..self.count {
            let posting = Posting::read_from(&mut self.file)?;
            match posting.token.cmp(&token) {
                Ordering::Less => continue,
                Ordering::Equal => {
                    result.push(posting.file_id);
                    if result.len() >= cap {
                        break;
                    }
                }
                Ordering::Greater => break,
            }
        }
        result.sort_unstable();
        result.dedup();
        Ok(result)
    }
}

pub fn content_path(index_path: impl AsRef<Path>) -> PathBuf {
    suffix(index_path.as_ref(), ".content")
}

pub fn content_checkpoints_path(content_index_path: impl AsRef<Path>) -> PathBuf {
    suffix(content_index_path.as_ref(), ".cpc")
}

pub fn is_text_candidate(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        ".txt", ".md", ".rs", ".py", ".js", ".ts", ".tsx", ".jsx", ".c", ".cc", ".cpp", ".h",
        ".hpp", ".cs", ".java", ".json", ".toml", ".yaml", ".yml", ".xml", ".ini", ".cfg", ".conf",
        ".log", ".ps1", ".bat", ".cmd", ".sh", ".html", ".css", ".sql",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
}

fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            for lower in ch.to_lowercase() {
                current.push(lower);
            }
            if current.len() > 96 {
                current.clear();
            }
        } else if current.len() >= 2 {
            out.push(std::mem::take(&mut current));
        } else {
            current.clear();
        }
    }
    if current.len() >= 2 {
        out.push(current);
    }
    out
}

fn hash_token(token: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in token.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct HeapItem {
    posting: Posting,
    source: usize,
}
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        (other.posting.token, other.posting.file_id, other.source).cmp(&(
            self.posting.token,
            self.posting.file_id,
            self.source,
        ))
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
) -> io::Result<(u64, u64)> {
    let mut readers: Vec<_> = chunks
        .iter()
        .map(|p| File::open(p).map(BufReader::new))
        .collect::<io::Result<_>>()?;
    let mut heap = BinaryHeap::new();
    for (source, reader) in readers.iter_mut().enumerate() {
        if let Some(posting) = read_optional(reader)? {
            heap.push(HeapItem { posting, source });
        }
    }
    let mut last = None;
    let mut written = 0_u64;
    let mut checkpoint_count = 0_u64;
    while let Some(item) = heap.pop() {
        if last != Some(item.posting) {
            if written.is_multiple_of(CHECKPOINT_STRIDE) {
                checkpoints.write_all(&item.posting.token.to_le_bytes())?;
                checkpoints.write_all(&written.to_le_bytes())?;
                checkpoint_count = checkpoint_count.saturating_add(1);
            }
            item.posting.write_to(out)?;
            last = Some(item.posting);
            written = written.saturating_add(1);
        }
        if let Some(posting) = read_optional(&mut readers[item.source])? {
            heap.push(HeapItem {
                posting,
                source: item.source,
            });
        }
    }
    Ok((written, checkpoint_count))
}

fn write_content_header<W: Write>(out: &mut W, count: u64) -> io::Result<()> {
    out.write_all(&MAGIC)?;
    out.write_all(&VERSION.to_le_bytes())?;
    out.write_all(&ENTRY_SIZE.to_le_bytes())?;
    out.write_all(&count.to_le_bytes())?;
    out.write_all(&[0_u8; 12])
}

fn write_checkpoint_header<W: Write>(
    out: &mut W,
    checkpoint_count: u64,
    source_count: u64,
) -> io::Result<()> {
    out.write_all(&CHECKPOINT_MAGIC)?;
    out.write_all(&CHECKPOINT_VERSION.to_le_bytes())?;
    out.write_all(&CHECKPOINT_ENTRY_SIZE.to_le_bytes())?;
    out.write_all(&checkpoint_count.to_le_bytes())?;
    out.write_all(&source_count.to_le_bytes())?;
    out.write_all(&[0_u8; 4])
}

fn load_content_checkpoints(path: &Path, source_count: u64) -> io::Result<Vec<Checkpoint>> {
    let mut input = BufReader::with_capacity(64 * 1024, File::open(path)?);
    let mut magic = [0_u8; 8];
    input.read_exact(&mut magic)?;
    if magic != CHECKPOINT_MAGIC {
        return Err(invalid("invalid content checkpoint magic"));
    }
    if read_u16(&mut input)? != CHECKPOINT_VERSION {
        return Err(invalid("unsupported content checkpoint version"));
    }
    if read_u16(&mut input)? != CHECKPOINT_ENTRY_SIZE {
        return Err(invalid("unexpected content checkpoint entry size"));
    }
    let checkpoint_count = read_u64(&mut input)?;
    if read_u64(&mut input)? != source_count {
        return Err(invalid("content checkpoint source count mismatch"));
    }
    let mut reserved = [0_u8; 4];
    input.read_exact(&mut reserved)?;
    let expected = CHECKPOINT_HEADER_SIZE
        .saturating_add(checkpoint_count.saturating_mul(CHECKPOINT_ENTRY_SIZE as u64));
    if input.get_ref().metadata()?.len() != expected {
        return Err(invalid("content checkpoint length mismatch"));
    }
    let mut checkpoints = Vec::with_capacity(checkpoint_count as usize);
    for _ in 0..checkpoint_count {
        checkpoints.push(Checkpoint {
            token: read_u64(&mut input)?,
            entry: read_u64(&mut input)?,
        });
    }
    Ok(checkpoints)
}

fn rebuild_content_checkpoints(path: &Path, count: u64) -> io::Result<()> {
    let final_path = content_checkpoints_path(path);
    let staging = suffix(&final_path, ".tmp");
    let mut input = BufReader::with_capacity(256 * 1024, File::open(path)?);
    input.seek(SeekFrom::Start(HEADER_SIZE))?;
    let checkpoint_count = if count == 0 {
        0
    } else {
        (count - 1) / CHECKPOINT_STRIDE + 1
    };
    let mut output = BufWriter::with_capacity(64 * 1024, File::create(&staging)?);
    write_checkpoint_header(&mut output, checkpoint_count, count)?;
    for entry in 0..count {
        let posting = Posting::read_from(&mut input)?;
        if entry % CHECKPOINT_STRIDE == 0 {
            output.write_all(&posting.token.to_le_bytes())?;
            output.write_all(&entry.to_le_bytes())?;
        }
    }
    output.flush()?;
    output.get_ref().sync_all()?;
    replace_file(&staging, &final_path)
}

fn replace_file(staging: &Path, final_path: &Path) -> io::Result<()> {
    if final_path.exists() {
        fs::remove_file(final_path)?;
    }
    fs::rename(staging, final_path)
}

fn read_optional<R: Read>(input: &mut R) -> io::Result<Option<Posting>> {
    let mut first = [0_u8; 1];
    match input.read_exact(&mut first) {
        Ok(()) => {
            let mut rest = [0_u8; ENTRY_SIZE as usize - 1];
            input.read_exact(&mut rest)?;
            let mut all = [0_u8; ENTRY_SIZE as usize];
            all[0] = first[0];
            all[1..].copy_from_slice(&rest);
            Posting::read_from(&mut &all[..]).map(Some)
        }
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(e),
    }
}

fn checkpoint_start(checkpoints: &[Checkpoint], token: u64) -> u64 {
    match checkpoints.binary_search_by_key(&token, |c| c.token) {
        Ok(i) => checkpoints[i].entry,
        Err(0) => 0,
        Err(i) => checkpoints[i - 1].entry,
    }
}

fn intersect_sorted(a: &[u64], b: &[u64], cap: usize) -> Vec<u64> {
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < a.len() && j < b.len() && out.len() < cap {
        match a[i].cmp(&b[j]) {
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out
}

fn suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}
fn invalid(msg: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}
fn read_u16<R: Read>(r: &mut R) -> io::Result<u16> {
    let mut b = [0; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}
fn read_u64<R: Read>(r: &mut R) -> io::Result<u64> {
    let mut b = [0; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    fn temp() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("search-tool-content-{n}"))
    }
    #[test]
    fn content_query_intersects_terms() {
        let path = temp();
        let mut b = ContentIndexBuilder::create_with_chunk(&path, 1024).unwrap();
        b.add_text(1, "rust search engine fast index", 100).unwrap();
        b.add_text(2, "python search engine", 100).unwrap();
        b.add_text(3, "rust compiler", 100).unwrap();
        b.finish().unwrap();
        let mut i = ContentIndex::open(&path).unwrap();
        assert_eq!(i.search("rust search", 10).unwrap(), vec![1]);
        assert!(i.memory_hint_bytes() < 4096);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(content_checkpoints_path(&path));
    }

    #[test]
    fn missing_checkpoint_is_rebuilt_once() {
        let path = temp();
        let mut b = ContentIndexBuilder::create_with_chunk(&path, 1024).unwrap();
        for id in 1..=5000 {
            b.add_text(id, &format!("common token-{id}"), 100).unwrap();
        }
        b.finish().unwrap();
        let checkpoint = content_checkpoints_path(&path);
        assert!(checkpoint.exists());
        fs::remove_file(&checkpoint).unwrap();
        let mut index = ContentIndex::open(&path).unwrap();
        assert!(checkpoint.exists());
        assert!(!index.search("common", 10).unwrap().is_empty());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(checkpoint);
    }

    #[test]
    fn builder_cleans_stale_temp_files_and_serializes() {
        let path = temp();
        let staging = suffix(&path, ".tmp");
        let checkpoint_staging = suffix(&content_checkpoints_path(&path), ".tmp");
        let chunk0 = suffix(&path, ".chunk0.tmp");
        let chunk42 = suffix(&path, ".chunk42.tmp");
        let unrelated = suffix(&path, ".chunkx.tmp");

        for stale in [&staging, &checkpoint_staging, &chunk0, &chunk42, &unrelated] {
            fs::write(stale, b"stale").unwrap();
        }

        let first = ContentIndexBuilder::create_with_chunk(&path, 1024).unwrap();
        assert!(!staging.exists());
        assert!(!checkpoint_staging.exists());
        assert!(!chunk0.exists());
        assert!(!chunk42.exists());
        assert!(unrelated.exists());

        let error = ContentIndexBuilder::create_with_chunk(&path, 1024).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);

        drop(first);
        let second = ContentIndexBuilder::create_with_chunk(&path, 1024).unwrap();
        drop(second);

        let _ = fs::remove_file(unrelated);
        let _ = fs::remove_file(suffix(&path, ".build.lock"));
    }

    #[test]
    fn candidate_extensions_are_conservative() {
        assert!(is_text_candidate("x\\Cargo.toml"));
        assert!(!is_text_candidate("video.mp4"));
    }
}
