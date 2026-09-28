use std::{ffi::c_void, io, ptr::null_mut};

const GENERIC_READ: u32 = 0x8000_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const FILE_SHARE_DELETE: u32 = 0x0000_0004;
const OPEN_EXISTING: u32 = 3;
const ERROR_HANDLE_EOF: i32 = 38;
const ERROR_NO_MORE_FILES: i32 = 18;

const FILE_DEVICE_FILE_SYSTEM: u32 = 0x0000_0009;
const FILE_DEVICE_MASS_STORAGE: u32 = 0x0000_002d;
const METHOD_BUFFERED: u32 = 0;
const METHOD_NEITHER: u32 = 3;
const FILE_ANY_ACCESS: u32 = 0;

const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

const FSCTL_ENUM_USN_DATA: u32 =
    ctl_code(FILE_DEVICE_FILE_SYSTEM, 44, METHOD_NEITHER, FILE_ANY_ACCESS);
const FSCTL_READ_USN_JOURNAL: u32 =
    ctl_code(FILE_DEVICE_FILE_SYSTEM, 46, METHOD_NEITHER, FILE_ANY_ACCESS);
const FSCTL_QUERY_USN_JOURNAL: u32 = ctl_code(
    FILE_DEVICE_FILE_SYSTEM,
    61,
    METHOD_BUFFERED,
    FILE_ANY_ACCESS,
);
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = ctl_code(
    FILE_DEVICE_MASS_STORAGE,
    0x0500,
    METHOD_BUFFERED,
    FILE_ANY_ACCESS,
);
const STORAGE_DEVICE_SEEK_PENALTY_PROPERTY: u32 = 7;
const PROPERTY_STANDARD_QUERY: u32 = 0;

const ENUM_BUFFER_BYTES: usize = 256 * 1024;
const READ_BUFFER_BYTES: usize = 256 * 1024;
const USN_RECORD_V2_MIN_BYTES: usize = 60;
const USN_REASON_FILE_CREATE: u32 = 0x0000_0100;
const USN_REASON_FILE_DELETE: u32 = 0x0000_0200;
const USN_REASON_RENAME_OLD_NAME: u32 = 0x0000_1000;
const USN_REASON_RENAME_NEW_NAME: u32 = 0x0000_2000;
const USN_REASON_BASIC_INFO_CHANGE: u32 = 0x0000_8000;
const USN_REASON_HARD_LINK_CHANGE: u32 = 0x0001_0000;
const USN_REASON_REPARSE_POINT_CHANGE: u32 = 0x0010_0000;
const INDEX_RELEVANT_REASON_MASK: u32 = USN_REASON_FILE_CREATE
    | USN_REASON_FILE_DELETE
    | USN_REASON_RENAME_OLD_NAME
    | USN_REASON_RENAME_NEW_NAME
    | USN_REASON_BASIC_INFO_CHANGE
    | USN_REASON_HARD_LINK_CHANGE
    | USN_REASON_REPARSE_POINT_CHANGE;

#[repr(C)]
struct MftEnumDataV0 {
    start_file_reference_number: u64,
    low_usn: i64,
    high_usn: i64,
}

#[repr(C)]
struct ReadUsnJournalDataV0 {
    start_usn: i64,
    reason_mask: u32,
    return_only_on_close: u32,
    timeout: u64,
    bytes_to_wait_for: u64,
    usn_journal_id: u64,
}

#[repr(C)]
struct StoragePropertyQuery {
    property_id: u32,
    query_type: u32,
    additional_parameters: [u8; 1],
}

#[repr(C)]
struct DeviceSeekPenaltyDescriptor {
    version: u32,
    size: u32,
    incurs_seek_penalty: u8,
}

type Handle = *mut c_void;

