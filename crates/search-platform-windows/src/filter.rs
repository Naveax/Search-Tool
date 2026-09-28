use std::{ffi::c_void, io, ptr::null_mut};

const S_OK: i32 = 0;
const S_FALSE: i32 = 1;
const FILTER_E_END_OF_CHUNKS: i32 = 0x8004_1700_u32 as i32;
const FILTER_E_NO_MORE_TEXT: i32 = 0x8004_1701_u32 as i32;
const FILTER_E_NO_TEXT: i32 = 0x8004_1705_u32 as i32;
const FILTER_S_LAST_TEXT: i32 = 0x0004_1709;
const CHUNK_TEXT: u32 = 0x1;
const IFILTER_INIT_CANON_PARAGRAPHS: u32 = 1;
const IFILTER_INIT_HARD_LINE_BREAKS: u32 = 2;
const IFILTER_INIT_CANON_HYPHENS: u32 = 4;
const IFILTER_INIT_CANON_SPACES: u32 = 8;
const IFILTER_INIT_INDEXING_ONLY: u32 = 64;
const COINIT_MULTITHREADED: u32 = 0;
const RPC_E_CHANGED_MODE: i32 = 0x8001_0106_u32 as i32;
const TEXT_BUFFER_CHARS: usize = 4096;

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
union PropSpecValue {
    propid: u32,
    lpwstr: *mut u16,
}

#[repr(C)]
struct PropSpec {
    kind: u32,
    value: PropSpecValue,
}

#[repr(C)]
struct FullPropSpec {
    guid_prop_set: Guid,
    property: PropSpec,
}

#[repr(C)]
struct StatChunk {
    id_chunk: u32,
    break_type: u32,
    flags: u32,
    locale: u32,
    attribute: FullPropSpec,
    id_chunk_source: u32,
    cwc_start_source: u32,
    cwc_len_source: u32,
}

#[repr(C)]
struct IFilter {
    vtable: *const IFilterVtable,
}

#[repr(C)]
struct IFilterVtable {
    query_interface: unsafe extern "system" fn(*mut IFilter, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut IFilter) -> u32,
    release: unsafe extern "system" fn(*mut IFilter) -> u32,
    init: unsafe extern "system" fn(*mut IFilter, u32, u32, *const FullPropSpec, *mut u32) -> i32,
    get_chunk: unsafe extern "system" fn(*mut IFilter, *mut StatChunk) -> i32,
    get_text: unsafe extern "system" fn(*mut IFilter, *mut u32, *mut u16) -> i32,
    get_value: unsafe extern "system" fn(*mut IFilter, *mut *mut c_void) -> i32,
    bind_region: unsafe extern "system" fn(
        *mut IFilter,
        *const c_void,
        *const Guid,
        *mut *mut c_void,
    ) -> i32,
}

#[link(name = "query")]
extern "system" {
    #[link_name = "LoadIFilter"]
    fn load_i_filter(path: *const u16, outer: *mut c_void, filter: *mut *mut IFilter) -> i32;
}

#[link(name = "ole32")]
extern "system" {
    #[link_name = "CoInitializeEx"]
    fn co_initialize_ex(reserved: *mut c_void, coinit: u32) -> i32;
    #[link_name = "CoUninitialize"]
    fn co_uninitialize();
}

struct FilterPtr(*mut IFilter);

impl FilterPtr {
    fn load(path: &str) -> io::Result<Self> {
        let path = wide(path);
        let mut filter = null_mut();
        let hr = unsafe { load_i_filter(path.as_ptr(), null_mut(), &mut filter) };
        if failed(hr) || filter.is_null() {
            return Err(hresult_error("LoadIFilter failed", hr));
        }
        Ok(Self(filter))
    }

    fn vtable(&self) -> &IFilterVtable {
        unsafe { &*(*self.0).vtable }
    }
}

impl Drop for FilterPtr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                (self.vtable().release)(self.0);
            }
        }
    }
}

struct ComApartment(bool);
impl ComApartment {
    fn initialize() -> io::Result<Self> {
        let hr = unsafe { co_initialize_ex(null_mut(), COINIT_MULTITHREADED) };
        if hr == S_OK || hr == S_FALSE {
            Ok(Self(true))
        } else if hr == RPC_E_CHANGED_MODE {
            Ok(Self(false))
        } else {
            Err(hresult_error("CoInitializeEx failed", hr))
        }
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { co_uninitialize() };
        }
    }
}

pub fn extract_filter_text(path: &str, max_chars: usize) -> io::Result<String> {
    if max_chars == 0 {
        return Ok(String::new());
    }
    let _com = ComApartment::initialize()?;
    let filter = FilterPtr::load(path)?;
    let mut out_flags = 0_u32;
    let init_flags = IFILTER_INIT_CANON_PARAGRAPHS
        | IFILTER_INIT_HARD_LINE_BREAKS
        | IFILTER_INIT_CANON_HYPHENS
        | IFILTER_INIT_CANON_SPACES
        | IFILTER_INIT_INDEXING_ONLY;
    let hr = unsafe { (filter.vtable().init)(filter.0, init_flags, 0, null_mut(), &mut out_flags) };
    if failed(hr) {
        return Err(hresult_error("IFilter::Init failed", hr));
    }

    let mut output = String::with_capacity(max_chars.min(64 * 1024));
    loop {
        let mut chunk: StatChunk = unsafe { std::mem::zeroed() };
        let hr = unsafe { (filter.vtable().get_chunk)(filter.0, &mut chunk) };
        if hr == FILTER_E_END_OF_CHUNKS {
            break;
        }
        if failed(hr) {
            return Err(hresult_error("IFilter::GetChunk failed", hr));
        }
        if chunk.flags & CHUNK_TEXT == 0 {
            continue;
        }

        let mut at_chunk_start = true;
        loop {
            let needs_separator = at_chunk_start
                && !output.is_empty()
                && output.chars().last().is_some_and(|ch| !ch.is_whitespace());
            let remaining = max_chars.saturating_sub(output.chars().count());
            let text_budget = remaining.saturating_sub(usize::from(needs_separator));
            if text_budget == 0 {
                return Ok(output);
            }
            let mut buffer = [0_u16; TEXT_BUFFER_CHARS];
            let mut chars = buffer.len().min(text_budget) as u32;
            let hr =
                unsafe { (filter.vtable().get_text)(filter.0, &mut chars, buffer.as_mut_ptr()) };
            if hr == FILTER_E_NO_MORE_TEXT || hr == FILTER_E_NO_TEXT {
                break;
            }
            if failed(hr) {
                return Err(hresult_error("IFilter::GetText failed", hr));
            }
            let count = (chars as usize).min(buffer.len());
            if count != 0 {
                if needs_separator {
                    output.push(' ');
                }
                output.push_str(&String::from_utf16_lossy(&buffer[..count]));
                at_chunk_start = false;
            }
            if hr == FILTER_S_LAST_TEXT {
                break;
            }
        }
    }
    Ok(output)
}

fn failed(hr: i32) -> bool {
    hr < 0
}

fn hresult_error(context: &'static str, hr: i32) -> io::Error {
    io::Error::other(format!("{context}: HRESULT 0x{:08X}", hr as u32))
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
