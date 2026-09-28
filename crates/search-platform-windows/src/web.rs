use std::{ffi::c_void, io, ptr::null_mut};

const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000;
const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
const HTTPS_PORT: u16 = 443;
const DEFAULT_RESPONSE_LIMIT: usize = 1024 * 1024;
const IO_CHUNK_BYTES: usize = 32 * 1024;

type HInternet = *mut c_void;

#[link(name = "winhttp")]
extern "system" {
    #[link_name = "WinHttpOpen"]
    fn win_http_open(
        user_agent: *const u16,
        access_type: u32,
        proxy_name: *const u16,
        proxy_bypass: *const u16,
        flags: u32,
    ) -> HInternet;

    #[link_name = "WinHttpConnect"]
    fn win_http_connect(
        session: HInternet,
        server_name: *const u16,
        server_port: u16,
        reserved: u32,
    ) -> HInternet;

    #[link_name = "WinHttpOpenRequest"]
    fn win_http_open_request(
        connect: HInternet,
        verb: *const u16,
        object_name: *const u16,
        version: *const u16,
        referrer: *const u16,
        accept_types: *const *const u16,
        flags: u32,
    ) -> HInternet;

    #[link_name = "WinHttpSetTimeouts"]
    fn win_http_set_timeouts(
        handle: HInternet,
        resolve_timeout: i32,
        connect_timeout: i32,
        send_timeout: i32,
        receive_timeout: i32,
    ) -> i32;

    #[link_name = "WinHttpSendRequest"]
    fn win_http_send_request(
        request: HInternet,
        headers: *const u16,
        headers_length: u32,
        optional: *mut c_void,
        optional_length: u32,
        total_length: u32,
        context: usize,
    ) -> i32;

    #[link_name = "WinHttpReceiveResponse"]
    fn win_http_receive_response(request: HInternet, reserved: *mut c_void) -> i32;

    #[link_name = "WinHttpQueryHeaders"]
    fn win_http_query_headers(
        request: HInternet,
        info_level: u32,
        name: *const u16,
        buffer: *mut c_void,
        buffer_length: *mut u32,
        index: *mut u32,
    ) -> i32;

    #[link_name = "WinHttpReadData"]
    fn win_http_read_data(
        request: HInternet,
        buffer: *mut c_void,
        bytes_to_read: u32,
        bytes_read: *mut u32,
    ) -> i32;

    #[link_name = "WinHttpCloseHandle"]
    fn win_http_close_handle(handle: HInternet) -> i32;
}

#[derive(Debug)]
struct InternetHandle(HInternet);

impl InternetHandle {
    fn new(raw: HInternet) -> io::Result<Self> {
        if raw.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }

    const fn raw(&self) -> HInternet {
        self.0
    }
}

impl Drop for InternetHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = win_http_close_handle(self.0);
            }
        }
    }
}

pub fn google_custom_search_json(
    api_key: &str,
    search_engine_id: &str,
    query: &str,
    max_results: usize,
) -> io::Result<String> {
    if api_key.trim().is_empty() || search_engine_id.trim().is_empty() || query.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Google Custom Search key, engine id, and query must be non-empty",
        ));
    }

    let count = max_results.clamp(1, 10);
    let path = format!(
        "/customsearch/v1?key={}&cx={}&q={}&num={count}",
        percent_encode(api_key),
        percent_encode(search_engine_id),
        percent_encode(query)
    );
    https_get("www.googleapis.com", &path, DEFAULT_RESPONSE_LIMIT)
}

fn https_get(host: &str, path: &str, max_bytes: usize) -> io::Result<String> {
    let agent = wide("SearchTool/0.1");
    let host = wide(host);
    let verb = wide("GET");
    let path = wide(path);

    let session = InternetHandle::new(unsafe {
        win_http_open(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            null_mut(),
            null_mut(),
            0,
        )
    })?;
    bool_result(unsafe { win_http_set_timeouts(session.raw(), 3000, 5000, 5000, 8000) })?;

    let connect = InternetHandle::new(unsafe {
        win_http_connect(session.raw(), host.as_ptr(), HTTPS_PORT, 0)
    })?;
    let request = InternetHandle::new(unsafe {
        win_http_open_request(
            connect.raw(),
            verb.as_ptr(),
            path.as_ptr(),
            null_mut(),
            null_mut(),
            null_mut(),
            WINHTTP_FLAG_SECURE,
        )
    })?;

    bool_result(unsafe {
        win_http_send_request(request.raw(), null_mut(), 0, null_mut(), 0, 0, 0)
    })?;
    bool_result(unsafe { win_http_receive_response(request.raw(), null_mut()) })?;

    let status = response_status(request.raw())?;
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
            win_http_read_data(
                request.raw(),
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

fn response_status(request: HInternet) -> io::Result<u32> {
    let mut status = 0_u32;
    let mut bytes = std::mem::size_of::<u32>() as u32;
    bool_result(unsafe {
        win_http_query_headers(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            null_mut(),
            (&mut status as *mut u32).cast(),
            &mut bytes,
            null_mut(),
        )
    })?;
    Ok(status)
}

fn bool_result(value: i32) -> io::Result<()> {
    if value == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
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

#[cfg(test)]
mod tests {
    use super::percent_encode;

    #[test]
    fn encodes_query_components_without_leaking_spaces() {
        assert_eq!(percent_encode("node js/test"), "node%20js%2Ftest");
        assert_eq!(percent_encode("abc-_.~XYZ09"), "abc-_.~XYZ09");
    }
}
