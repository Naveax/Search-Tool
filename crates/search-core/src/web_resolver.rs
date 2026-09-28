use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const CACHE_MAGIC: [u8; 8] = *b"STWEB\0\0\0";
const CACHE_VERSION: u16 = 1;
const CACHE_HEADER_SIZE: u64 = 32;
pub const DEFAULT_WEB_CACHE_LIMIT_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebLookupQuery {
    pub filename: String,
    pub publisher: Option<String>,
    pub product: Option<String>,
    pub version: Option<String>,
}

impl WebLookupQuery {
    pub fn sanitized(filename_or_path: &str) -> Self {
        let filename = filename_or_path
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(filename_or_path)
            .trim()
            .chars()
            .take(260)
            .collect();
        Self {
            filename,
            publisher: None,
            product: None,
            version: None,
        }
    }

    pub fn search_text(&self) -> String {
        let mut parts = vec![self.filename.clone()];
        if let Some(value) = &self.publisher {
            if !value.is_empty() {
                parts.push(value.clone());
            }
        }
        if let Some(value) = &self.product {
            if !value.is_empty() {
                parts.push(value.clone());
            }
        }
        if let Some(value) = &self.version {
            if !value.is_empty() {
                parts.push(value.clone());
            }
        }
        parts.join(" ")
    }

    pub fn cache_key(&self) -> u64 {
        fnv1a(self.search_text().as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
}

pub fn parse_google_custom_search_json(json: &str, limit: usize) -> Vec<WebResult> {
    let Some(items_at) = json.find("\"items\"") else {
        return Vec::new();
    };
    let mut rest = &json[items_at..];
    let mut results = Vec::new();
    while results.len() < limit {
        let Some(title_at) = rest.find("\"title\"") else {
            break;
        };
        rest = &rest[title_at..];
        let Some((title, after_title)) = extract_json_field(rest, "title") else {
            break;
        };
        let Some((link, after_link)) = extract_json_field(after_title, "link") else {
            break;
        };
        let Some((snippet, after_snippet)) = extract_json_field(after_link, "snippet") else {
            break;
        };
        results.push(WebResult {
            title,
            link,
            snippet,
        });
        rest = after_snippet;
    }
    results
}

#[derive(Debug)]
pub struct WebCache {
    path: PathBuf,
    max_bytes: u64,
}
impl WebCache {
    pub fn new(path: impl AsRef<Path>, max_bytes: u64) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            max_bytes: max_bytes.max(CACHE_HEADER_SIZE),
        }
    }

    pub fn get(&self, key: u64) -> io::Result<Option<Vec<WebResult>>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let mut input = BufReader::new(File::open(&self.path)?);
        validate_cache_header(&mut input)?;
        let mut found = None;
        while let Some(record_key) = read_optional_u64(&mut input)? {
            let count = read_u16(&mut input)? as usize;
            let _ = read_u16(&mut input)?;
            let mut results = Vec::with_capacity(count);
            for _ in 0..count {
                results.push(WebResult {
                    title: read_string(&mut input, 16 * 1024)?,
                    link: read_string(&mut input, 16 * 1024)?,
                    snippet: read_string(&mut input, 64 * 1024)?,
                });
            }
            if record_key == key {
                found = Some(results);
            }
        }
        Ok(found)
    }

    pub fn put(&self, key: u64, results: &[WebResult]) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !self.path.exists() {
            let mut file = File::create(&self.path)?;
            write_cache_header(&mut file)?;
            file.sync_all()?;
        }
        if fs::metadata(&self.path)?.len() > self.max_bytes {
            let mut file = File::create(&self.path)?;
            write_cache_header(&mut file)?;
        }
        let file = OpenOptions::new().append(true).open(&self.path)?;
        let mut out = BufWriter::new(file);
        out.write_all(&key.to_le_bytes())?;
        out.write_all(&(results.len().min(u16::MAX as usize) as u16).to_le_bytes())?;
        out.write_all(&0_u16.to_le_bytes())?;
        for result in results.iter().take(u16::MAX as usize) {
            write_string(&mut out, &result.title)?;
            write_string(&mut out, &result.link)?;
            write_string(&mut out, &result.snippet)?;
        }
        out.flush()?;
        out.get_ref().sync_all()
    }
}

pub fn web_cache_path(index_path: impl AsRef<Path>) -> PathBuf {
    let mut value = index_path.as_ref().as_os_str().to_os_string();
    value.push(".webcache");
    PathBuf::from(value)
}

