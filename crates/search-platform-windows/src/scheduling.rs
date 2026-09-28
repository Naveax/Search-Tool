use std::ffi::c_void;
use std::io;

type Handle = *mut c_void;

const PROCESS_MODE_BACKGROUND_BEGIN: u32 = 0x0010_0000;

#[link(name = "kernel32")]
extern "system" {
    #[link_name = "GetCurrentProcess"]
    fn get_current_process() -> Handle;
    #[link_name = "SetPriorityClass"]
    fn set_priority_class(process: Handle, priority_class: u32) -> i32;
}

/// Ask Windows to schedule the current process as background work.
///
/// PROCESS_MODE_BACKGROUND_BEGIN lowers CPU and I/O/resource scheduling priority
/// for the process without changing Search Tool's own correctness/throttling rules.
pub fn enter_process_background_mode() -> io::Result<()> {
    let process = unsafe { get_current_process() };
    let ok = unsafe { set_priority_class(process, PROCESS_MODE_BACKGROUND_BEGIN) };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
