#[cfg(not(windows))]
fn main() {
    eprintln!("search-tool-service is only available on Windows");
}

#[cfg(windows)]
mod windows_service {
    use search_core::{
        attribute_index_path, compact_index, content_path, delta_path, pending_delta,
        sidecar_is_fresh, write_service_state, Decision, IoClass, LowEndPolicy, MaintenanceKind,
        QueryCost, ResourceGovernor, ServiceVolumeState, WorkClass,
    };
    use search_platform_windows::{
        enter_process_background_mode, rebuild_index, sync_index_bounded,
        usn_reconciliation_required, NtfsVolume, WindowsResourceProbe,
    };
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::ptr::null_mut;
    use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
    use std::sync::OnceLock;
    use std::thread;
    use std::time::{Duration, Instant};

    const DEFAULT_SERVICE_NAME: &str = "SearchToolIndexer";
    const SERVICE_WIN32_OWN_PROCESS: u32 = 0x0000_0010;
    const SERVICE_STOPPED: u32 = 0x0000_0001;
    const SERVICE_START_PENDING: u32 = 0x0000_0002;
    const SERVICE_STOP_PENDING: u32 = 0x0000_0003;
    const SERVICE_RUNNING: u32 = 0x0000_0004;
    const SERVICE_ACCEPT_STOP: u32 = 0x0000_0001;
    const SERVICE_ACCEPT_SHUTDOWN: u32 = 0x0000_0004;
    const SERVICE_CONTROL_STOP: u32 = 0x0000_0001;
    const SERVICE_CONTROL_SHUTDOWN: u32 = 0x0000_0005;
    const NO_ERROR: u32 = 0;
    const SC_MANAGER_CONNECT: u32 = 0x0000_0001;
    const SC_MANAGER_CREATE_SERVICE: u32 = 0x0000_0002;
    const SERVICE_QUERY_STATUS: u32 = 0x0000_0004;
    const SERVICE_START: u32 = 0x0000_0010;
    const SERVICE_STOP: u32 = 0x0000_0020;
    const DELETE_ACCESS: u32 = 0x0001_0000;
    const SERVICE_ALL_ACCESS: u32 = 0x000f_01ff;
    const SERVICE_AUTO_START: u32 = 0x0000_0002;
    const SERVICE_ERROR_NORMAL: u32 = 0x0000_0001;
    const ERROR_SERVICE_ALREADY_RUNNING: i32 = 1056;
    const ERROR_SERVICE_NOT_ACTIVE: i32 = 1062;
    const DEFAULT_SYNC_INTERVAL_MS: u64 = 3_000;
    const DEFAULT_COMPACT_DELTA_BYTES: u64 = 8 * 1024 * 1024;
    const MAX_LIVE_DELTA_BYTES: u64 = 2 * 1024 * 1024;
    const USN_BATCH_BYTES_ESTIMATE: u64 = 256 * 1024;
    const METADATA_MIN_IDLE_MS: u64 = 60_000;
    const CONTENT_MIN_IDLE_MS: u64 = 5 * 60_000;
    const METADATA_RETRY_INTERVAL: Duration = Duration::from_secs(10 * 60);
    const CONTENT_RETRY_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

    type ServiceStatusHandle = *mut c_void;
    type ScHandle = *mut c_void;

    #[repr(C)]
    struct ServiceStatus {
        service_type: u32,
        current_state: u32,
        controls_accepted: u32,
        win32_exit_code: u32,
        service_specific_exit_code: u32,
        check_point: u32,
        wait_hint: u32,
    }

    type ServiceMain = unsafe extern "system" fn(u32, *mut *mut u16);
    type HandlerEx = unsafe extern "system" fn(u32, u32, *mut c_void, *mut c_void) -> u32;

    #[repr(C)]
    struct ServiceTableEntryW {
        service_name: *mut u16,
        service_proc: Option<ServiceMain>,
    }

    #[link(name = "advapi32")]
    extern "system" {
        #[link_name = "StartServiceCtrlDispatcherW"]
        fn start_service_ctrl_dispatcher_w(table: *const ServiceTableEntryW) -> i32;
        #[link_name = "RegisterServiceCtrlHandlerExW"]
        fn register_service_ctrl_handler_ex_w(
            service_name: *const u16,
            handler: Option<HandlerEx>,
            context: *mut c_void,
        ) -> ServiceStatusHandle;
        #[link_name = "SetServiceStatus"]
        fn set_service_status(handle: ServiceStatusHandle, status: *const ServiceStatus) -> i32;
        #[link_name = "OpenSCManagerW"]
        fn open_sc_manager_w(machine: *const u16, database: *const u16, access: u32) -> ScHandle;
        #[link_name = "CreateServiceW"]
        fn create_service_w(
            manager: ScHandle,
            service_name: *const u16,
            display_name: *const u16,
            desired_access: u32,
            service_type: u32,
            start_type: u32,
            error_control: u32,
            binary_path: *const u16,
            load_order_group: *const u16,
            tag_id: *mut u32,
            dependencies: *const u16,
            service_start_name: *const u16,
            password: *const u16,
        ) -> ScHandle;
        #[link_name = "OpenServiceW"]
        fn open_service_w(manager: ScHandle, service_name: *const u16, access: u32) -> ScHandle;
        #[link_name = "DeleteService"]
        fn delete_service(service: ScHandle) -> i32;
        #[link_name = "StartServiceW"]
        fn start_service_w(service: ScHandle, argc: u32, argv: *const *const u16) -> i32;
        #[link_name = "ControlService"]
        fn control_service(service: ScHandle, control: u32, status: *mut ServiceStatus) -> i32;
        #[link_name = "QueryServiceStatus"]
        fn query_service_status(service: ScHandle, status: *mut ServiceStatus) -> i32;
        #[link_name = "CloseServiceHandle"]
        fn close_service_handle(handle: ScHandle) -> i32;
    }