fn extract_json_field<'a>(input: &'a str, field: &str) -> Option<(String, &'a str)> {
    let pattern = format!("\"{field}\"");
    let key = input.find(&pattern)?;
    let after_key = &input[key + pattern.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let (value, consumed) = parse_json_string(after_colon)?;
    Some((value, &after_colon[consumed..]))
}

fn parse_json_string(input: &str) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    if bytes.first().copied()? != b'"' {
        return None;
    }
    let mut out = String::new();
    let mut i = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Some((out, i + 1)),
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    return None;
                }
                match bytes[i] {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        if i + 4 >= bytes.len() {
                            return None;
                        }
                        let hex = &input[i + 1..i + 5];
                        let value = u16::from_str_radix(hex, 16).ok()?;
                        out.push(
                            char::from_u32(value as u32).unwrap_or(char::REPLACEMENT_CHARACTER),
                        );
                        i += 4;
                    }
                    _ => return None,
                }
            }
            _ => {
                let tail = &input[i..];
                let ch = tail.chars().next()?;
                out.push(ch);
                i += ch.len_utf8() - 1;
            }
        }
        i += 1;
    }
    None
}

fn write_cache_header<W: Write>(out: &mut W) -> io::Result<()> {
    out.write_all(&CACHE_MAGIC)?;
    out.write_all(&CACHE_VERSION.to_le_bytes())?;
    out.write_all(&0_u16.to_le_bytes())?;
    out.write_all(&[0_u8; 20])
}
fn validate_cache_header<R: Read>(input: &mut R) -> io::Result<()> {
    let mut magic = [0; 8];
    input.read_exact(&mut magic)?;
    if magic != CACHE_MAGIC {
        return Err(invalid("invalid web cache magic"));
    }
    if read_u16(input)? != CACHE_VERSION {
        return Err(invalid("unsupported web cache version"));
    }
    let _ = read_u16(input)?;
    let mut reserved = [0; 20];
    input.read_exact(&mut reserved)?;
    Ok(())
}
fn write_string<W: Write>(out: &mut W, value: &str) -> io::Result<()> {
    let b = value.as_bytes();
    out.write_all(&(b.len() as u32).to_le_bytes())?;
    out.write_all(b)
}
fn read_string<R: Read>(input: &mut R, cap: usize) -> io::Result<String> {
    let len = read_u32(input)? as usize;
    if len > cap {
        return Err(invalid("web cache string too large"));
    }
    let mut b = vec![0; len];
    input.read_exact(&mut b)?;
    String::from_utf8(b).map_err(|_| invalid("web cache string invalid UTF-8"))
}
fn read_optional_u64<R: Read>(input: &mut R) -> io::Result<Option<u64>> {
    let mut first = [0; 1];
    match input.read_exact(&mut first) {
        Ok(()) => {
            let mut rest = [0; 7];
            input.read_exact(&mut rest)?;
            let mut b = [0; 8];
            b[0] = first[0];
            b[1..].copy_from_slice(&rest);
            Ok(Some(u64::from_le_bytes(b)))
        }
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(e),
    }
}
fn read_u16<R: Read>(r: &mut R) -> io::Result<u16> {
    let mut b = [0; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}
fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut b = [0; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
fn invalid(m: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn sanitizer_never_leaks_parent_path() {
        let q = WebLookupQuery::sanitized(r"C:\Users\Secret Name\Desktop\thing.dll");
        assert_eq!(q.filename, "thing.dll");
        assert!(!q.search_text().contains("Secret"));
    }
    #[test]
    fn parses_google_items_without_json_dependency() {
        let json = r#"{"items":[{"title":"Thing \"DLL\"","link":"https://example.com/a","snippet":"Useful component"},{"title":"Second","link":"https://example.com/b","snippet":"Other"}]}"#;
        let r = parse_google_custom_search_json(json, 5);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].title, "Thing \"DLL\"");
    }
    #[test]
    fn bounded_cache_roundtrip() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("stweb-{n}"));
        let c = WebCache::new(&path, 1024 * 1024);
        let data = vec![WebResult {
            title: "A".into(),
            link: "https://a".into(),
            snippet: "B".into(),
        }];
        c.put(7, &data).unwrap();
        assert_eq!(c.get(7).unwrap(), Some(data));
        let _ = fs::remove_file(path);
    }
}
