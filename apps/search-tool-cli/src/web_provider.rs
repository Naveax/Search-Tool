use search_core::WebResult;
use std::io;

const HTTP_PORT: u16 = 80;
const HTTPS_PORT: u16 = 443;
#[cfg(windows)]
const DEFAULT_RESPONSE_LIMIT: usize = 1024 * 1024;

pub fn parse_searxng_json(json: &str, limit: usize) -> Vec<WebResult> {
    let Some(results_at) = json.find("\"results\"") else {
        return Vec::new();
    };
    let mut rest = &json[results_at..];
    let mut results = Vec::new();
    while results.len() < limit {
        let Some(title_at) = rest.find("\"title\"") else {
            break;
        };
        rest = &rest[title_at..];
        let Some((title, after_title)) = extract_json_field(rest, "title") else {
            break;
        };
        let Some((link, after_link)) = extract_json_field(after_title, "url") else {
            break;
        };
        let (snippet, after_result) = match extract_json_field(after_link, "content") {
            Some((value, after_content)) => (value, after_content),
            None => (String::new(), after_link),
        };
        results.push(WebResult {
            title,
            link,
            snippet,
        });
        rest = after_result;
    }
    results
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpEndpoint {
    secure: bool,
    host: String,
    port: u16,
    base_path: String,
}
fn parse_http_endpoint(endpoint: &str) -> io::Result<HttpEndpoint> {
    let endpoint = endpoint.trim().trim_end_matches('/');
    let (secure, rest, default_port) = if let Some(rest) = endpoint.strip_prefix("https://") {
        (true, rest, HTTPS_PORT)
    } else if let Some(rest) = endpoint.strip_prefix("http://") {
        (false, rest, HTTP_PORT)
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SearXNG endpoint must start with http:// or https://",
        ));
    };

    if rest.is_empty() || rest.contains(['?', '#']) || rest.contains('@') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SearXNG endpoint contains unsupported URL components",
        ));
    }

    let (authority, base_path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, ""),
    };
    if authority.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SearXNG endpoint host is empty",
        ));
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => {
            let port = port.parse::<u16>().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "SearXNG endpoint port is invalid",
                )
            })?;
            (host, port)
        }
        Some(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "IPv6 SearXNG endpoints are not supported",
            ));
        }
        None => (authority, default_port),
    };
    if host.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SearXNG endpoint host is empty",
        ));
    }
    if !secure && !host.eq_ignore_ascii_case("localhost") && host != "127.0.0.1" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "plain HTTP SearXNG endpoints are allowed only on localhost",
        ));
    }

    Ok(HttpEndpoint {
        secure,
        host: host.to_owned(),
        port,
        base_path: base_path.trim_end_matches('/').to_owned(),
    })
}

#[cfg(windows)]
pub fn searxng_search_json(endpoint: &str, query: &str, max_results: usize) -> io::Result<String> {
    if query.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SearXNG query must be non-empty",
        ));
    }
    let endpoint = parse_http_endpoint(endpoint)?;
    searxng_search_json_platform(&endpoint, query, max_results)
}

#[cfg(windows)]
fn searxng_search_json_platform(
    endpoint: &HttpEndpoint,
    query: &str,
    _max_results: usize,
) -> io::Result<String> {
    let path = format!(
        "{}/search?q={}&format=json&categories=general&language=all&safesearch=1&pageno=1",
        endpoint.base_path,
        percent_encode(query)
    );
    http_get(
        &endpoint.host,
        endpoint.port,
        endpoint.secure,
        &path,
        DEFAULT_RESPONSE_LIMIT,
    )
}
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    out
}

#[cfg(windows)]
mod winhttp {
    use std::{ffi::c_void, io, ptr::null_mut};

    pub(super) const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
    pub(super) const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000;
    pub(super) const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
    pub(super) const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
    pub(super) const IO_CHUNK_BYTES: usize = 32 * 1024;
    pub(super) type HInternet = *mut c_void;

    #[link(name = "winhttp")]
    extern "system" {
        #[link_name = "WinHttpOpen"]
        pub(super) fn open(
            user_agent: *const u16,
            access_type: u32,
            proxy_name: *const u16,
            proxy_bypass: *const u16,
            flags: u32,
        ) -> HInternet;

        #[link_name = "WinHttpConnect"]
        pub(super) fn connect(
            session: HInternet,
            server_name: *const u16,
            server_port: u16,
            reserved: u32,
        ) -> HInternet;

        #[link_name = "WinHttpOpenRequest"]
        pub(super) fn open_request(
            connect: HInternet,
            verb: *const u16,
            object_name: *const u16,
            version: *const u16,
            referrer: *const u16,
            accept_types: *const *const u16,
            flags: u32,
        ) -> HInternet;

        #[link_name = "WinHttpSetTimeouts"]
        pub(super) fn set_timeouts(
            handle: HInternet,
            resolve_timeout: i32,
            connect_timeout: i32,
            send_timeout: i32,
            receive_timeout: i32,
        ) -> i32;

        #[link_name = "WinHttpSendRequest"]
        pub(super) fn send_request(
            request: HInternet,
            headers: *const u16,
            headers_length: u32,
            optional: *mut c_void,
            optional_length: u32,
            total_length: u32,
            context: usize,
        ) -> i32;

        #[link_name = "WinHttpReceiveResponse"]
        pub(super) fn receive_response(request: HInternet, reserved: *mut c_void) -> i32;

        #[link_name = "WinHttpQueryHeaders"]
        pub(super) fn query_headers(
            request: HInternet,
            info_level: u32,
            name: *const u16,
            buffer: *mut c_void,
            buffer_length: *mut u32,
            index: *mut u32,
        ) -> i32;

        #[link_name = "WinHttpReadData"]
        pub(super) fn read_data(
            request: HInternet,
            buffer: *mut c_void,
            bytes_to_read: u32,
            bytes_read: *mut u32,
        ) -> i32;

        #[link_name = "WinHttpCloseHandle"]
        pub(super) fn close_handle(handle: HInternet) -> i32;
    }

