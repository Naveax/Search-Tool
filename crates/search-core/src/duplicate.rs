use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

const SAMPLE_BYTES: usize = 64 * 1024;
const COMPARE_BUFFER_BYTES: usize = 256 * 1024;

pub fn sample_fingerprint(path: impl AsRef<Path>, size: u64) -> io::Result<u64> {
    let mut file = File::open(path)?;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ size;
    let mut buffer = vec![0_u8; SAMPLE_BYTES];
    let first = file.read(&mut buffer)?;
    fnv_update(&mut hash, &buffer[..first]);
    if size > SAMPLE_BYTES as u64 {
        let start = size.saturating_sub(SAMPLE_BYTES as u64);
        file.seek(SeekFrom::Start(start))?;
        let last = file.read(&mut buffer)?;
        fnv_update(&mut hash, &buffer[..last]);
    }
    Ok(hash)
}

pub fn full_fingerprint(path: impl AsRef<Path>) -> io::Result<u64> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut input = BufReader::with_capacity(COMPARE_BUFFER_BYTES, file);
    let mut buffer = vec![0_u8; COMPARE_BUFFER_BYTES];
    let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ size.rotate_left(17);
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        fnv_update(&mut hash, &buffer[..read]);
    }
    Ok(hash)
}

pub fn exact_files_equal(left: impl AsRef<Path>, right: impl AsRef<Path>) -> io::Result<bool> {
    let left = File::open(left)?;
    let right = File::open(right)?;
    if left.metadata()?.len() != right.metadata()?.len() {
        return Ok(false);
    }
    let mut left = BufReader::with_capacity(COMPARE_BUFFER_BYTES, left);
    let mut right = BufReader::with_capacity(COMPARE_BUFFER_BYTES, right);
    let mut a = vec![0_u8; COMPARE_BUFFER_BYTES];
    let mut b = vec![0_u8; COMPARE_BUFFER_BYTES];
    loop {
        let an = left.read(&mut a)?;
        let bn = right.read(&mut b)?;
        if an != bn || a[..an] != b[..bn] {
            return Ok(false);
        }
        if an == 0 {
            return Ok(true);
        }
    }
}

fn fnv_update(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= *byte as u64;
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn exact_compare_is_collision_safe_final_gate() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let a = std::env::temp_dir().join(format!("st-dup-a-{n}"));
        let b = std::env::temp_dir().join(format!("st-dup-b-{n}"));
        let c = std::env::temp_dir().join(format!("st-dup-c-{n}"));
        fs::write(&a, b"same bytes").unwrap();
        fs::write(&b, b"same bytes").unwrap();
        fs::write(&c, b"other byte").unwrap();
        assert_eq!(full_fingerprint(&a).unwrap(), full_fingerprint(&b).unwrap());
        assert_ne!(full_fingerprint(&a).unwrap(), full_fingerprint(&c).unwrap());
        assert!(exact_files_equal(&a, &b).unwrap());
        assert!(!exact_files_equal(&a, &c).unwrap());
        let _ = fs::remove_file(a);
        let _ = fs::remove_file(b);
        let _ = fs::remove_file(c);
    }
}
