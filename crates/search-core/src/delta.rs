use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const DELTA_MAGIC: [u8; 8] = *b"STDLTA\0\0";
const DELTA_VERSION: u16 = 1;
const DELTA_HEADER_SIZE: u64 = 32;
const CHECKPOINT_MAGIC: [u8; 8] = *b"STUSN\0\0\0";
const CHECKPOINT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeltaOp {
    Upsert = 1,
    Delete = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaRecord {
    pub op: DeltaOp,
    pub file_id: u64,
    pub parent_id: u64,
    pub size_bytes: u64,
    pub flags: u16,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncCheckpoint {
    pub journal_id: u64,
    pub next_usn: i64,
}

#[derive(Debug)]
pub struct DeltaWriter {
    path: PathBuf,
    out: BufWriter<File>,
}

impl DeltaWriter {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let exists = path.exists();
        if exists {
            // A process crash can leave only a prefix of the final
            // variable-length record on disk. Readers deliberately ignore that
            // uncommitted tail, but a restarted writer must remove it before
            // appending again; otherwise future bytes could make the old partial
            // record appear complete and consume the beginning of a new record.
            let mut repair = OpenOptions::new().read(true).write(true).open(&path)?;
            validate_delta_header(&mut repair)?;
            truncate_partial_delta_tail(&mut repair)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        if exists {
            validate_delta_header(&mut file)?;
        } else {
            write_delta_header(&mut file)?;
            file.sync_all()?;
        }
        Ok(Self {
            path,
            out: BufWriter::with_capacity(128 * 1024, file),
        })
    }

    pub fn append(&mut self, record: &DeltaRecord) -> io::Result<()> {
        let name = record.name.as_bytes();
        if name.len() > u32::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "delta filename too large",
            ));
        }
        self.out.write_all(&[record.op as u8])?;
        self.out.write_all(&[0_u8])?;
        self.out.write_all(&record.flags.to_le_bytes())?;
        self.out.write_all(&record.file_id.to_le_bytes())?;
        self.out.write_all(&record.parent_id.to_le_bytes())?;
        self.out.write_all(&record.size_bytes.to_le_bytes())?;
        self.out.write_all(&(name.len() as u32).to_le_bytes())?;
        self.out.write_all(&0_u32.to_le_bytes())?;
        self.out.write_all(name)?;
        Ok(())
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.out.flush()?;
        self.out.get_ref().sync_all()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub fn load_latest_delta(
    path: impl AsRef<Path>,
    max_unique_entries: usize,
) -> io::Result<HashMap<u64, DeltaRecord>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let file = File::open(path)?;
    if file.metadata()?.len() < DELTA_HEADER_SIZE {
        return Err(invalid_data("truncated delta file"));
    }
    let mut input = BufReader::with_capacity(128 * 1024, file);
    validate_delta_header(&mut input)?;
    let mut latest = HashMap::new();
    while let Some(record) = read_delta_record(&mut input)? {
        latest.insert(record.file_id, record);
        if latest.len() > max_unique_entries {
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "delta overlay exceeded configured in-memory limit; compact the index",
            ));
        }
    }
    Ok(latest)
}

pub(crate) fn for_each_delta_record(
    path: impl AsRef<Path>,
    mut visit: impl FnMut(u64, DeltaRecord) -> io::Result<()>,
) -> io::Result<u64> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(0);
    }
    let file = File::open(path)?;
    if file.metadata()?.len() < DELTA_HEADER_SIZE {
        return Err(invalid_data("truncated delta file"));
    }
    let mut input = BufReader::with_capacity(128 * 1024, file);
    validate_delta_header(&mut input)?;
    let mut sequence = 0_u64;
    while let Some(record) = read_delta_record(&mut input)? {
        visit(sequence, record)?;
        sequence = sequence.saturating_add(1);
    }
    Ok(sequence)
}

pub fn delta_record_count(path: impl AsRef<Path>) -> io::Result<u64> {
    for_each_delta_record(path, |_sequence, _record| Ok(()))
}