    pub(super) struct InternetHandle(pub(super) HInternet);
    impl InternetHandle {
        pub(super) fn new(raw: HInternet) -> io::Result<Self> {
            if raw.is_null() {
                Err(io::Error::last_os_error())
            } else {
                Ok(Self(raw))
            }
        }
    }

    impl Drop for InternetHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let _ = close_handle(self.0);
                }
            }
        }
    }

    pub(super) fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub(super) fn bool_result(value: i32) -> io::Result<()> {
        if value == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub(super) fn null() -> *mut c_void {
        null_mut()
    }
}

#[cfg(windows)]
fn http_get(
    host: &str,
    port: u16,
    secure: bool,
    path: &str,
    max_bytes: usize,
) -> io::Result<String> {
    use winhttp::*;

    let agent = wide("SearchTool/0.1");
    let host = wide(host);
    let verb = wide("GET");
    let path = wide(path);

    let session = InternetHandle::new(unsafe {
        open(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    })?;
    bool_result(unsafe { set_timeouts(session.0, 3000, 5000, 5000, 8000) })?;

    let connect = InternetHandle::new(unsafe { connect(session.0, host.as_ptr(), port, 0) })?;
    let request = InternetHandle::new(unsafe {
        open_request(
            connect.0,
            verb.as_ptr(),
            path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            if secure { WINHTTP_FLAG_SECURE } else { 0 },
        )
    })?;

    bool_result(unsafe { send_request(request.0, std::ptr::null(), 0, null(), 0, 0, 0) })?;
    bool_result(unsafe { receive_response(request.0, null()) })?;

    let status = response_status(request.0)?;
    if !(200..300).contains(&status) {
        return Err(io::Error::other(format!(
            "HTTP request failed with status {status}"
        )));
    }

    let mut output = Vec::with_capacity(16 * 1024);
    let mut chunk = [0_u8; IO_CHUNK_BYTES];
    loop {
        let mut read = 0_u32;
        bool_result(unsafe {
            read_data(
                request.0,
                chunk.as_mut_ptr().cast(),
                chunk.len() as u32,
                &mut read,
            )
        })?;
        if read == 0 {
            break;
        }
        let read = read as usize;
        if output.len().saturating_add(read) > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "web response exceeded configured byte limit",
            ));
        }
        output.extend_from_slice(&chunk[..read]);
    }

    String::from_utf8(output)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "web response is not UTF-8"))
}

#[cfg(windows)]
fn response_status(request: winhttp::HInternet) -> io::Result<u32> {
    use winhttp::*;
    let mut status = 0_u32;
    let mut bytes = std::mem::size_of::<u32>() as u32;
    bool_result(unsafe {
        query_headers(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            std::ptr::null(),
            (&mut status as *mut u32).cast(),
            &mut bytes,
            std::ptr::null_mut(),
        )
    })?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_searxng_results() {
        let json = r#"{"results":[{"title":"PowerShell","url":"https://learn.microsoft.com/powershell","content":"Cross-platform shell"},{"title":"Second","url":"https://example.com/b","content":"Other"}]}"#;
        let results = parse_searxng_json(json, 5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "PowerShell");
        assert_eq!(results[0].link, "https://learn.microsoft.com/powershell");
        assert_eq!(results[0].snippet, "Cross-platform shell");
    }

    #[test]
    fn accepts_https_endpoint_with_base_path() {
        let endpoint = parse_http_endpoint("https://search.example.test/searx").unwrap();
        assert!(endpoint.secure);
        assert_eq!(endpoint.host, "search.example.test");
        assert_eq!(endpoint.port, HTTPS_PORT);
        assert_eq!(endpoint.base_path, "/searx");
    }

    #[test]
    fn plain_http_is_local_only() {
        assert!(parse_http_endpoint("http://127.0.0.1:8888").is_ok());
        assert!(parse_http_endpoint("http://localhost:8888").is_ok());
        assert!(parse_http_endpoint("http://search.example.test").is_err());
    }
    #[test]
    fn percent_encoding_is_query_safe() {
        assert_eq!(percent_encode("node js/test"), "node%20js%2Ftest");
        assert_eq!(percent_encode("abc-_.~XYZ09"), "abc-_.~XYZ09");
    }
}
