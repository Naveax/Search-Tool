use search_core::ResourceSample;
use std::mem::size_of;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FileTime {
    low: u32,
    high: u32,
}

#[repr(C)]
#[allow(dead_code)]
struct MemoryStatusEx {
    length: u32,
    memory_load: u32,
    total_phys: u64,
    avail_phys: u64,
    total_page_file: u64,
    avail_page_file: u64,
    total_virtual: u64,
    avail_virtual: u64,
    avail_extended_virtual: u64,
}

impl Default for MemoryStatusEx {
    fn default() -> Self {
        Self {
            length: size_of::<Self>() as u32,
            memory_load: 0,
            total_phys: 0,
            avail_phys: 0,
            total_page_file: 0,
            avail_page_file: 0,
            total_virtual: 0,
            avail_virtual: 0,
            avail_extended_virtual: 0,
        }
    }
}

#[repr(C)]
struct LastInputInfo {
    cb_size: u32,
    time: u32,
}

#[repr(C)]
#[allow(dead_code)]
struct SystemPowerStatus {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

#[link(name = "kernel32")]
extern "system" {
    #[link_name = "GetSystemTimes"]
    fn get_system_times(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    #[link_name = "GlobalMemoryStatusEx"]
    fn global_memory_status_ex(status: *mut MemoryStatusEx) -> i32;
    #[link_name = "GetTickCount"]
    fn get_tick_count() -> u32;
    #[link_name = "GetSystemPowerStatus"]
    fn get_system_power_status(status: *mut SystemPowerStatus) -> i32;
}

#[link(name = "user32")]
extern "system" {
    #[link_name = "GetLastInputInfo"]
    fn get_last_input_info(info: *mut LastInputInfo) -> i32;
}

#[derive(Debug, Default)]
pub struct WindowsResourceProbe {
    previous_idle: u64,
    previous_kernel: u64,
    previous_user: u64,
    initialized: bool,
}

impl WindowsResourceProbe {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sample(&mut self) -> ResourceSample {
        ResourceSample {
            cpu_busy_percent: self.cpu_busy_percent(),
            memory_load_percent: memory_load_percent(),
            user_idle_ms: user_idle_ms(),
            on_battery: on_battery(),
        }
    }

    fn cpu_busy_percent(&mut self) -> f32 {
        let mut idle = FileTime::default();
        let mut kernel = FileTime::default();
        let mut user = FileTime::default();

        let ok = unsafe { get_system_times(&mut idle, &mut kernel, &mut user) };
        if ok == 0 {
            return 0.0;
        }

        let idle = filetime_to_u64(idle);
        let kernel = filetime_to_u64(kernel);
        let user = filetime_to_u64(user);

        if !self.initialized {
            self.previous_idle = idle;
            self.previous_kernel = kernel;
            self.previous_user = user;
            self.initialized = true;
            return 0.0;
        }

        let idle_delta = idle.saturating_sub(self.previous_idle);
        let kernel_delta = kernel.saturating_sub(self.previous_kernel);
        let user_delta = user.saturating_sub(self.previous_user);

        self.previous_idle = idle;
        self.previous_kernel = kernel;
        self.previous_user = user;

        let total = kernel_delta.saturating_add(user_delta);
        if total == 0 {
            return 0.0;
        }

        let busy = total.saturating_sub(idle_delta);
        ((busy as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as f32
    }
}

fn filetime_to_u64(value: FileTime) -> u64 {
    ((value.high as u64) << 32) | value.low as u64
}

fn memory_load_percent() -> u8 {
    let mut status = MemoryStatusEx::default();
    let ok = unsafe { global_memory_status_ex(&mut status) };
    if ok == 0 {
        0
    } else {
        status.memory_load.min(100) as u8
    }
}

fn user_idle_ms() -> u64 {
    let mut info = LastInputInfo {
        cb_size: size_of::<LastInputInfo>() as u32,
        time: 0,
    };
    let ok = unsafe { get_last_input_info(&mut info) };
    if ok == 0 {
        return u64::MAX;
    }

    let now = unsafe { get_tick_count() };
    now.wrapping_sub(info.time) as u64
}

fn on_battery() -> bool {
    let mut status = SystemPowerStatus {
        ac_line_status: 255,
        battery_flag: 0,
        battery_life_percent: 255,
        system_status_flag: 0,
        battery_life_time: u32::MAX,
        battery_full_life_time: u32::MAX,
    };

    let ok = unsafe { get_system_power_status(&mut status) };
    ok != 0 && status.ac_line_status == 0
}
