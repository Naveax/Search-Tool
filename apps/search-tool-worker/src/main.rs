use std::fs::File;
use std::io::{self, Read, Write};
#[cfg(windows)]
use std::path::Path;
use std::process::ExitCode;

const DEFAULT_MAX_CHARS: usize = 2 * 1024 * 1024;
const MAX_TEXT_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_DOCUMENT_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FALLBACK_XML_BYTES: usize = 16 * 1024 * 1024;

fn main() -> ExitCode {
    let mut args = std::env::args();
    let _ = args.next();
    match args.next().as_deref() {
        Some("extract") => {
            let Some(path) = args.next() else {
                eprintln!("usage: search-tool-worker extract PATH [MAX_CHARS]");
                return ExitCode::from(2);
            };
            let max_chars = args
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(DEFAULT_MAX_CHARS)
                .clamp(1, DEFAULT_MAX_CHARS);
            match extract(&path, max_chars) {
                Ok(text) => {
                    if io::stdout().write_all(text.as_bytes()).is_ok() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(error) => {
                    eprintln!("extract failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("preview") => {
            let Some(path) = args.next() else {
                eprintln!("usage: search-tool-worker preview PATH");
                return ExitCode::from(2);
            };
            match extract(&path, 64 * 1024) {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("preview failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("serve") => match serve() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("worker server failed: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("Search Tool isolated parser/preview worker");
            eprintln!("  search-tool-worker extract PATH [MAX_CHARS]");
            eprintln!("  search-tool-worker preview PATH");
            eprintln!("  search-tool-worker serve");
            ExitCode::from(2)
        }
    }
}

fn serve() -> io::Result<()> {
    const MAX_PATH_BYTES: usize = 32 * 1024;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    loop {
        let Some(path_len) = read_u32_optional(&mut input)? else {
            return Ok(());
        };
        if path_len == 0 {
            return Ok(());
        }
        let path_len = path_len as usize;
        if path_len > MAX_PATH_BYTES {
            write_response(&mut output, 1, b"path exceeds worker protocol limit")?;
            continue;
        }
        let max_chars = read_u32(&mut input)? as usize;
        let mut path = vec![0_u8; path_len];
        input.read_exact(&mut path)?;
        let path = match String::from_utf8(path) {
            Ok(path) => path,
            Err(_) => {
                write_response(&mut output, 1, b"path is not UTF-8")?;
                continue;
            }
        };
        match extract(&path, max_chars.clamp(1, DEFAULT_MAX_CHARS)) {
            Ok(text) => write_response(&mut output, 0, text.as_bytes())?,
            Err(error) => write_response(&mut output, 1, error.to_string().as_bytes())?,
        }
    }
}

fn write_response<W: Write>(out: &mut W, status: u32, payload: &[u8]) -> io::Result<()> {
    let payload = &payload[..payload.len().min(u32::MAX as usize)];
    out.write_all(&status.to_le_bytes())?;
    out.write_all(&(payload.len() as u32).to_le_bytes())?;
    out.write_all(payload)?;
    out.flush()
}

fn read_u32<R: Read>(input: &mut R) -> io::Result<u32> {
    let mut bytes = [0_u8; 4];
    input.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u32_optional<R: Read>(input: &mut R) -> io::Result<Option<u32>> {
    let mut bytes = [0_u8; 4];
    match input.read_exact(&mut bytes) {
        Ok(()) => Ok(Some(u32::from_le_bytes(bytes))),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error),
    }
}

fn extract(path: &str, max_chars: usize) -> io::Result<String> {
    if search_core::is_text_candidate(path) {
        return extract_plain_text(path, max_chars);
    }
    extract_document(path, max_chars)
}

fn extract_plain_text(path: &str, max_chars: usize) -> io::Result<String> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    if size > MAX_TEXT_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "plain-text preview exceeds worker byte limit",
        ));
    }
    let mut bytes = Vec::with_capacity(size as usize);
    file.take(MAX_TEXT_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(text.chars().take(max_chars).collect())
}

#[cfg(windows)]
fn extract_document(path: &str, max_chars: usize) -> io::Result<String> {
    let path = Path::new(path);
    if !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "document not found",
        ));
    }
    if path.metadata()?.len() > MAX_DOCUMENT_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "document exceeds worker byte limit",
        ));
    }

    let ifilter =
        search_platform_windows::extract_filter_text(path.to_string_lossy().as_ref(), max_chars);
    if let Ok(text) = &ifilter {
        if !text.trim().is_empty() {
            return Ok(text.chars().take(max_chars).collect());
        }
    }

    match extract_document_fallback(path, max_chars) {
        Ok(text) => Ok(text),
        Err(fallback_error) => match ifilter {
            Err(ifilter_error) => Err(io::Error::other(format!(
                "IFilter failed: {ifilter_error}; fallback failed: {fallback_error}"
            ))),
            Ok(_) => Err(fallback_error),
        },
    }
}

