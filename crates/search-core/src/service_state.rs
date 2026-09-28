use crate::IoClass;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC: [u8; 8] = *b"STSVST\0\0";
const VERSION: u16 = 1;
const RECORD_SIZE: u16 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncState {
    #[default]
    Never,
    Ok,
    Error,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaintenanceKind {
    #[default]
    None,
    Metadata,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceVolumeState {
    pub updated_unix_ms: u64,
    pub last_sync_unix_ms: u64,
    pub delta_bytes: u64,
    pub last_compact_unix_ms: u64,
    pub maintenance_started_unix_ms: u64,
    pub last_error_code: i32,
    pub last_maintenance_exit: i32,
    pub sync_state: SyncState,
    pub storage_class: IoClass,
    pub maintenance: MaintenanceKind,
}

impl ServiceVolumeState {
    pub const fn new(storage_class: IoClass) -> Self {
        Self {
            updated_unix_ms: 0,
            last_sync_unix_ms: 0,
            delta_bytes: 0,
            last_compact_unix_ms: 0,
            maintenance_started_unix_ms: 0,
            last_error_code: 0,
            last_maintenance_exit: 0,
            sync_state: SyncState::Never,
            storage_class,
            maintenance: MaintenanceKind::None,
        }
    }

    pub fn mark_sync_ok(&mut self, delta_bytes: u64) {
        let now = unix_millis();
        self.updated_unix_ms = now;
        self.last_sync_unix_ms = now;
        self.delta_bytes = delta_bytes;
        self.last_error_code = 0;
        self.sync_state = SyncState::Ok;
    }

    pub fn mark_sync_deferred(&mut self, delta_bytes: u64) {
        self.updated_unix_ms = unix_millis();
        self.delta_bytes = delta_bytes;
        self.last_error_code = 0;
        self.sync_state = SyncState::Deferred;
    }

    pub fn mark_sync_error(&mut self, error_code: i32, delta_bytes: u64) {
        let now = unix_millis();
        self.updated_unix_ms = now;
        self.last_sync_unix_ms = now;
        self.delta_bytes = delta_bytes;
        self.last_error_code = error_code;
        self.sync_state = SyncState::Error;
    }

    pub fn mark_compacted(&mut self, delta_bytes: u64) {
        let now = unix_millis();
        self.updated_unix_ms = now;
        self.last_compact_unix_ms = now;
        self.delta_bytes = delta_bytes;
    }

    pub fn mark_maintenance_started(&mut self, kind: MaintenanceKind) {
        let now = unix_millis();
        self.updated_unix_ms = now;
        self.maintenance_started_unix_ms = now;
        self.maintenance = kind;
    }

    pub fn mark_maintenance_finished(&mut self, exit_code: i32) {
        self.updated_unix_ms = unix_millis();
        self.last_maintenance_exit = exit_code;
        self.maintenance = MaintenanceKind::None;
        self.maintenance_started_unix_ms = 0;
    }
}

pub fn service_state_path(index_path: impl AsRef<Path>) -> PathBuf {
    let mut path = index_path.as_ref().as_os_str().to_os_string();
    path.push(".service");
    PathBuf::from(path)
}

pub fn read_service_state(index_path: impl AsRef<Path>) -> io::Result<ServiceVolumeState> {
    let path = service_state_path(index_path);
    let mut input = File::open(path)?;
    let mut bytes = [0_u8; RECORD_SIZE as usize];
    input.read_exact(&mut bytes)?;

    if bytes[0..8] != MAGIC {
        return Err(invalid("invalid service-state magic"));
    }
    if u16::from_le_bytes([bytes[8], bytes[9]]) != VERSION
        || u16::from_le_bytes([bytes[10], bytes[11]]) != RECORD_SIZE
    {
        return Err(invalid("unsupported service-state format"));
    }

    Ok(ServiceVolumeState {
        updated_unix_ms: read_u64(&bytes, 12),
        last_sync_unix_ms: read_u64(&bytes, 20),
        delta_bytes: read_u64(&bytes, 28),
        last_compact_unix_ms: read_u64(&bytes, 36),
        maintenance_started_unix_ms: read_u64(&bytes, 44),
        last_error_code: i32::from_le_bytes([bytes[52], bytes[53], bytes[54], bytes[55]]),
        last_maintenance_exit: i32::from_le_bytes([bytes[56], bytes[57], bytes[58], bytes[59]]),
        sync_state: decode_sync(bytes[60])?,
        storage_class: decode_storage(bytes[61])?,
        maintenance: decode_maintenance(bytes[62])?,
    })
}

pub fn write_service_state(
    index_path: impl AsRef<Path>,
    state: ServiceVolumeState,
) -> io::Result<()> {
    let path = service_state_path(index_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut staging_os = path.as_os_str().to_os_string();
    staging_os.push(".tmp");
    let staging = PathBuf::from(staging_os);

    let mut bytes = [0_u8; RECORD_SIZE as usize];
    bytes[0..8].copy_from_slice(&MAGIC);
    bytes[8..10].copy_from_slice(&VERSION.to_le_bytes());
    bytes[10..12].copy_from_slice(&RECORD_SIZE.to_le_bytes());
    bytes[12..20].copy_from_slice(&state.updated_unix_ms.to_le_bytes());
    bytes[20..28].copy_from_slice(&state.last_sync_unix_ms.to_le_bytes());
    bytes[28..36].copy_from_slice(&state.delta_bytes.to_le_bytes());
    bytes[36..44].copy_from_slice(&state.last_compact_unix_ms.to_le_bytes());
    bytes[44..52].copy_from_slice(&state.maintenance_started_unix_ms.to_le_bytes());
    bytes[52..56].copy_from_slice(&state.last_error_code.to_le_bytes());
    bytes[56..60].copy_from_slice(&state.last_maintenance_exit.to_le_bytes());
    bytes[60] = encode_sync(state.sync_state);
    bytes[61] = encode_storage(state.storage_class);
    bytes[62] = encode_maintenance(state.maintenance);

    let mut out = File::create(&staging)?;
    out.write_all(&bytes)?;
    out.sync_all()?;
    drop(out);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(staging, path)
}

pub fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn encode_sync(value: SyncState) -> u8 {
    match value {
        SyncState::Never => 0,
        SyncState::Ok => 1,
        SyncState::Error => 2,
        SyncState::Deferred => 3,
    }
}

fn decode_sync(value: u8) -> io::Result<SyncState> {
    match value {
        0 => Ok(SyncState::Never),
        1 => Ok(SyncState::Ok),
        2 => Ok(SyncState::Error),
        3 => Ok(SyncState::Deferred),
        _ => Err(invalid("invalid service sync state")),
    }
}

fn encode_storage(value: IoClass) -> u8 {
    match value {
        IoClass::Unknown => 0,
        IoClass::Ssd => 1,
        IoClass::Hdd => 2,
    }
}

fn decode_storage(value: u8) -> io::Result<IoClass> {
    match value {
        0 => Ok(IoClass::Unknown),
        1 => Ok(IoClass::Ssd),
        2 => Ok(IoClass::Hdd),
        _ => Err(invalid("invalid service storage class")),
    }
}

fn encode_maintenance(value: MaintenanceKind) -> u8 {
    match value {
        MaintenanceKind::None => 0,
        MaintenanceKind::Metadata => 1,
        MaintenanceKind::Content => 2,
    }
}

fn decode_maintenance(value: u8) -> io::Result<MaintenanceKind> {
    match value {
        0 => Ok(MaintenanceKind::None),
        1 => Ok(MaintenanceKind::Metadata),
        2 => Ok(MaintenanceKind::Content),
        _ => Err(invalid("invalid service maintenance kind")),
    }
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_state_roundtrip_is_fixed_and_volume_scoped() {
        let root =
            std::env::temp_dir().join(format!("search-tool-service-state-{}", unix_millis()));
        let index = root.join("C.stidx");
        fs::create_dir_all(&root).unwrap();
        let state = ServiceVolumeState {
            updated_unix_ms: 100,
            last_sync_unix_ms: 90,
            delta_bytes: 1234,
            last_compact_unix_ms: 80,
            maintenance_started_unix_ms: 70,
            last_error_code: 5,
            last_maintenance_exit: 7,
            sync_state: SyncState::Error,
            storage_class: IoClass::Hdd,
            maintenance: MaintenanceKind::Content,
        };
        write_service_state(&index, state).unwrap();
        assert_eq!(fs::metadata(service_state_path(&index)).unwrap().len(), 64);
        assert_eq!(read_service_state(&index).unwrap(), state);
        let _ = fs::remove_dir_all(root);
    }
}
