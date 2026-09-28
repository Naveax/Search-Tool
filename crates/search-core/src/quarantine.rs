use crate::{classify_path, cleanup_decision, CleanupAction};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const META_MAGIC: [u8; 8] = *b"STQUAR\0\0";
const META_VERSION: u16 = 1;
const MAX_PATH_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineEntry {
    pub id: String,
    pub original_path: PathBuf,
    pub stored_path: PathBuf,
    pub size_bytes: u64,
}

pub fn quarantine_path(
    source: impl AsRef<Path>,
    root: impl AsRef<Path>,
) -> io::Result<QuarantineEntry> {
    let source = source.as_ref();
    let root = root.as_ref();
    let metadata = fs::symlink_metadata(source)?;
    let display = source.to_string_lossy();
    let decision = cleanup_decision(&display, classify_path(&display, 0));
    if decision.action != CleanupAction::Quarantine {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "cleanup policy does not allow this path to be quarantined",
        ));
    }

    let id = quarantine_id();
    let items = root.join("items");
    let meta = root.join("meta");
    fs::create_dir_all(&items)?;
    fs::create_dir_all(&meta)?;
    let stored_path = items.join(&id);
    if stored_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "quarantine id collision",
        ));
    }

    fs::rename(source, &stored_path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "quarantine move failed; keep quarantine on the same volume as the source: {error}"
            ),
        )
    })?;

    let entry = QuarantineEntry {
        id: id.clone(),
        original_path: source.to_path_buf(),
        stored_path: stored_path.clone(),
        size_bytes: if metadata.is_file() {
            metadata.len()
        } else {
            0
        },
    };
    if let Err(error) = write_meta(&meta.join(format!("{id}.meta")), &entry) {
        let _ = fs::rename(&stored_path, source);
        return Err(error);
    }
    Ok(entry)
}

pub fn restore_quarantine(root: impl AsRef<Path>, id: &str) -> io::Result<QuarantineEntry> {
    validate_id(id)?;
    let root = root.as_ref();
    let meta_path = root.join("meta").join(format!("{id}.meta"));
    let entry = read_meta(&meta_path, root)?;
    if entry.original_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "original path already exists; refusing to overwrite it",
        ));
    }
    if let Some(parent) = entry.original_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&entry.stored_path, &entry.original_path)?;
    fs::remove_file(meta_path)?;
    Ok(entry)
}

pub fn purge_quarantine(root: impl AsRef<Path>, id: &str) -> io::Result<QuarantineEntry> {
    validate_id(id)?;
    let root = root.as_ref();
    let meta_path = root.join("meta").join(format!("{id}.meta"));
    let entry = read_meta(&meta_path, root)?;
    let metadata = fs::symlink_metadata(&entry.stored_path)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(&entry.stored_path)?;
    } else {
        fs::remove_file(&entry.stored_path)?;
    }
    fs::remove_file(meta_path)?;
    Ok(entry)
}

pub fn list_quarantine(root: impl AsRef<Path>, limit: usize) -> io::Result<Vec<QuarantineEntry>> {
    let root = root.as_ref();
    let meta_dir = root.join("meta");
    if !meta_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<_> = fs::read_dir(&meta_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|v| v.to_str()) == Some("meta"))
        .collect();
    paths.sort_unstable();
    paths.reverse();
    let mut entries = Vec::new();
    for path in paths.into_iter().take(limit.max(1)) {
        if let Ok(entry) = read_meta(&path, root) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn quarantine_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos:032x}-{:08x}", std::process::id())
}

fn validate_id(id: &str) -> io::Result<()> {
    if id.len() > 64 || id.is_empty() || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid quarantine id",
        ));
    }
    Ok(())
}

fn write_meta(path: &Path, entry: &QuarantineEntry) -> io::Result<()> {
    let staging = path.with_extension("tmp");
    let original = entry.original_path.to_string_lossy();
    if original.len() > MAX_PATH_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path too large",
        ));
    }
    let mut out = File::create(&staging)?;
    out.write_all(&META_MAGIC)?;
    out.write_all(&META_VERSION.to_le_bytes())?;
    out.write_all(&0_u16.to_le_bytes())?;
    out.write_all(&entry.size_bytes.to_le_bytes())?;
    out.write_all(&(original.len() as u32).to_le_bytes())?;
    out.write_all(original.as_bytes())?;
    out.sync_all()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(staging, path)
}

fn read_meta(path: &Path, root: &Path) -> io::Result<QuarantineEntry> {
    let mut input = File::open(path)?;
    let mut magic = [0_u8; 8];
    input.read_exact(&mut magic)?;
    if magic != META_MAGIC {
        return Err(invalid("invalid quarantine metadata magic"));
    }
    if read_u16(&mut input)? != META_VERSION {
        return Err(invalid("unsupported quarantine metadata version"));
    }
    let _ = read_u16(&mut input)?;
    let size_bytes = read_u64(&mut input)?;
    let len = read_u32(&mut input)? as usize;
    if len > MAX_PATH_BYTES {
        return Err(invalid("quarantine metadata path exceeds safety limit"));
    }
    let mut bytes = vec![0_u8; len];
    input.read_exact(&mut bytes)?;
    let original = String::from_utf8(bytes).map_err(|_| invalid("quarantine path is not UTF-8"))?;
    let id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid("invalid quarantine metadata filename"))?
        .to_string();
    validate_id(&id)?;
    Ok(QuarantineEntry {
        stored_path: root.join("items").join(&id),
        id,
        original_path: PathBuf::from(original),
        size_bytes,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("search-tool-quarantine-{name}-{}", quarantine_id()))
    }

    #[test]
    fn cache_file_can_be_quarantined_and_restored() {
        let root = temp_root("restore");
        let source = root
            .join("AppData")
            .join("Local")
            .join("Temp")
            .join("cache.tmp");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, b"cache").unwrap();
        let quarantine = root.join("quarantine");
        let entry = quarantine_path(&source, &quarantine).unwrap();
        assert!(!source.exists());
        assert!(entry.stored_path.exists());
        restore_quarantine(&quarantine, &entry.id).unwrap();
        assert!(source.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn protected_or_unknown_file_is_denied() {
        let root = std::env::current_dir()
            .unwrap()
            .join(format!("search-tool-quarantine-deny-{}", quarantine_id()));
        let source = root.join("important.bin");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"important").unwrap();
        let error = quarantine_path(&source, root.join("q")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(source.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn purge_permanently_removes_quarantined_item() {
        let root = temp_root("purge");
        let source = root.join("cache").join("payload.tmp");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, b"123456").unwrap();
        let quarantine = root.join("quarantine");
        let entry = quarantine_path(&source, &quarantine).unwrap();
        let stored = entry.stored_path.clone();
        let purged = purge_quarantine(&quarantine, &entry.id).unwrap();
        assert_eq!(purged.size_bytes, 6);
        assert!(!stored.exists());
        assert!(!source.exists());
        assert!(list_quarantine(&quarantine, 10).unwrap().is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