    static STOP: AtomicBool = AtomicBool::new(false);
    static STATUS_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
    static STOP_WAKER: OnceLock<thread::Thread> = OnceLock::new();
    static SERVICE_NAME: OnceLock<String> = OnceLock::new();

    #[derive(Debug, Clone)]
    struct VolumeConfig {
        drive: char,
        index: PathBuf,
    }

    #[derive(Debug, Clone)]
    struct ServiceConfig {
        volumes: Vec<VolumeConfig>,
        sync_interval_ms: u64,
        compact_delta_bytes: u64,
    }

    #[derive(Debug, Clone, Copy)]
    struct RuntimeVolume<'a> {
        config: &'a VolumeConfig,
        storage_class: IoClass,
        low_end: LowEndPolicy,
    }

    #[derive(Debug)]
    struct MaintenanceChild {
        child: Child,
        drive: char,
        kind: &'static str,
    }

    #[derive(Debug)]
    struct VolumeStatusRuntime {
        index: PathBuf,
        state: ServiceVolumeState,
        last_persist: Option<Instant>,
        last_persisted_updated_unix_ms: u64,
    }

    impl VolumeStatusRuntime {
        fn new(index: PathBuf, storage_class: IoClass) -> Self {
            Self {
                index,
                state: ServiceVolumeState::new(storage_class),
                last_persist: None,
                last_persisted_updated_unix_ms: 0,
            }
        }

        fn persist(&mut self, force: bool) {
            let interval = match self.state.storage_class {
                IoClass::Hdd => Duration::from_secs(5 * 60),
                IoClass::Ssd | IoClass::Unknown => Duration::from_secs(60),
            };
            let due = self
                .last_persist
                .is_none_or(|last| last.elapsed() >= interval);
            let first_real_update =
                self.last_persisted_updated_unix_ms == 0 && self.state.updated_unix_ms != 0;
            if (force || due || first_real_update)
                && write_service_state(&self.index, self.state).is_ok()
            {
                self.last_persist = Some(Instant::now());
                self.last_persisted_updated_unix_ms = self.state.updated_unix_ms;
            }
        }
    }

    impl ServiceConfig {
        fn single(drive: char, index: PathBuf) -> Self {
            Self {
                volumes: vec![VolumeConfig { drive, index }],
                sync_interval_ms: DEFAULT_SYNC_INTERVAL_MS,
                compact_delta_bytes: DEFAULT_COMPACT_DELTA_BYTES,
            }
        }

        fn load(path: &Path) -> io::Result<Self> {
            let text = fs::read_to_string(path)?;
            let mut volumes = Vec::new();
            let mut legacy_drive = None;
            let mut legacy_index = None;
            let mut sync_interval_ms = DEFAULT_SYNC_INTERVAL_MS;
            let mut compact_delta_bytes = DEFAULT_COMPACT_DELTA_BYTES;
            for raw in text.lines() {
                let line = raw.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                match key.trim() {
                    "volume" => volumes.push(parse_volume_config(value.trim())?),
                    "drive" => {
                        legacy_drive = value.trim().chars().next().map(|c| c.to_ascii_uppercase());
                    }
                    "index" => legacy_index = Some(PathBuf::from(value.trim())),
                    "sync_interval_ms" => {
                        sync_interval_ms = value.trim().parse().unwrap_or(DEFAULT_SYNC_INTERVAL_MS)
                    }
                    "compact_delta_bytes" => {
                        compact_delta_bytes =
                            value.trim().parse().unwrap_or(DEFAULT_COMPACT_DELTA_BYTES)
                    }
                    _ => {}
                }
            }

            if volumes.is_empty() {
                let drive = legacy_drive
                    .filter(|c| c.is_ascii_alphabetic())
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "service config has no volume entries",
                        )
                    })?;
                let index = legacy_index
                    .filter(|p| !p.as_os_str().is_empty())
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "service config index is missing",
                        )
                    })?;
                volumes.push(VolumeConfig { drive, index });
            }

            validate_volumes(&mut volumes)?;
            Ok(Self {
                volumes,
                sync_interval_ms: sync_interval_ms.clamp(1_000, 60_000),
                compact_delta_bytes: compact_delta_bytes.max(1024 * 1024),
            })
        }
    }

    fn parse_service_name_option(args: &mut Vec<String>) -> io::Result<String> {
        let mut selected = None;
        let mut index = 1;
        while index < args.len() {
            if args[index] != "--service-name" {
                index += 1;
                continue;
            }
            if selected.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--service-name may be specified only once",
                ));
            }
            let value = args.get(index + 1).cloned().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--service-name requires a value",
                )
            })?;
            validate_service_name(&value)?;
            selected = Some(value);
            args.drain(index..=index + 1);
        }
        Ok(selected.unwrap_or_else(|| DEFAULT_SERVICE_NAME.to_owned()))
    }

    fn validate_service_name(value: &str) -> io::Result<()> {
        let utf16_len = value.encode_utf16().count();
        if value.trim().is_empty()
            || utf16_len > 256
            || value.contains('\0')
            || value
                .chars()
                .any(|ch| matches!(ch, '\\' | '/' | '"' | '<' | '>' | ':' | '|' | '?' | '*'))
            || value.ends_with(' ')
            || value.ends_with('.')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid Windows service name",
            ));
        }
        Ok(())
    }

    fn current_service_name() -> &'static str {
        SERVICE_NAME
            .get()
            .map(String::as_str)
            .unwrap_or(DEFAULT_SERVICE_NAME)
    }

    pub fn entry() -> io::Result<()> {
        let mut args: Vec<String> = std::env::args().collect();
        let service_name = parse_service_name_option(&mut args)?;
        SERVICE_NAME
            .set(service_name)
            .map_err(|_| io::Error::other("service name was already initialized"))?;
        if args.get(1).map(String::as_str) == Some("--write-config") {
            let drive = parse_drive(args.get(2).map(String::as_str))?;
            let index = args
                .get(3)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing index path"))?;
            return write_default_config(drive, Path::new(index));
        }
        if args.get(1).map(String::as_str) == Some("--write-config-multi") {
            let index_dir = args
                .get(2)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing index dir"))?;
            let drives = args
                .get(3)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing drive list"))?;
            return write_multi_config(Path::new(index_dir), drives);
        }
        if args.get(1).map(String::as_str) == Some("--install") {
            let drive = parse_drive(args.get(2).map(String::as_str))?;
            let index = args
                .get(3)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing index path"))?;
            write_default_config(drive, Path::new(index))?;
            return install_service();
        }
        if args.get(1).map(String::as_str) == Some("--install-multi") {
            let index_dir = args
                .get(2)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing index dir"))?;
            let drives = args
                .get(3)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing drive list"))?;
            write_multi_config(Path::new(index_dir), drives)?;
            return install_service();
        }
        if args.get(1).map(String::as_str) == Some("--uninstall") {
            return uninstall_service();
        }
        if args.get(1).map(String::as_str) == Some("--start") {
            return start_installed_service();
        }
        if args.get(1).map(String::as_str) == Some("--stop") {
            return stop_installed_service();
        }
        if args.get(1).map(String::as_str) == Some("--console") {
            let drive = parse_drive(args.get(2).map(String::as_str))?;
            let index = args
                .get(3)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing index path"))?;
            return run_loop(ServiceConfig::single(drive, PathBuf::from(index)));
        }
        if args.get(1).map(String::as_str) == Some("--console-config") {
            return default_config_path()
                .and_then(|path| ServiceConfig::load(&path))
                .and_then(run_loop);
        }

        let mut name = wide(current_service_name());
        let table = [
            ServiceTableEntryW {
                service_name: name.as_mut_ptr(),
                service_proc: Some(service_main),
            },
            ServiceTableEntryW {
                service_name: null_mut(),
                service_proc: None,
            },
        ];
        let ok = unsafe { start_service_ctrl_dispatcher_w(table.as_ptr()) };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    unsafe extern "system" fn service_main(_argc: u32, _argv: *mut *mut u16) {
        let name = wide(current_service_name());
        let handle =
            register_service_ctrl_handler_ex_w(name.as_ptr(), Some(service_handler), null_mut());
        if handle.is_null() {
            return;
        }
        STATUS_HANDLE.store(handle, Ordering::Release);
        set_status(SERVICE_START_PENDING, 0, 4_000, 1);

        let result = default_config_path()
            .and_then(|path| ServiceConfig::load(&path))
            .and_then(run_loop);
        let exit = if result.is_ok() { 0 } else { 1 };
        set_status(SERVICE_STOPPED, exit, 0, 0);
    }

    unsafe extern "system" fn service_handler(
        control: u32,
        _event_type: u32,
        _event_data: *mut c_void,
        _context: *mut c_void,
    ) -> u32 {
        if matches!(control, SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN) {
            STOP.store(true, Ordering::Release);
            if let Some(thread) = STOP_WAKER.get() {
                thread.unpark();
            }
            set_status(SERVICE_STOP_PENDING, 0, 5_000, 1);
        }
        NO_ERROR
    }

    fn run_loop(config: ServiceConfig) -> io::Result<()> {
        STOP.store(false, Ordering::Release);
        // Best-effort OS-level background scheduling. ResourceGovernor still owns
        // correctness/throttling decisions; this only makes foreground apps win
        // scheduler and disk contention more aggressively.
        let _ = enter_process_background_mode();
        let _ = STOP_WAKER.set(thread::current());
        if !STATUS_HANDLE.load(Ordering::Acquire).is_null() {
            unsafe { set_status(SERVICE_RUNNING, 0, 0, 0) };
        }
        if config.volumes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "service has no configured volumes",
            ));
        }

        let governor = ResourceGovernor::new(search_core::GovernorConfig::default());
        let runtime_volumes: Vec<_> = config
            .volumes
            .iter()
            .map(|volume_config| {
                let storage_class = NtfsVolume::open_drive(volume_config.drive)
                    .map(|volume| volume.storage_class())
                    .unwrap_or(IoClass::Unknown);
                RuntimeVolume {
                    config: volume_config,
                    storage_class,
                    low_end: LowEndPolicy::for_storage(storage_class),
                }
            })
            .collect();
        let any_hdd = runtime_volumes
            .iter()
            .any(|volume| volume.storage_class == IoClass::Hdd);
        let mut probe = WindowsResourceProbe::new();
        let _ = probe.sample();
        let mut metadata_attempts: HashMap<char, Instant> = HashMap::new();
        let mut content_attempts: HashMap<char, Instant> = HashMap::new();
        let mut maintenance: Option<MaintenanceChild> = None;
        let mut volume_status: HashMap<char, VolumeStatusRuntime> = runtime_volumes
            .iter()
            .map(|volume| {
                (
                    volume.config.drive,
                    VolumeStatusRuntime::new(volume.config.index.clone(), volume.storage_class),
                )
            })
            .collect();
        for status in volume_status.values_mut() {
            status.persist(true);
        }

        while !STOP.load(Ordering::Acquire) {
            if let Some((drive, exit_code)) = reap_maintenance(&mut maintenance) {
                if let Some(status) = volume_status.get_mut(&drive) {
                    status.state.mark_maintenance_finished(exit_code);
                    status.persist(true);
                }
            }
            let sample = probe.sample();
            let decision = governor.decide(WorkClass::MetadataUpdate, sample);

            let mut journal_backlog = false;
            for volume in &runtime_volumes {
                if STOP.load(Ordering::Acquire) {
                    break;
                }

                let compact_allowed =
                    volume
                        .low_end
                        .permit(&governor, QueryCost::Expensive, sample)
                        == Decision::Run
                        && sample.user_idle_ms >= 30_000
                        && !sample.on_battery;
                let live_delta_limit = config.compact_delta_bytes.min(MAX_LIVE_DELTA_BYTES);
                let delta = delta_path(&volume.config.index);
                let mut delta_bytes = fs::metadata(&delta).map(|m| m.len()).unwrap_or(0);

                // Do not keep ingesting journal pages while the user is active once the
                // bounded in-memory live overlay is near its byte budget. The durable USN
                // checkpoint stays behind, so idle catch-up remains lossless.
                if delta_bytes >= live_delta_limit && !compact_allowed {
                    if let Some(status) = volume_status.get_mut(&volume.config.drive) {
                        status.state.mark_sync_deferred(delta_bytes);
                        status.persist(false);
                    }
                    continue;
                }

                if delta_bytes >= live_delta_limit
                    && compact_allowed
                    && compact_index(&volume.config.index).is_ok()
                {
                    delta_bytes = fs::metadata(delta_path(&volume.config.index))
                        .map(|m| m.len())
                        .unwrap_or(0);
                    if let Some(status) = volume_status.get_mut(&volume.config.drive) {
                        status.state.mark_compacted(delta_bytes);
                        status.persist(true);
                    }
                }

                let mut sync_error = None;
                let sync_attempted = true;
                let normal_batches = if volume.storage_class == IoClass::Hdd {
                    4
                } else {
                    8
                };
                // A full Pause should stop expensive/background enrichment, not make
                // filename freshness unbounded. One bounded USN page every paused loop
                // is cheap (<= READ_BUFFER_BYTES at the platform layer), while the
                // longer Pause sleep still gives foreground work priority.
                let max_sync_batches = if decision == Decision::Pause {
                    1
                } else {
                    normal_batches
                };
                let remaining_bytes = live_delta_limit.saturating_sub(delta_bytes);
                let budget_batches = remaining_bytes
                    .div_ceil(USN_BATCH_BYTES_ESTIMATE)
                    .clamp(1, max_sync_batches as u64)
                    as usize;
                let sync_result =
                    sync_index_bounded(volume.config.drive, &volume.config.index, budget_batches);
                if let Ok(stats) = &sync_result {
                    if !stats.caught_up {
                        journal_backlog = true;
                    }
                }
                if let Err(error) = sync_result {
                    if error.kind() == io::ErrorKind::WouldBlock {
                        journal_backlog = true;
                        if let Some(status) = volume_status.get_mut(&volume.config.drive) {
                            status.state.mark_sync_deferred(delta_bytes);
                            status.persist(false);
                        }
                        continue;
                    }
                    let recoverable = usn_reconciliation_required(&error);
                    let rebuild_allowed = recoverable
                        && governor.decide(WorkClass::BackgroundIndex, sample) == Decision::Run
                        && volume
                            .low_end
                            .permit(&governor, QueryCost::Expensive, sample)
                            == Decision::Run
                        && sample.user_idle_ms >= 30_000
                        && !sample.on_battery;
                    if rebuild_allowed {
                        // A sidecar worker belongs to the generation that just became
                        // untrustworthy. Stop it before publishing a rebuilt base, or
                        // it can race the rebuild and republish stale derived files.
                        let cancelled_maintenance =
                            maintenance.as_ref().map(|task| (task.drive, task.kind));
                        if let Some((task_drive, task_kind)) = cancelled_maintenance {
                            stop_maintenance(&mut maintenance);
                            match task_kind {
                                "metadata-build" => {
                                    metadata_attempts.remove(&task_drive);
                                }
                                "content-build" => {
                                    content_attempts.remove(&task_drive);
                                }
                                _ => {}
                            }
                            if let Some(status) = volume_status.get_mut(&task_drive) {
                                status.state.mark_maintenance_finished(-2);
                                status.persist(true);
                            }
                        }
                        if let Err(rebuild_error) =
                            rebuild_index(volume.config.drive, &volume.config.index)
                        {
                            sync_error = Some(error_code(&rebuild_error));
                        }
                    } else {
                        sync_error = Some(error_code(&error));
                    }
                }

                delta_bytes = fs::metadata(&delta).map(|m| m.len()).unwrap_or(0);
                if sync_attempted {
                    if let Some(status) = volume_status.get_mut(&volume.config.drive) {
                        if let Some(code) = sync_error {
                            status.state.mark_sync_error(code, delta_bytes);
                            status.persist(true);
                        } else {
                            status.state.mark_sync_ok(delta_bytes);
                            status.persist(false);
                        }
                    }
                }
                if delta_bytes >= live_delta_limit
                    && compact_allowed
                    && compact_index(&volume.config.index).is_ok()
                {
                    if let Some(status) = volume_status.get_mut(&volume.config.drive) {
                        let remaining = fs::metadata(delta_path(&volume.config.index))
                            .map(|m| m.len())
                            .unwrap_or(0);
                        status.state.mark_compacted(remaining);
                        status.persist(true);
                    }
                }

                if compact_allowed {
                    let maintenance_before = maintenance.is_some();
                    maybe_refresh_sidecars(
                        volume,
                        sample.user_idle_ms,
                        &mut metadata_attempts,
                        &mut content_attempts,
                        &mut maintenance,
                    );
                    if !maintenance_before {
                        if let Some(task) = maintenance.as_ref() {
                            if let Some(status) = volume_status.get_mut(&task.drive) {
                                let kind = match task.kind {
                                    "metadata-build" => MaintenanceKind::Metadata,
                                    "content-build" => MaintenanceKind::Content,
                                    _ => MaintenanceKind::None,
                                };
                                status.state.mark_maintenance_started(kind);
                                status.persist(true);
                            }
                        }
                    }
                }
            }

            let base_interval = if any_hdd {
                config.sync_interval_ms.max(5_000)
            } else {
                config.sync_interval_ms
            };
            // Keep filename freshness bounded even while foreground work has priority.
            // Pause mode still reads only one USN page per loop; an actual backlog simply
            // avoids the additional 4x sleep multiplier until the checkpoint catches up.
            let sleep_ms = if journal_backlog && decision == Decision::Pause {
                base_interval
            } else {
                match decision {
                    Decision::Run => base_interval,
                    Decision::Throttle => base_interval.saturating_mul(2),
                    Decision::Pause => base_interval.saturating_mul(4),
                }
            };
            sleep_interruptible(sleep_ms);
        }
        stop_maintenance(&mut maintenance);
        Ok(())
    }

    unsafe fn set_status(state: u32, exit_code: u32, wait_hint: u32, checkpoint: u32) {
        let handle = STATUS_HANDLE.load(Ordering::Acquire);
        if handle.is_null() {
            return;
        }
        let accepted = if state == SERVICE_RUNNING {
            SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN
        } else {
            0
        };
        let status = ServiceStatus {
            service_type: SERVICE_WIN32_OWN_PROCESS,
            current_state: state,
            controls_accepted: accepted,
            win32_exit_code: exit_code,
            service_specific_exit_code: 0,
            check_point: checkpoint,
            wait_hint,
        };
        let _ = set_service_status(handle, &status);
    }

    fn maybe_refresh_sidecars(
        volume: &RuntimeVolume<'_>,
        user_idle_ms: u64,
        metadata_attempts: &mut HashMap<char, Instant>,
        content_attempts: &mut HashMap<char, Instant>,
        maintenance: &mut Option<MaintenanceChild>,
    ) {
        if maintenance.is_some() || pending_delta(&volume.config.index).unwrap_or(true) {
            return;
        }

        let metadata_path = attribute_index_path(&volume.config.index);
        let metadata_stale =
            !sidecar_is_fresh(&volume.config.index, &metadata_path).unwrap_or(false);
        if metadata_stale
            && user_idle_ms >= METADATA_MIN_IDLE_MS
            && attempt_due(
                metadata_attempts.get(&volume.config.drive),
                METADATA_RETRY_INTERVAL,
            )
        {
            metadata_attempts.insert(volume.config.drive, Instant::now());
            if let Ok(child) = start_enrichment_command(
                "metadata-build",
                volume.config.drive,
                &volume.config.index,
            ) {
                *maintenance = Some(MaintenanceChild {
                    child,
                    drive: volume.config.drive,
                    kind: "metadata-build",
                });
            }
            return;
        }

        let content_index = content_path(&volume.config.index);
        let content_stale =
            !sidecar_is_fresh(&volume.config.index, &content_index).unwrap_or(false);
        if content_stale
            && user_idle_ms >= CONTENT_MIN_IDLE_MS
            && attempt_due(
                content_attempts.get(&volume.config.drive),
                CONTENT_RETRY_INTERVAL,
            )
        {
            content_attempts.insert(volume.config.drive, Instant::now());
            if let Ok(child) =
                start_enrichment_command("content-build", volume.config.drive, &volume.config.index)
            {
                *maintenance = Some(MaintenanceChild {
                    child,
                    drive: volume.config.drive,
                    kind: "content-build",
                });
            }
        }
    }

    fn attempt_due(last: Option<&Instant>, interval: Duration) -> bool {
        last.is_none_or(|last| last.elapsed() >= interval)
    }

    fn start_enrichment_command(command: &str, drive: char, index: &Path) -> io::Result<Child> {
        let cli = std::env::current_exe()?.with_file_name("search-tool.exe");
        Command::new(cli)
            .arg(command)
            .arg(format!("{drive}:"))
            .arg(index)
            .env("SEARCH_TOOL_BACKGROUND", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }

    fn reap_maintenance(maintenance: &mut Option<MaintenanceChild>) -> Option<(char, i32)> {
        let task = maintenance.as_mut()?;
        match task.child.try_wait() {
            Ok(Some(status)) => {
                let result = (task.drive, status.code().unwrap_or(-1));
                *maintenance = None;
                Some(result)
            }
            Ok(None) => None,
            Err(_) => {
                let drive = task.drive;
                let _ = task.child.kill();
                let _ = task.child.wait();
                *maintenance = None;
                Some((drive, -1))
            }
        }
    }

    fn stop_maintenance(maintenance: &mut Option<MaintenanceChild>) {
        if let Some(mut task) = maintenance.take() {
            let _ = task.child.kill();
            let _ = task.child.wait();
        }
    }

    fn error_code(error: &io::Error) -> i32 {
        error.raw_os_error().unwrap_or(-1)
    }

    fn sleep_interruptible(total_ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(total_ms);
        while !STOP.load(Ordering::Acquire) {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            thread::park_timeout(deadline.saturating_duration_since(now));
        }
    }

    fn install_service() -> io::Result<()> {
        let manager =
            unsafe { open_sc_manager_w(null_mut(), null_mut(), SC_MANAGER_CREATE_SERVICE) };
        if manager.is_null() {
            return Err(io::Error::last_os_error());
        }
        let exe = std::env::current_exe()?;
        let binary = wide(&format!(
            "\"{}\" --service-name \"{}\"",
            exe.display(),
            current_service_name()
        ));
        let name = wide(current_service_name());
        let display_name = if current_service_name() == DEFAULT_SERVICE_NAME {
            "Search Tool Indexer".to_owned()
        } else {
            format!("Search Tool Indexer [{}]", current_service_name())
        };
        let display = wide(&display_name);
        let service = unsafe {
            create_service_w(
                manager,
                name.as_ptr(),
                display.as_ptr(),
                SERVICE_ALL_ACCESS,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                binary.as_ptr(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        let result = if service.is_null() {
            Err(io::Error::last_os_error())
        } else {
            unsafe { close_service_handle(service) };
            Ok(())
        };
        unsafe { close_service_handle(manager) };
        result
    }

    fn with_service<T>(
        access: u32,
        action: impl FnOnce(ScHandle) -> io::Result<T>,
    ) -> io::Result<T> {
        let manager = unsafe { open_sc_manager_w(null_mut(), null_mut(), SC_MANAGER_CONNECT) };
        if manager.is_null() {
            return Err(io::Error::last_os_error());
        }
        let name = wide(current_service_name());
        let service = unsafe { open_service_w(manager, name.as_ptr(), access) };
        if service.is_null() {
            let error = io::Error::last_os_error();
            unsafe { close_service_handle(manager) };
            return Err(error);
        }
        let result = action(service);
        unsafe {
            close_service_handle(service);
            close_service_handle(manager);
        }
        result
    }

    fn empty_service_status() -> ServiceStatus {
        ServiceStatus {
            service_type: 0,
            current_state: 0,
            controls_accepted: 0,
            win32_exit_code: 0,
            service_specific_exit_code: 0,
            check_point: 0,
            wait_hint: 0,
        }
    }

    fn query_service_state(service: ScHandle) -> io::Result<u32> {
        let mut status = empty_service_status();
        if unsafe { query_service_status(service, &mut status) } == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(status.current_state)
        }
    }

    fn wait_for_service_state(service: ScHandle, target: u32, timeout: Duration) -> io::Result<()> {
        let started = Instant::now();
        loop {
            if query_service_state(service)? == target {
                return Ok(());
            }
            if started.elapsed() >= timeout {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timed out waiting for service state",
                ));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn start_installed_service() -> io::Result<()> {
        with_service(SERVICE_START | SERVICE_QUERY_STATUS, |service| {
            let ok = unsafe { start_service_w(service, 0, null_mut()) };
            if ok == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(ERROR_SERVICE_ALREADY_RUNNING) {
                    return Err(error);
                }
            }
            wait_for_service_state(service, SERVICE_RUNNING, Duration::from_secs(20))
        })
    }

    fn stop_installed_service() -> io::Result<()> {
        with_service(SERVICE_STOP | SERVICE_QUERY_STATUS, |service| {
            if query_service_state(service)? == SERVICE_STOPPED {
                return Ok(());
            }
            let mut status = empty_service_status();
            let ok = unsafe { control_service(service, SERVICE_CONTROL_STOP, &mut status) };
            if ok == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(ERROR_SERVICE_NOT_ACTIVE) {
                    return Err(error);
                }
            }
            wait_for_service_state(service, SERVICE_STOPPED, Duration::from_secs(20))
        })
    }

    fn uninstall_service() -> io::Result<()> {
        stop_installed_service()?;
        with_service(DELETE_ACCESS | SERVICE_QUERY_STATUS, |service| {
            let ok = unsafe { delete_service(service) };
            if ok == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        })
    }

    fn parse_drive(value: Option<&str>) -> io::Result<char> {
        let value = value.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "drive is required, e.g. C:")
        })?;
        let drive = value
            .chars()
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if drive.is_ascii_alphabetic() {
            Ok(drive)
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid drive"))
        }
    }

    fn service_config_pointer_path(exe: &Path, service_name: &str) -> PathBuf {
        if service_name == DEFAULT_SERVICE_NAME {
            exe.with_file_name("service.conf.path")
        } else {
            exe.with_file_name(format!("service.{service_name}.conf.path"))
        }
    }

    fn service_local_config_path(exe: &Path, service_name: &str) -> PathBuf {
        exe.with_file_name(format!("service.{service_name}.conf"))
    }

    fn read_config_pointer(pointer: &Path) -> io::Result<Option<PathBuf>> {
        match fs::read_to_string(pointer) {
            Ok(raw) => {
                let trimmed = raw.trim().trim_start_matches('\u{feff}');
                if trimmed.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{} is empty", pointer.display()),
                    ));
                }
                Ok(Some(PathBuf::from(trimmed)))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn default_config_path() -> io::Result<PathBuf> {
        // Installed/portable layouts may intentionally keep Search Tool data outside
        // the machine-wide default ProgramData directory. A pointer beside the
        // service binary lets the installer choose that location without embedding
        // mutable configuration in the SCM command line. Custom service names first
        // get their own pointer/config so validation or side-by-side instances cannot
        // overwrite the production service configuration.
        let exe = std::env::current_exe()?;
        let service_name = current_service_name();
        let pointer = service_config_pointer_path(&exe, service_name);
        if let Some(path) = read_config_pointer(&pointer)? {
            return Ok(path);
        }

        if service_name != DEFAULT_SERVICE_NAME {
            let legacy_pointer = exe.with_file_name("service.conf.path");
            if let Some(path) = read_config_pointer(&legacy_pointer)? {
                return Ok(path);
            }
            return Ok(service_local_config_path(&exe, service_name));
        }

        let root = std::env::var_os("ProgramData")
            .or_else(|| std::env::var_os("ALLUSERSPROFILE"))
            .unwrap_or_else(|| "C:\\ProgramData".into());
        Ok(PathBuf::from(root).join("SearchTool").join("service.conf"))
    }

    fn write_default_config(drive: char, index: &Path) -> io::Result<()> {
        write_service_config(&[VolumeConfig {
            drive,
            index: index.to_path_buf(),
        }])
    }

    fn write_multi_config(index_dir: &Path, drives: &str) -> io::Result<()> {
        let mut volumes = Vec::new();
        for raw in drives.split([',', ';']) {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let drive = parse_drive(Some(trimmed))?;
            volumes.push(VolumeConfig {
                drive,
                index: index_dir.join(format!("{drive}.stidx")),
            });
        }
        validate_volumes(&mut volumes)?;
        write_service_config(&volumes)
    }

    fn write_service_config(volumes: &[VolumeConfig]) -> io::Result<()> {
        if volumes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "at least one volume is required",
            ));
        }
        let path = default_config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut text = String::new();
        for volume in volumes {
            text.push_str(&format!(
                "volume={}|{}\n",
                volume.drive,
                volume.index.display()
            ));
        }
        text.push_str(&format!(
            "sync_interval_ms={DEFAULT_SYNC_INTERVAL_MS}\ncompact_delta_bytes={DEFAULT_COMPACT_DELTA_BYTES}\n"
        ));
        fs::write(path, text)
    }

    fn parse_volume_config(value: &str) -> io::Result<VolumeConfig> {
        let (drive, index) = value.split_once('|').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "volume entry must be DRIVE|INDEX_PATH",
            )
        })?;
        let drive = parse_drive(Some(drive.trim()))?;
        let index = PathBuf::from(index.trim());
        if index.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "volume index path is empty",
            ));
        }
        Ok(VolumeConfig { drive, index })
    }

    fn validate_volumes(volumes: &mut [VolumeConfig]) -> io::Result<()> {
        if volumes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "service config has no volume entries",
            ));
        }
        volumes.sort_unstable_by_key(|volume| volume.drive);
        for pair in volumes.windows(2) {
            if pair[0].drive == pair[1].drive {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("duplicate volume {}", pair[0].drive),
                ));
            }
        }
        Ok(())
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    #[cfg(test)]
    mod tests {
        use super::{
            parse_service_name_option, service_config_pointer_path, service_local_config_path,
            DEFAULT_SERVICE_NAME,
        };
        use std::path::Path;

        #[test]
        fn service_name_defaults_without_option() {
            let mut args = vec!["service".to_owned(), "--start".to_owned()];
            let name = parse_service_name_option(&mut args).unwrap();
            assert_eq!(name, DEFAULT_SERVICE_NAME);
            assert_eq!(args, ["service", "--start"]);
        }

        #[test]
        fn custom_service_name_is_removed_from_command_args() {
            let mut args = vec![
                "service".to_owned(),
                "--service-name".to_owned(),
                "SearchToolIndexerFaultTest".to_owned(),
                "--start".to_owned(),
            ];
            let name = parse_service_name_option(&mut args).unwrap();
            assert_eq!(name, "SearchToolIndexerFaultTest");
            assert_eq!(args, ["service", "--start"]);
        }

        #[test]
        fn unsafe_or_duplicate_service_names_are_rejected() {
            for invalid in [
                "",
                "bad/name",
                r"bad\name",
                "bad\"name",
                "bad:name",
                "bad*name",
                "bad|name",
                "bad.",
            ] {
                let mut args = vec![
                    "service".to_owned(),
                    "--service-name".to_owned(),
                    invalid.to_owned(),
                ];
                assert!(parse_service_name_option(&mut args).is_err());
            }

            let mut duplicate = vec![
                "service".to_owned(),
                "--service-name".to_owned(),
                "one".to_owned(),
                "--service-name".to_owned(),
                "two".to_owned(),
            ];
            assert!(parse_service_name_option(&mut duplicate).is_err());
        }

        #[test]
        fn custom_service_config_paths_are_isolated() {
            let exe = Path::new(r"C:\SearchTool\search-tool-service.exe");
            assert_eq!(
                service_config_pointer_path(exe, DEFAULT_SERVICE_NAME),
                Path::new(r"C:\SearchTool\service.conf.path")
            );
            assert_eq!(
                service_config_pointer_path(exe, "SearchToolIndexerIntegration"),
                Path::new(r"C:\SearchTool\service.SearchToolIndexerIntegration.conf.path")
            );
            assert_eq!(
                service_local_config_path(exe, "SearchToolIndexerIntegration"),
                Path::new(r"C:\SearchTool\service.SearchToolIndexerIntegration.conf")
            );
        }
    }
}

#[cfg(windows)]
fn main() {
    if let Err(error) = windows_service::entry() {
        eprintln!("search-tool-service: {error}");
        std::process::exit(1);
    }
}