#[link(name = "kernel32")]
extern "system" {
    #[link_name = "CreateFileW"]
    fn create_file_w(
        file_name: *const u16,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *mut c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: Handle,
    ) -> Handle;

    #[link_name = "DeviceIoControl"]
    fn device_io_control(
        device: Handle,
        control_code: u32,
        in_buffer: *mut c_void,
        in_buffer_size: u32,
        out_buffer: *mut c_void,
        out_buffer_size: u32,
        bytes_returned: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;

    #[link_name = "CloseHandle"]
    fn close_handle(object: Handle) -> i32;
}

const INVALID_HANDLE_VALUE: isize = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalInfo {
    pub journal_id: u64,
    pub first_usn: i64,
    pub next_usn: i64,
    pub lowest_valid_usn: i64,
    pub max_usn: i64,
    pub maximum_size: u64,
    pub allocation_delta: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalCheckpoint {
    pub journal_id: u64,
    pub next_usn: i64,
}

impl From<JournalInfo> for JournalCheckpoint {
    fn from(value: JournalInfo) -> Self {
        Self {
            journal_id: value.journal_id,
            next_usn: value.next_usn,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UsnRecordView<'a> {
    pub file_reference_number: u64,
    pub parent_file_reference_number: u64,
    pub usn: i64,
    pub reason: u32,
    pub file_attributes: u32,
    pub name_utf16_le: &'a [u8],
}

impl UsnRecordView<'_> {
    pub fn decode_name(&self) -> String {
        let words = self
            .name_utf16_le
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair));
        std::char::decode_utf16(words)
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }

    pub fn is_directory(&self) -> bool {
        self.file_attributes & 0x0000_0010 != 0
    }

    pub fn is_delete_event(&self) -> bool {
        self.reason & USN_REASON_FILE_DELETE != 0
    }

    pub fn is_rename_old_event(&self) -> bool {
        self.reason & USN_REASON_RENAME_OLD_NAME != 0
            && self.reason & USN_REASON_RENAME_NEW_NAME == 0
    }

    pub fn is_index_relevant(&self) -> bool {
        self.reason & INDEX_RELEVANT_REASON_MASK != 0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EnumerationStats {
    pub records: u64,
    pub batches: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JournalReadStats {
    pub records: u64,
    pub next_usn: i64,
}

#[derive(Debug)]
pub struct NtfsVolume {
    handle: Handle,
    drive_letter: char,
}

impl NtfsVolume {
    pub fn open_drive(drive_letter: char) -> io::Result<Self> {
        let drive_letter = drive_letter.to_ascii_uppercase();
        if !drive_letter.is_ascii_alphabetic() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "drive letter must be A-Z",
            ));
        }

        let path = format!(r"\\.\{}:", drive_letter);
        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0);

        let handle = unsafe {
            create_file_w(
                wide.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null_mut(),
                OPEN_EXISTING,
                0,
                null_mut(),
            )
        };

        if handle as isize == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            handle,
            drive_letter,
        })
    }

    pub const fn drive_letter(&self) -> char {
        self.drive_letter
    }

    pub fn query_journal(&self) -> io::Result<JournalInfo> {
        let mut output = [0_u8; 64];
        let bytes = self.device_io(
            FSCTL_QUERY_USN_JOURNAL,
            null_mut(),
            0,
            output.as_mut_ptr().cast(),
            output.len() as u32,
        )? as usize;

        if bytes < 56 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "USN journal response shorter than V0 header",
            ));
        }

        Ok(JournalInfo {
            journal_id: u64_at(&output, 0)?,
            first_usn: i64_at(&output, 8)?,
            next_usn: i64_at(&output, 16)?,
            lowest_valid_usn: i64_at(&output, 24)?,
            max_usn: i64_at(&output, 32)?,
            maximum_size: u64_at(&output, 40)?,
            allocation_delta: u64_at(&output, 48)?,
        })
    }

    pub fn storage_class(&self) -> search_core::IoClass {
        let mut query = StoragePropertyQuery {
            property_id: STORAGE_DEVICE_SEEK_PENALTY_PROPERTY,
            query_type: PROPERTY_STANDARD_QUERY,
            additional_parameters: [0],
        };
        let mut descriptor = DeviceSeekPenaltyDescriptor {
            version: 0,
            size: 0,
            incurs_seek_penalty: 0,
        };
        let result = self.device_io(
            IOCTL_STORAGE_QUERY_PROPERTY,
            (&mut query as *mut StoragePropertyQuery).cast(),
            std::mem::size_of::<StoragePropertyQuery>() as u32,
            (&mut descriptor as *mut DeviceSeekPenaltyDescriptor).cast(),
            std::mem::size_of::<DeviceSeekPenaltyDescriptor>() as u32,
        );
        match result {
            Ok(bytes) if bytes >= 9 && descriptor.incurs_seek_penalty != 0 => {
                search_core::IoClass::Hdd
            }
            Ok(bytes) if bytes >= 9 => search_core::IoClass::Ssd,
            _ => search_core::IoClass::Unknown,
        }
    }

    pub fn enumerate_mft<F>(&self, mut visit: F) -> io::Result<EnumerationStats>
    where
        F: FnMut(UsnRecordView<'_>) -> io::Result<()>,
    {
        let mut request = MftEnumDataV0 {
            start_file_reference_number: 0,
            low_usn: 0,
            high_usn: i64::MAX,
        };
        let mut output = vec![0_u8; ENUM_BUFFER_BYTES];
        let mut stats = EnumerationStats::default();

        loop {
            let result = self.device_io(
                FSCTL_ENUM_USN_DATA,
                (&mut request as *mut MftEnumDataV0).cast(),
                std::mem::size_of::<MftEnumDataV0>() as u32,
                output.as_mut_ptr().cast(),
                output.len() as u32,
            );

            let bytes = match result {
                Ok(bytes) => bytes as usize,
                Err(error) if is_enumeration_complete(&error) => break,
                Err(error) => return Err(error),
            };

            if bytes < 8 {
                break;
            }

            request.start_file_reference_number = u64_at(&output[..bytes], 0)?;
            stats.batches += 1;

            walk_usn_v2_records(&output[8..bytes], |record| {
                stats.records += 1;
                visit(record)
            })?;
        }

        Ok(stats)
    }

    pub fn read_journal_once<F>(
        &self,
        checkpoint: JournalCheckpoint,
        visit: F,
    ) -> io::Result<JournalReadStats>
    where
        F: FnMut(UsnRecordView<'_>) -> io::Result<()>,
    {
        let current = self.query_journal()?;
        if current.journal_id != checkpoint.journal_id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "USN journal identity changed; full reconciliation required",
            ));
        }
        if checkpoint.next_usn < current.first_usn {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "USN checkpoint was truncated; full reconciliation required",
            ));
        }
        self.read_journal_once_validated(checkpoint, visit)
    }

    pub(crate) fn read_journal_once_validated<F>(
        &self,
        checkpoint: JournalCheckpoint,
        mut visit: F,
    ) -> io::Result<JournalReadStats>
    where
        F: FnMut(UsnRecordView<'_>) -> io::Result<()>,
    {
        let mut request = ReadUsnJournalDataV0 {
            start_usn: checkpoint.next_usn,
            reason_mask: INDEX_RELEVANT_REASON_MASK,
            return_only_on_close: 0,
            timeout: 0,
            bytes_to_wait_for: 0,
            usn_journal_id: checkpoint.journal_id,
        };
        let mut output = vec![0_u8; READ_BUFFER_BYTES];

        let bytes = self.device_io(
            FSCTL_READ_USN_JOURNAL,
            (&mut request as *mut ReadUsnJournalDataV0).cast(),
            std::mem::size_of::<ReadUsnJournalDataV0>() as u32,
            output.as_mut_ptr().cast(),
            output.len() as u32,
        )? as usize;

        if bytes < 8 {
            return Ok(JournalReadStats {
                records: 0,
                next_usn: checkpoint.next_usn,
            });
        }

        let next_usn = i64_at(&output[..bytes], 0)?;
        let mut records = 0_u64;
        walk_usn_v2_records(&output[8..bytes], |record| {
            records += 1;
            visit(record)
        })?;

        Ok(JournalReadStats { records, next_usn })
    }

    fn device_io(
        &self,
        code: u32,
        input: *mut c_void,
        input_bytes: u32,
        output: *mut c_void,
        output_bytes: u32,
    ) -> io::Result<u32> {
        let mut returned = 0_u32;
        let ok = unsafe {
            device_io_control(
                self.handle,
                code,
                input,
                input_bytes,
                output,
                output_bytes,
                &mut returned,
                null_mut(),
            )
        };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(returned)
        }
    }
}

