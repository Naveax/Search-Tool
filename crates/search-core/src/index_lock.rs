use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct IndexMutationGuard {
    _file: File,
}

impl IndexMutationGuard {
    pub fn try_acquire(index_path: impl AsRef<Path>) -> io::Result<Self> {
        let path = mutation_lock_path(index_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => io::Error::new(
                io::ErrorKind::WouldBlock,
                "index mutation is already in progress",
            ),
            std::fs::TryLockError::Error(error) => error,
        })?;
        Ok(Self { _file: file })
    }
}

pub fn mutation_lock_path(index_path: impl AsRef<Path>) -> PathBuf {
    let mut value = index_path.as_ref().as_os_str().to_os_string();
    value.push(".mutation.lock");
    PathBuf::from(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn mutation_lock_is_exclusive_and_released_on_drop() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let index = std::env::temp_dir().join(format!("search-tool-lock-{nonce}.stidx"));
        let first = IndexMutationGuard::try_acquire(&index).unwrap();
        let second = IndexMutationGuard::try_acquire(&index).unwrap_err();
        assert_eq!(second.kind(), io::ErrorKind::WouldBlock);
        drop(first);
        IndexMutationGuard::try_acquire(&index).unwrap();
        let _ = std::fs::remove_file(mutation_lock_path(index));
    }
}