pub fn delta_path(index_path: impl AsRef<Path>) -> PathBuf {
    append_suffix(index_path.as_ref(), ".delta")
}

pub fn checkpoint_path(index_path: impl AsRef<Path>) -> PathBuf {
    append_suffix(index_path.as_ref(), ".usn")
}

pub fn write_checkpoint(path: impl AsRef<Path>, checkpoint: SyncCheckpoint) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let staging = append_suffix(path, ".tmp");
    {
        let mut out = BufWriter::new(File::create(&staging)?);
        out.write_all(&CHECKPOINT_MAGIC)?;
        out.write_all(&CHECKPOINT_VERSION.to_le_bytes())?;
        out.write_all(&0_u16.to_le_bytes())?;
        out.write_all(&checkpoint.journal_id.to_le_bytes())?;
        out.write_all(&checkpoint.next_usn.to_le_bytes())?;
        out.write_all(&0_u32.to_le_bytes())?;
        out.flush()?;
        out.get_ref().sync_all()?;
    }
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(staging, path)
}

pub fn read_checkpoint(path: impl AsRef<Path>) -> io::Result<Option<SyncCheckpoint>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(None);
    }
    let mut input = BufReader::new(File::open(path)?);
    let mut magic = [0_u8; 8];
    input.read_exact(&mut magic)?;
    if magic != CHECKPOINT_MAGIC {
        return Err(invalid_data("invalid USN checkpoint magic"));
    }
    if read_u16(&mut input)? != CHECKPOINT_VERSION {
        return Err(invalid_data("unsupported USN checkpoint version"));
    }
    let _ = read_u16(&mut input)?;
    let journal_id = read_u64(&mut input)?;
    let next_usn = read_i64(&mut input)?;
    let _ = read_u32(&mut input)?;
    Ok(Some(SyncCheckpoint {
        journal_id,
        next_usn,
    }))
}

fn truncate_partial_delta_tail(file: &mut File) -> io::Result<()> {
    let file_len = file.metadata()?.len();
    if file_len < DELTA_HEADER_SIZE {
        return Err(invalid_data("truncated delta file"));
    }

    let mut input = BufReader::with_capacity(128 * 1024, file.try_clone()?);
    input.seek(SeekFrom::Start(DELTA_HEADER_SIZE))?;
    loop {
        let record_start = input.stream_position()?;
        if record_start >= file_len {
            break;
        }
        match read_delta_record(&mut input)? {
            Some(_) => {}
            None => {
                file.set_len(record_start)?;
                file.sync_all()?;
                break;
            }
        }
    }
    Ok(())
}

fn write_delta_header<W: Write>(out: &mut W) -> io::Result<()> {
    out.write_all(&DELTA_MAGIC)?;
    out.write_all(&DELTA_VERSION.to_le_bytes())?;
    out.write_all(&0_u16.to_le_bytes())?;
    out.write_all(&[0_u8; 20])
}

fn validate_delta_header<R: Read>(input: &mut R) -> io::Result<()> {
    let mut magic = [0_u8; 8];
    input.read_exact(&mut magic)?;
    if magic != DELTA_MAGIC {
        return Err(invalid_data("invalid delta magic"));
    }
    if read_u16(input)? != DELTA_VERSION {
        return Err(invalid_data("unsupported delta version"));
    }
    let _ = read_u16(input)?;
    let mut reserved = [0_u8; 20];
    input.read_exact(&mut reserved)?;
    Ok(())
}

