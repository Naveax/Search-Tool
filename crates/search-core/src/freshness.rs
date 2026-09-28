use crate::delta::{checkpoint_path, read_checkpoint, SyncCheckpoint};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: [u8; 8] = *b"STFRSH\0\0";
const VERSION: u16 = 1;
const SIZE: u64 = 32;
const DELTA_HEADER_SIZE: u64 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexGeneration(pub Option<SyncCheckpoint>);

pub fn capture_index_generation(index_path: impl AsRef<Path>) -> io::Result<IndexGeneration> {
    Ok(IndexGeneration(read_checkpoint(checkpoint_path(
        index_path,
    ))?))
}

pub fn sidecar_freshness_path(sidecar_path: impl AsRef<Path>) -> PathBuf {
    append_suffix(sidecar_path.as_ref(), ".fresh")
}

pub fn write_sidecar_generation(
    sidecar_path: impl AsRef<Path>,
    generation: IndexGeneration,
) -> io::Result<()> {
    let final_path = sidecar_freshness_path(sidecar_path);
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let staging = append_suffix(&final_path, ".tmp");
    {
        let mut out = BufWriter::new(File::create(&staging)?);
        out.write_all(&MAGIC)?;
        out.write_all(&VERSION.to_le_bytes())?;
        out.write_all(&0_u16.to_le_bytes())?;
        match generation.0 {
            Some(checkpoint) => {
                out.write_all(&1_u32.to_le_bytes())?;
                out.write_all(&checkpoint.journal_id.to_le_bytes())?;
                out.write_all(&checkpoint.next_usn.to_le_bytes())?;
            }
            None => {
                out.write_all(&0_u32.to_le_bytes())?;
                out.write_all(&0_u64.to_le_bytes())?;
                out.write_all(&0_i64.to_le_bytes())?;
            }
        }
        out.flush()?;
        out.get_ref().sync_all()?;
    }
    atomic_replace(&staging, &final_path)
}

pub fn read_sidecar_generation(
    sidecar_path: impl AsRef<Path>,
) -> io::Result<Option<IndexGeneration>> {
    let path = sidecar_freshness_path(sidecar_path);
    if !path.exists() {
        return Ok(None);
    }
    let mut input = BufReader::new(File::open(&path)?);
    if input.get_ref().metadata()?.len() != SIZE {
        return Err(invalid("invalid sidecar freshness length"));
    }
    let mut magic = [0_u8; 8];
    input.read_exact(&mut magic)?;
    if magic != MAGIC {
        return Err(invalid("invalid sidecar freshness magic"));
    }
    if read_u16(&mut input)? != VERSION {
        return Err(invalid("unsupported sidecar freshness version"));
    }
    let _ = read_u16(&mut input)?;
    let present = read_u32(&mut input)?;
    let journal_id = read_u64(&mut input)?;
    let next_usn = read_i64(&mut input)?;
    let generation = match present {
        0 => IndexGeneration(None),
        1 => IndexGeneration(Some(SyncCheckpoint {
            journal_id,
            next_usn,
        })),
        _ => return Err(invalid("invalid sidecar freshness state")),
    };
    Ok(Some(generation))
}

pub fn sidecar_is_fresh(
    index_path: impl AsRef<Path>,
    sidecar_path: impl AsRef<Path>,
) -> io::Result<bool> {
    let sidecar_path = sidecar_path.as_ref();
    if !sidecar_path.exists() {
        return Ok(false);
    }
    let current = capture_index_generation(index_path)?;
    Ok(read_sidecar_generation(sidecar_path)? == Some(current))
}

pub fn pending_delta(index_path: impl AsRef<Path>) -> io::Result<bool> {
    let path = append_suffix(index_path.as_ref(), ".delta");
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len() > DELTA_HEADER_SIZE),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
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
    use crate::delta::{write_checkpoint, DeltaRecord, DeltaWriter};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("search-tool-fresh-{name}-{n}.stidx"))
    }

    #[test]
    fn sidecar_generation_tracks_usn_checkpoint() {
        let index = temp("generation");
        let sidecar = append_suffix(&index, ".content");
        fs::write(&sidecar, b"x").unwrap();
        write_checkpoint(
            checkpoint_path(&index),
            SyncCheckpoint {
                journal_id: 9,
                next_usn: 100,
            },
        )
        .unwrap();
        let generation = capture_index_generation(&index).unwrap();
        write_sidecar_generation(&sidecar, generation).unwrap();
        assert!(sidecar_is_fresh(&index, &sidecar).unwrap());

        write_checkpoint(
            checkpoint_path(&index),
            SyncCheckpoint {
                journal_id: 9,
                next_usn: 101,
            },
        )
        .unwrap();
        assert!(!sidecar_is_fresh(&index, &sidecar).unwrap());

        let _ = fs::remove_file(sidecar_freshness_path(&sidecar));
        let _ = fs::remove_file(checkpoint_path(&index));
        let _ = fs::remove_file(sidecar);
    }

    #[test]
    fn pending_delta_ignores_header_only_file() {
        let index = temp("delta");
        let delta = append_suffix(&index, ".delta");
        let writer = DeltaWriter::open(&delta).unwrap();
        drop(writer);
        assert!(!pending_delta(&index).unwrap());

        let mut writer = DeltaWriter::open(&delta).unwrap();
        writer
            .append(&DeltaRecord {
                op: crate::delta::DeltaOp::Upsert,
                file_id: 1,
                parent_id: 1,
                size_bytes: 0,
                flags: 0,
                name: "x".into(),
            })
            .unwrap();
        writer.sync().unwrap();
        assert!(pending_delta(&index).unwrap());
        let _ = fs::remove_file(delta);
    }
}
