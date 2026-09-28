use std::{ffi::c_void, io, ptr::null};

type ScHandle = *mut c_void;

const SC_MANAGER_CONNECT: u32 = 0x0001;
const SERVICE_QUERY_STATUS: u32 = 0x0004;
const SC_STATUS_PROCESS_INFO: u32 = 0;
const SERVICE_RUNNING: u32 = 0x0000_0004;

#[repr(C)]
#[derive(Debug, Default)]
struct ServiceStatusProcess {
    service_type: u32,
    current_state: u32,
    controls_accepted: u32,
    win32_exit_code: u32,
    service_specific_exit_code: u32,
    check_point: u32,
    wait_hint: u32,
    process_id: u32,
    service_flags: u32,
}

#[link(name = "advapi32")]
extern "system" {
    #[link_name = "OpenSCManagerW"]
    fn open_sc_manager_w(
        machine_name: *const u16,
        database_name: *const u16,
        desired_access: u32,
    ) -> ScHandle;

    #[link_name = "OpenServiceW"]
    fn open_service_w(manager: ScHandle, service_name: *const u16, desired_access: u32)
        -> ScHandle;

    #[link_name = "QueryServiceStatusEx"]
    fn query_service_status_ex(
        service: ScHandle,
        info_level: u32,
        buffer: *mut u8,
        buffer_size: u32,
        bytes_needed: *mut u32,
    ) -> i32;

    #[link_name = "CloseServiceHandle"]
    fn close_service_handle(handle: ScHandle) -> i32;
}

struct ServiceHandle(ScHandle);

impl ServiceHandle {
    fn new(raw: ScHandle) -> io::Result<Self> {
        if raw.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }

    const fn raw(&self) -> ScHandle {
        self.0
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = close_service_handle(self.0);
            }
        }
    }
}

pub fn is_service_running(service_name: &str) -> io::Result<bool> {
    if service_name.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "service name must not be empty",
        ));
    }

    let manager =
        ServiceHandle::new(unsafe { open_sc_manager_w(null(), null(), SC_MANAGER_CONNECT) })?;
    let name = wide(service_name);
    let service = ServiceHandle::new(unsafe {
        open_service_w(manager.raw(), name.as_ptr(), SERVICE_QUERY_STATUS)
    })?;

    let mut status = ServiceStatusProcess::default();
    let mut needed = 0_u32;
    let ok = unsafe {
        query_service_status_ex(
            service.raw(),
            SC_STATUS_PROCESS_INFO,
            (&mut status as *mut ServiceStatusProcess).cast(),
            std::mem::size_of::<ServiceStatusProcess>() as u32,
            &mut needed,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(status.current_state == SERVICE_RUNNING)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
