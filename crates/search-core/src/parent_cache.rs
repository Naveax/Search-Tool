//! Optional, versioned parent adjacency cache for large scoped searches.
//! This cache is an acceleration hint, never a source of search truth.
//! A SHA-256 of the complete base index binds every cache read to its exact
//! byte generation. Corrupt/stale/missing cache falls back to a rebuild.
use crate::store::{SearchStore, FLAG_DIRECTORY};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC: [u8; 8] = *b"STPARENT";
const VERSION: u32 = 1;
const HEADER_BYTES: u64 = 8 + 4 + 8 + 8 + 32 + 32;
// Keep disk cache and asynchronous in-memory adjacency limits in sync.
const MAX_ENTRIES: u64 = 4_000_000;

#[derive(Debug)]
pub(crate) struct BaseParentIndex {
    pub(crate) children: Vec<(u64, u64)>,
    pub(crate) directories: Vec<u64>,
}

impl BaseParentIndex {
    pub(crate) fn build(base: &mut SearchStore) -> io::Result<Self> {
        let mut children = Vec::with_capacity(base.record_count() as usize);
        let mut directories = Vec::new();
        for index in 0..base.record_count() {
            let record = base.read_record(index)?;
            children.push((record.parent_id, index));
            if record.flags & FLAG_DIRECTORY != 0 {
                directories.push(index);
            }
        }
        children.sort_unstable();
        Ok(Self {
            children,
            directories,
        })
    }

    pub(crate) fn child_range(&self, parent_id: u64) -> std::ops::Range<usize> {
        let start = self
            .children
            .partition_point(|(parent, _)| *parent < parent_id);
        let end = self
            .children
            .partition_point(|(parent, _)| *parent <= parent_id);
        start..end
    }

    /// A cached parent table is useful only on large indexes. Do not block
    /// the GUI: callers run this entire routine in a background worker.
    pub(crate) fn load_or_build(path: &Path, base: &mut SearchStore) -> io::Result<Self> {
        let count = base.record_count();
        if count > MAX_ENTRIES {
            return Err(invalid("parent adjacency exceeds guarded size limit"));
        }
        let digest = fingerprint(path)?;
        if let Ok(Some(cached)) = Self::load(path, count, &digest) {
            return Ok(cached);
        }
        let built = Self::build(base)?;
        // An optional disk cache must never make a valid search fail.
        let _ = built.save(path, &digest);
        Ok(built)
    }

    fn load(path: &Path, count: u64, base_digest: &[u8; 32]) -> io::Result<Option<Self>> {
        let file = match File::open(cache_path(path)) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let size = file.metadata()?.len();
        if count > MAX_ENTRIES || size < HEADER_BYTES {
            return Err(invalid("parent cache header size or record limit invalid"));
        }
        let mut reader = BufReader::with_capacity(512 * 1024, file);
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if magic != MAGIC || read_u32(&mut reader)? != VERSION {
            return Err(invalid("parent cache format/version mismatch"));
        }
        let stored_count = read_u64(&mut reader)?;
        let dirs_count = read_u64(&mut reader)?;
        let mut stored_base_digest = [0u8; 32];
        reader.read_exact(&mut stored_base_digest)?;
        let mut stored_body_digest = [0u8; 32];
        reader.read_exact(&mut stored_body_digest)?;
        if stored_count != count || stored_base_digest != *base_digest || dirs_count > count {
            return Err(invalid("parent cache base generation/count mismatch"));
        }
        let wanted_size = HEADER_BYTES
            .checked_add(
                count
                    .checked_mul(16)
                    .ok_or_else(|| invalid("size overflow"))?,
            )
            .and_then(|size| size.checked_add(dirs_count * 8))
            .ok_or_else(|| invalid("size overflow"))?;
        if size != wanted_size {
            return Err(invalid("parent cache length mismatch"));
        }
        let mut body_hash = Sha256::new();
        let mut children = Vec::with_capacity(count as usize);
        let mut seen = vec![false; count as usize];
        let mut previous = None;
        for _ in 0..count {
            let mut bytes = [0u8; 16];
            reader.read_exact(&mut bytes)?;
            body_hash.update(bytes);
            let parent = u64::from_le_bytes(bytes[..8].try_into().unwrap());
            let row = u64::from_le_bytes(bytes[8..].try_into().unwrap());
            if row >= count || seen[row as usize] {
                return Err(invalid("parent cache invalid/duplicate record index"));
            }
            let pair = (parent, row);
            if previous.is_some_and(|last| last > pair) {
                return Err(invalid("parent cache not sorted"));
            }
            previous = Some(pair);
            seen[row as usize] = true;
            children.push(pair);
        }
        let mut directories = Vec::with_capacity(dirs_count as usize);
        for _ in 0..dirs_count {
            let mut bytes = [0u8; 8];
            reader.read_exact(&mut bytes)?;
            body_hash.update(bytes);
            let index = u64::from_le_bytes(bytes);
            if index >= count || directories.last().is_some_and(|prev| *prev >= index) {
                return Err(invalid("parent cache invalid directory index"));
            }
            directories.push(index);
        }
        if body_hash.finalize().as_slice() != stored_body_digest {
            return Err(invalid("parent cache checksum mismatch"));
        }
        Ok(Some(Self {
            children,
            directories,
        }))
    }