fn read_delta_record<R: Read>(input: &mut R) -> io::Result<Option<DeltaRecord>> {
    // The delta is an append-only crash log. Readers may race a writer after the
    // OS has exposed only part of the newest record, or a crash may leave a
    // partial tail permanently. A partial *last* record is therefore not index
    // corruption: ignore it and let the next refresh/replay observe the complete
    // record once the writer flushes it. Invalid fully-readable fields still
    // fail closed.
    let mut op = [0_u8; 1];
    if !read_exact_or_partial_tail(input, &mut op)? {
        return Ok(None);
    }
    let mut pad = [0_u8; 1];
    if !read_exact_or_partial_tail(input, &mut pad)? {
        return Ok(None);
    }

    let Some(flags) = read_u16_or_partial_tail(input)? else {
        return Ok(None);
    };
    let Some(file_id) = read_u64_or_partial_tail(input)? else {
        return Ok(None);
    };
    let Some(parent_id) = read_u64_or_partial_tail(input)? else {
        return Ok(None);
    };
    let Some(size_bytes) = read_u64_or_partial_tail(input)? else {
        return Ok(None);
    };
    let Some(name_len) = read_u32_or_partial_tail(input)? else {
        return Ok(None);
    };
    let Some(_reserved) = read_u32_or_partial_tail(input)? else {
        return Ok(None);
    };
    let name_len = name_len as usize;
    if name_len > 64 * 1024 {
        return Err(invalid_data("delta filename exceeds safety limit"));
    }
    let mut name = vec![0_u8; name_len];
    if !read_exact_or_partial_tail(input, &mut name)? {
        return Ok(None);
    }
    let name = String::from_utf8(name).map_err(|_| invalid_data("delta filename is not UTF-8"))?;
    let op = match op[0] {
        1 => DeltaOp::Upsert,
        2 => DeltaOp::Delete,
        _ => return Err(invalid_data("invalid delta operation")),
    };
    Ok(Some(DeltaRecord {
        op,
        file_id,
        parent_id,
        size_bytes,
        flags,
        name,
    }))
}

fn read_exact_or_partial_tail<R: Read>(input: &mut R, bytes: &mut [u8]) -> io::Result<bool> {
    match input.read_exact(bytes) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(error),
    }
}

fn read_u16_or_partial_tail<R: Read>(input: &mut R) -> io::Result<Option<u16>> {
    let mut bytes = [0_u8; 2];
    Ok(read_exact_or_partial_tail(input, &mut bytes)?.then(|| u16::from_le_bytes(bytes)))
}

fn read_u32_or_partial_tail<R: Read>(input: &mut R) -> io::Result<Option<u32>> {
    let mut bytes = [0_u8; 4];
    Ok(read_exact_or_partial_tail(input, &mut bytes)?.then(|| u32::from_le_bytes(bytes)))
}