#[cfg(windows)]
fn extract_document_fallback(path: &Path, max_chars: usize) -> io::Result<String> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => extract_pdf_fallback(path, max_chars),
        "docx" | "xlsx" | "pptx" => extract_ooxml_fallback(path, &ext, max_chars),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no built-in fallback parser for this document type",
        )),
    }
}

#[cfg(windows)]
fn extract_pdf_fallback(path: &Path, max_chars: usize) -> io::Result<String> {
    let text = pdf_extract::extract_text(path)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    Ok(text.chars().take(max_chars).collect())
}

#[cfg(windows)]
fn extract_ooxml_fallback(path: &Path, ext: &str, max_chars: usize) -> io::Result<String> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let mut output = String::new();
    let mut byte_budget = MAX_FALLBACK_XML_BYTES;

    for index in 0..archive.len() {
        if byte_budget == 0 || output.chars().count() >= max_chars {
            break;
        }
        let mut entry = archive
            .by_index(index)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        let name = entry.name().replace('\\', "/");
        if !ooxml_text_part(ext, &name) {
            continue;
        }
        let entry_limit = byte_budget.min(entry.size().min(usize::MAX as u64) as usize);
        let mut bytes = Vec::with_capacity(entry_limit.min(256 * 1024));
        entry
            .by_ref()
            .take(entry_limit as u64)
            .read_to_end(&mut bytes)?;
        byte_budget = byte_budget.saturating_sub(bytes.len());
        let xml = String::from_utf8_lossy(&bytes);
        append_xml_visible_text(&xml, &mut output, max_chars);
    }

    if output.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "OOXML fallback produced no visible text",
        ));
    }
    Ok(output)
}

#[cfg(windows)]
fn ooxml_text_part(ext: &str, name: &str) -> bool {
    match ext {
        "docx" => {
            name == "word/document.xml"
                || (name.starts_with("word/header") && name.ends_with(".xml"))
                || (name.starts_with("word/footer") && name.ends_with(".xml"))
                || matches!(
                    name,
                    "word/footnotes.xml" | "word/endnotes.xml" | "word/comments.xml"
                )
        }
        "xlsx" => {
            name == "xl/workbook.xml"
                || name == "xl/sharedStrings.xml"
                || (name.starts_with("xl/worksheets/") && name.ends_with(".xml"))
        }
        "pptx" => {
            (name.starts_with("ppt/slides/slide") && name.ends_with(".xml"))
                || (name.starts_with("ppt/notesSlides/notesSlide") && name.ends_with(".xml"))
        }
        _ => false,
    }
}

#[cfg(windows)]
fn append_xml_visible_text(xml: &str, output: &mut String, max_chars: usize) {
    let mut in_tag = false;
    let mut segment = String::new();

    for ch in xml.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
            }
            continue;
        }
        if ch == '<' {
            append_xml_segment(&segment, output, max_chars);
            segment.clear();
            in_tag = true;
            if output.chars().count() >= max_chars {
                break;
            }
        } else {
            segment.push(ch);
        }
    }
    append_xml_segment(&segment, output, max_chars);
}

#[cfg(windows)]
fn append_xml_segment(segment: &str, output: &mut String, max_chars: usize) {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        return;
    }
    let decoded = trimmed
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    let used = output.chars().count();
    if used >= max_chars {
        return;
    }
    if !output.is_empty() {
        output.push(' ');
    }
    let remaining = max_chars.saturating_sub(output.chars().count());
    output.extend(decoded.chars().take(remaining));
}

#[cfg(not(windows))]
fn extract_document(_path: &str, _max_chars: usize) -> io::Result<String> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "document IFilter extraction is only available on Windows",
    ))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn xml_visible_text_is_bounded_and_decodes_entities() {
        let mut output = String::new();
        append_xml_visible_text(
            "<root><t>Hello &amp; goodbye</t><t>&lt;tag&gt;</t></root>",
            &mut output,
            64,
        );
        assert_eq!(output, "Hello & goodbye <tag>");
    }

    #[test]
    fn ooxml_text_parts_are_scoped_to_document_content() {
        assert!(ooxml_text_part("docx", "word/document.xml"));
        assert!(ooxml_text_part("xlsx", "xl/worksheets/sheet1.xml"));
        assert!(ooxml_text_part("pptx", "ppt/slides/slide1.xml"));
        assert!(!ooxml_text_part("docx", "word/styles.xml"));
        assert!(!ooxml_text_part("xlsx", "docProps/core.xml"));
    }
}