impl Drop for NtfsVolume {
    fn drop(&mut self) {
        if self.handle as isize != INVALID_HANDLE_VALUE && !self.handle.is_null() {
            let _ = unsafe { close_handle(self.handle) };
        }
    }
}

fn walk_usn_v2_records<F>(mut bytes: &[u8], mut visit: F) -> io::Result<()>
where
    F: FnMut(UsnRecordView<'_>) -> io::Result<()>,
{
    while !bytes.is_empty() {
        if bytes.len() < 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "truncated USN record prefix",
            ));
        }

        let record_length = u32_at(bytes, 0)? as usize;
        if record_length < USN_RECORD_V2_MIN_BYTES || record_length > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid USN record length",
            ));
        }

        let record = &bytes[..record_length];
        let major = u16_at(record, 4)?;
        if major != 2 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported USN record version {major}"),
            ));
        }

        let file_name_length = u16_at(record, 56)? as usize;
        let file_name_offset = u16_at(record, 58)? as usize;
        let file_name_end = file_name_offset
            .checked_add(file_name_length)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "USN name overflow"))?;

        if !file_name_length.is_multiple_of(2)
            || file_name_offset < USN_RECORD_V2_MIN_BYTES
            || file_name_end > record.len()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid USN filename bounds",
            ));
        }

        visit(UsnRecordView {
            file_reference_number: u64_at(record, 8)?,
            parent_file_reference_number: u64_at(record, 16)?,
            usn: i64_at(record, 24)?,
            reason: u32_at(record, 40)?,
            file_attributes: u32_at(record, 52)?,
            name_utf16_le: &record[file_name_offset..file_name_end],
        })?;

        bytes = &bytes[record_length..];
    }

    Ok(())
}