    fn save(&self, path: &Path, base_digest: &[u8; 32]) -> io::Result<()> {
        let count = self.children.len() as u64;
        if count > MAX_ENTRIES || self.directories.len() > self.children.len() {
            return Err(invalid("parent cache exceeds size limit"));
        }
        let target = cache_path(path);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut suffix = target.as_os_str().to_os_string();
        suffix.push(format!(".{}.{}.tmp", std::process::id(), nonce));
        let staging = PathBuf::from(suffix);
        let result = (|| {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&staging)?;
            let mut writer = BufWriter::with_capacity(512 * 1024, file);
            writer.write_all(&MAGIC)?;
            writer.write_all(&VERSION.to_le_bytes())?;
            writer.write_all(&count.to_le_bytes())?;
            writer.write_all(&(self.directories.len() as u64).to_le_bytes())?;
            writer.write_all(base_digest)?;
            let mut body_hash = Sha256::new();
            for (parent, row) in &self.children {
                body_hash.update(parent.to_le_bytes());
                body_hash.update(row.to_le_bytes());
            }
            for row in &self.directories {
                body_hash.update(row.to_le_bytes());
            }
            writer.write_all(&body_hash.finalize())?;
            for (parent, row) in &self.children {
                writer.write_all(&parent.to_le_bytes())?;
                writer.write_all(&row.to_le_bytes())?;
            }
            for row in &self.directories {
                writer.write_all(&row.to_le_bytes())?;
            }
            writer.flush()?;
            writer.get_ref().sync_all()?;
            drop(writer);
            // Windows rename cannot atomically replace an existing open
            // cache. A brief missing cache is safe: readers rebuild instead.
            #[cfg(windows)]
            if target.exists() {
                fs::remove_file(&target)?;
            }
            fs::rename(&staging, &target)?;
            #[cfg(unix)]
            if let Some(dir) = target.parent() {
                File::open(dir)?.sync_all()?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(staging);
        }
        result
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn read_u32(reader: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(reader: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

pub(crate) fn cache_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".parents");
    PathBuf::from(name)
}

pub(crate) fn fingerprint(path: &Path) -> io::Result<[u8; 32]> {
    let mut reader = BufReader::with_capacity(512 * 1024, File::open(path)?);
    let mut digest = Sha256::new();
    let mut block = [0u8; 128 * 1024];
    loop {
        let count = reader.read(&mut block)?;
        if count == 0 {
            break;
        }
        digest.update(&block[..count]);
    }
    Ok(digest.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{BuildOptions, IndexBuilder, InputRecord};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "search-parent-cache-{}-{nonce}.stidx",
            std::process::id()
        ));
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (id, parent, name, flags) in [
            (1, 1, "C:", FLAG_DIRECTORY),
            (2, 1, "Private", FLAG_DIRECTORY),
            (3, 2, "report.txt", 0),
            (4, 1, "Outside", FLAG_DIRECTORY),
            (5, 4, "unrelated.txt", 0),
        ] {
            builder
                .push(InputRecord {
                    file_id: id,
                    parent_id: parent,
                    name,
                    flags,
                    size_bytes: 1,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        path
    }

    fn cleanup(path: &Path) {
        for suffix in ["", ".names", ".ids", ".ncp", ".icp", ".parents"] {
            let mut value = path.as_os_str().to_os_string();
            value.push(suffix);
            let _ = fs::remove_file(PathBuf::from(value));
        }
    }

    #[test]
    fn valid_parent_cache_roundtrips_without_new_base_scan() {
        let path = fixture();
        let mut store = SearchStore::open(&path).unwrap();
        let original = BaseParentIndex::load_or_build(&path, &mut store).unwrap();
        assert!(cache_path(&path).is_file());
        assert_eq!(original.children.len(), 5);
        assert_eq!(original.child_range(2).len(), 1);
        let from_disk =
            BaseParentIndex::load(&path, store.record_count(), &fingerprint(&path).unwrap())
                .unwrap()
                .unwrap();
        assert_eq!(from_disk.children, original.children);
        assert_eq!(from_disk.directories, original.directories);
        cleanup(&path);
    }

    #[test]
    fn corrupt_truncated_and_stale_cache_rebuilds() {
        let path = fixture();
        let mut store = SearchStore::open(&path).unwrap();
        BaseParentIndex::load_or_build(&path, &mut store).unwrap();
        let cached = cache_path(&path);
        let digest = fingerprint(&path).unwrap();
        assert!(BaseParentIndex::load(&path, 5, &[1; 32]).is_err());
        let mut bytes = fs::read(&cached).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x55;
        fs::write(&cached, &bytes).unwrap();
        assert!(BaseParentIndex::load(&path, 5, &digest).is_err());
        assert_eq!(
            BaseParentIndex::load_or_build(&path, &mut store)
                .unwrap()
                .children
                .len(),
            5
        );
        fs::write(&cached, b"STPARENT").unwrap();
        assert_eq!(
            BaseParentIndex::load_or_build(&path, &mut store)
                .unwrap()
                .children
                .len(),
            5
        );
        assert!(BaseParentIndex::load(&path, 5, &digest).unwrap().is_some());
        cleanup(&path);
    }

    #[test]
    fn changed_base_bytes_invalidate_cache_even_with_equal_length() {
        let path = fixture();
        let mut store = SearchStore::open(&path).unwrap();
        BaseParentIndex::load_or_build(&path, &mut store).unwrap();
        let old_hash = fingerprint(&path).unwrap();
        // Rebuild same-length index with a different parent relationship.
        let mut builder = IndexBuilder::create(&path, BuildOptions::default()).unwrap();
        for (id, parent, name, flags) in [
            (1, 1, "C:", FLAG_DIRECTORY),
            (2, 1, "Private", FLAG_DIRECTORY),
            (3, 4, "report.txt", 0),
            (4, 1, "Outside", FLAG_DIRECTORY),
            (5, 4, "unrelated.txt", 0),
        ] {
            builder
                .push(InputRecord {
                    file_id: id,
                    parent_id: parent,
                    name,
                    flags,
                    size_bytes: 1,
                })
                .unwrap();
        }
        builder.finish().unwrap();
        let fresh_hash = fingerprint(&path).unwrap();
        assert_ne!(old_hash, fresh_hash);
        assert!(BaseParentIndex::load(&path, 5, &fresh_hash).is_err());
        let mut fresh_store = SearchStore::open(&path).unwrap();
        let fresh = BaseParentIndex::load_or_build(&path, &mut fresh_store).unwrap();
        assert!(fresh.children.contains(&(4, 2)));
        assert!(BaseParentIndex::load(&path, 5, &fresh_hash)
            .unwrap()
            .is_some());
        cleanup(&path);
    }
}