fn read_u64_or_partial_tail<R: Read>(input: &mut R) -> io::Result<Option<u64>> {
    let mut bytes = [0_u8; 8];
    Ok(read_exact_or_partial_tail(input, &mut bytes)?.then(|| u64::from_le_bytes(bytes)))
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
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
fn read_i64<R: Read>(input: &mut R) -> io::Result<i64> {
    let mut bytes = [0_u8; 8];
    input.read_exact(&mut bytes)?;
    Ok(i64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("search-tool-{name}-{n}"))
    }

    #[test]
    fn delta_keeps_last_state_per_file_id() {
        let path = temp("delta");
        let mut out = DeltaWriter::open(&path).unwrap();
        out.append(&DeltaRecord {
            op: DeltaOp::Upsert,
            file_id: 7,
            parent_id: 1,
            size_bytes: 0,
            flags: 0,
            name: "old.txt".into(),
        })
        .unwrap();
        out.append(&DeltaRecord {
            op: DeltaOp::Upsert,
            file_id: 7,
            parent_id: 2,
            size_bytes: 0,
            flags: 0,
            name: "new.txt".into(),
        })
        .unwrap();
        out.append(&DeltaRecord {
            op: DeltaOp::Delete,
            file_id: 8,
            parent_id: 0,
            size_bytes: 0,
            flags: 0,
            name: String::new(),
        })
        .unwrap();
        out.sync().unwrap();
        drop(out);
        let map = load_latest_delta(&path, 16).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map[&7].name, "new.txt");
        assert_eq!(map[&8].op, DeltaOp::Delete);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn partial_tail_during_append_is_ignored_until_record_is_complete() {
        let path = temp("partial-tail");
        let mut out = DeltaWriter::open(&path).unwrap();
        out.append(&DeltaRecord {
            op: DeltaOp::Upsert,
            file_id: 7,
            parent_id: 1,
            size_bytes: 3,
            flags: 0,
            name: "complete.txt".into(),
        })
        .unwrap();
        out.sync().unwrap();
        drop(out);

        // Simulate a concurrently flushed/crash-truncated second record. This
        // used to surface as raw UnexpectedEof/"failed to fill whole buffer".
        let mut append = OpenOptions::new().append(true).open(&path).unwrap();
        append.write_all(&[DeltaOp::Upsert as u8, 0]).unwrap();
        append.write_all(&0_u16.to_le_bytes()).unwrap();
        append.write_all(&8_u64.to_le_bytes()).unwrap();
        append.write_all(&1_u64.to_le_bytes()).unwrap();
        append.write_all(&4_u64.to_le_bytes()).unwrap();
        append.write_all(&12_u32.to_le_bytes()).unwrap();
        append.write_all(&0_u32.to_le_bytes()).unwrap();
        append.write_all(b"partial").unwrap();
        append.sync_all().unwrap();
        drop(append);

        let map = load_latest_delta(&path, 16).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map[&7].name, "complete.txt");
        assert!(!map.contains_key(&8));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn writer_reopen_truncates_partial_tail_before_new_append() {
        let path = temp("repair-partial-tail");
        let mut out = DeltaWriter::open(&path).unwrap();
        out.append(&DeltaRecord {
            op: DeltaOp::Upsert,
            file_id: 7,
            parent_id: 1,
            size_bytes: 3,
            flags: 0,
            name: "complete.txt".into(),
        })
        .unwrap();
        out.sync().unwrap();
        drop(out);
        let committed_len = fs::metadata(&path).unwrap().len();

        let mut append = OpenOptions::new().append(true).open(&path).unwrap();
        append.write_all(&[DeltaOp::Upsert as u8, 0]).unwrap();
        append.write_all(&0_u16.to_le_bytes()).unwrap();
        append.write_all(&8_u64.to_le_bytes()).unwrap();
        append.write_all(&1_u64.to_le_bytes()).unwrap();
        append.write_all(&4_u64.to_le_bytes()).unwrap();
        append.write_all(&12_u32.to_le_bytes()).unwrap();
        append.write_all(&0_u32.to_le_bytes()).unwrap();
        append.write_all(b"partial").unwrap();
        append.sync_all().unwrap();
        drop(append);
        assert!(fs::metadata(&path).unwrap().len() > committed_len);

        let mut reopened = DeltaWriter::open(&path).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), committed_len);
        reopened
            .append(&DeltaRecord {
                op: DeltaOp::Upsert,
                file_id: 9,
                parent_id: 1,
                size_bytes: 5,
                flags: 0,
                name: "after.txt".into(),
            })
            .unwrap();
        reopened.sync().unwrap();
        drop(reopened);

        let map = load_latest_delta(&path, 16).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map[&7].name, "complete.txt");
        assert_eq!(map[&9].name, "after.txt");
        assert!(!map.contains_key(&8));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn partial_fixed_header_tail_is_ignored() {
        let path = temp("partial-fixed-tail");
        let mut out = DeltaWriter::open(&path).unwrap();
        out.append(&DeltaRecord {
            op: DeltaOp::Upsert,
            file_id: 1,
            parent_id: 1,
            size_bytes: 0,
            flags: 0,
            name: "stable.txt".into(),
        })
        .unwrap();
        out.sync().unwrap();
        drop(out);

        let mut append = OpenOptions::new().append(true).open(&path).unwrap();
        append.write_all(&[DeltaOp::Delete as u8, 0, 0]).unwrap();
        append.sync_all().unwrap();
        drop(append);

        let map = load_latest_delta(&path, 16).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map[&1].name, "stable.txt");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn checkpoint_roundtrip() {
        let path = temp("checkpoint");
        let value = SyncCheckpoint {
            journal_id: 123,
            next_usn: 456,
        };
        write_checkpoint(&path, value).unwrap();
        assert_eq!(read_checkpoint(&path).unwrap(), Some(value));
        let _ = fs::remove_file(path);
    }
}