fn is_enumeration_complete(error: &io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(ERROR_HANDLE_EOF) | Some(ERROR_NO_MORE_FILES)
    )
}

fn u16_at(bytes: &[u8], offset: usize) -> io::Result<u16> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "u16 out of bounds"))?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> io::Result<u32> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "u32 out of bounds"))?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn u64_at(bytes: &[u8], offset: usize) -> io::Result<u64> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "u64 out of bounds"))?;
    Ok(u64::from_le_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

fn i64_at(bytes: &[u8], offset: usize) -> io::Result<i64> {
    Ok(u64_at(bytes, offset)? as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_codes_match_windows_definitions() {
        assert_eq!(FSCTL_ENUM_USN_DATA, 0x0009_00B3);
        assert_eq!(FSCTL_READ_USN_JOURNAL, 0x0009_00BB);
        assert_eq!(FSCTL_QUERY_USN_JOURNAL, 0x0009_00F4);
        assert_eq!(INDEX_RELEVANT_REASON_MASK, 0x0011_B300);
    }

    #[test]
    fn parses_usn_v2_without_allocating_filename() {
        let name: Vec<u16> = "node.exe".encode_utf16().collect();
        let name_bytes = name.len() * 2;
        let record_len = USN_RECORD_V2_MIN_BYTES + name_bytes;
        let mut raw = vec![0_u8; record_len];
        raw[0..4].copy_from_slice(&(record_len as u32).to_le_bytes());
        raw[4..6].copy_from_slice(&2_u16.to_le_bytes());
        raw[8..16].copy_from_slice(&123_u64.to_le_bytes());
        raw[16..24].copy_from_slice(&44_u64.to_le_bytes());
        raw[24..32].copy_from_slice(&987_i64.to_le_bytes());
        raw[52..56].copy_from_slice(&0x20_u32.to_le_bytes());
        raw[56..58].copy_from_slice(&(name_bytes as u16).to_le_bytes());
        raw[58..60].copy_from_slice(&(USN_RECORD_V2_MIN_BYTES as u16).to_le_bytes());
        for (i, word) in name.into_iter().enumerate() {
            let at = USN_RECORD_V2_MIN_BYTES + i * 2;
            raw[at..at + 2].copy_from_slice(&word.to_le_bytes());
        }

        let mut seen = 0;
        walk_usn_v2_records(&raw, |record| {
            seen += 1;
            assert_eq!(record.file_reference_number, 123);
            assert_eq!(record.parent_file_reference_number, 44);
            assert_eq!(record.usn, 987);
            assert_eq!(record.decode_name(), "node.exe");
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, 1);
    }
}
