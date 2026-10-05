#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod theme;

#[cfg(not(windows))]
fn main() {
    eprintln!("search-tool-gui is only available on Windows");
}

#[cfg(windows)]
mod windows_app {
    use crate::theme::{
        self, Backdrop, BackgroundFit, Density, Palette, Rgb, ThemeMode, ThemePreset, UiTheme,
    };
    use search_core::{
        content_terms, parse_search_query, query_subject, relation_for_query, ItemTypeFilter,
        MultiLiveSearchStore, QueryIntent, TinyIntentModel, FLAG_DIRECTORY,
    };
    use std::{
        env,
        ffi::{c_char, c_void},
        io,
        path::{Path, PathBuf},
        ptr::null_mut,
        slice,
        sync::{
            atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
            Mutex,
        },
        thread,
        time::Instant,
    };

    type Hwnd = *mut c_void;
    type Hinstance = *mut c_void;
    type Hicon = *mut c_void;
    type Hcursor = *mut c_void;
    type Hbrush = *mut c_void;
    type Hfont = *mut c_void;
    type Hdc = *mut c_void;
    type Hgdiobj = *mut c_void;
    type Hmenu = *mut c_void;
    type Hmonitor = *mut c_void;
    type Hkl = *mut c_void;
    type GpImage = *mut c_void;
    type GpGraphics = *mut c_void;
    type GpImageAttributes = *mut c_void;
    type Lparam = isize;
    type Wparam = usize;
    type Lresult = isize;

    const WS_POPUP: u32 = 0x8000_0000;
    const WS_THICKFRAME: u32 = 0x0004_0000;
    const WS_CLIPCHILDREN: u32 = 0x0200_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;
    const WS_CHILD: u32 = 0x4000_0000;
    const WS_TABSTOP: u32 = 0x0001_0000;
    const WS_VSCROLL: u32 = 0x0020_0000;
    const WS_BORDER: u32 = 0x0080_0000;
    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
    const WS_EX_APPWINDOW: u32 = 0x0004_0000;
    const WS_EX_LAYERED: u32 = 0x0008_0000;
    const ES_AUTOHSCROLL: u32 = 0x0080;
    const LBS_NOTIFY: u32 = 0x0001;
    const LBS_OWNERDRAWFIXED: u32 = 0x0010;
    const LBS_HASSTRINGS: u32 = 0x0040;
    const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
    const SS_LEFT: u32 = 0x0000;
    const BS_PUSHBUTTON: u32 = 0x0000;
    const BS_OWNERDRAW: u32 = 0x000B;

    const SW_HIDE: i32 = 0;
    const SW_SHOW: i32 = 5;
    const SW_RESTORE: i32 = 9;
    const SW_SHOWNORMAL: i32 = 1;

    const WM_NCCREATE: u32 = 0x0081;
    const WM_NCDESTROY: u32 = 0x0082;
    const WM_CREATE: u32 = 0x0001;
    const WM_DESTROY: u32 = 0x0002;
    const WM_SIZE: u32 = 0x0005;
    const WM_SETTINGCHANGE: u32 = 0x001A;
    const WM_DISPLAYCHANGE: u32 = 0x007E;
    const WM_DPICHANGED: u32 = 0x02E0;
    const WM_COMMAND: u32 = 0x0111;
    const WM_CLOSE: u32 = 0x0010;
    const WM_HOTKEY: u32 = 0x0312;
    const WM_COPYDATA: u32 = 0x004A;
    const WM_DRAWITEM: u32 = 0x002B;
    const WM_MEASUREITEM: u32 = 0x002C;
    const WM_ERASEBKGND: u32 = 0x0014;
    const WM_CTLCOLORSTATIC: u32 = 0x0138;
    const WM_CTLCOLOREDIT: u32 = 0x0133;
    const WM_CTLCOLORLISTBOX: u32 = 0x0134;
    const WM_CTLCOLORBTN: u32 = 0x0135;
    const WM_NCHITTEST: u32 = 0x0084;
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;
    const WM_SYSKEYDOWN: u32 = 0x0104;
    const WM_SYSKEYUP: u32 = 0x0105;
    const WM_SETFONT: u32 = 0x0030;
    const WM_SHELL_BRIDGE_BEGIN: u32 = 0x8000 + 0x51;
    const WM_SHELL_BRIDGE_CHAR: u32 = 0x8000 + 0x52;
    const WM_SHELL_BRIDGE_KEY: u32 = 0x8000 + 0x53;

    const GWL_EXSTYLE: i32 = -20;
    const GWLP_USERDATA: i32 = -21;
    const EN_CHANGE: usize = 0x0300;
    const BN_CLICKED: usize = 0;
    const LBN_DBLCLK: usize = 2;

    const LB_ADDSTRING: u32 = 0x0180;
    const LB_RESETCONTENT: u32 = 0x0184;
    const LB_SETCURSEL: u32 = 0x0186;
    const LB_GETCURSEL: u32 = 0x0188;
    const LB_GETITEMDATA: u32 = 0x0199;
    const LB_SETITEMDATA: u32 = 0x019A;
    const LB_SETITEMHEIGHT: u32 = 0x01A0;

    const EM_GETSEL: u32 = 0x00B0;
    const EM_SETSEL: u32 = 0x00B1;
    const EM_REPLACESEL: u32 = 0x00C2;
    const EM_SETMARGINS: u32 = 0x00D3;
    const EM_SETCUEBANNER: u32 = 0x1501;

    const WM_CUT: u32 = 0x0300;
    const WM_COPY: u32 = 0x0301;
    const WM_PASTE: u32 = 0x0302;
    const WM_UNDO: u32 = 0x0304;
    const EC_LEFTMARGIN: usize = 0x0001;
    const EC_RIGHTMARGIN: usize = 0x0002;

    const ODS_SELECTED: u32 = 0x0001;
    const DT_LEFT: u32 = 0x0000;
    const DT_CENTER: u32 = 0x0001;
    const DT_VCENTER: u32 = 0x0004;
    const DT_SINGLELINE: u32 = 0x0020;
    const DT_END_ELLIPSIS: u32 = 0x8000;
    const DT_NOPREFIX: u32 = 0x0800;
    const TRANSPARENT: i32 = 1;

    const IDC_ARROW: usize = 32512;
    const MAX_QUERY_U16: i32 = 1024;
    const MAX_RESULTS: usize = 80;

    const HOTKEY_ID: i32 = 0x5345;
    const MOD_ALT: u32 = 0x0001;
    const MOD_CONTROL: u32 = 0x0002;
    const MOD_NOREPEAT: u32 = 0x4000;
    const VK_BACK: u32 = 0x08;
    const VK_TAB: u32 = 0x09;
    const VK_CONTROL: i32 = 0x11;
    const VK_MENU: i32 = 0x12;
    const VK_SPACE: u32 = 0x20;
    const VK_END: u32 = 0x23;
    const VK_HOME: u32 = 0x24;
    const VK_LEFT: u32 = 0x25;
    const VK_UP: u32 = 0x26;
    const VK_RIGHT: u32 = 0x27;
    const VK_DELETE: u32 = 0x2E;
    const VK_LWIN: u32 = 0x5B;
    const VK_RWIN: u32 = 0x5C;
    const VK_ESCAPE: usize = 0x1B;
    const VK_RETURN: usize = 0x0D;
    const VK_DOWN: usize = 0x28;

    const WH_KEYBOARD_LL: i32 = 13;
    const HC_ACTION: i32 = 0;
    const LLKHF_EXTENDED: u32 = 0x01;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const SHELL_BRIDGE_ARM_MS: u64 = 2_000;
    const SHELL_BRIDGE_KEY_CONTROL: isize = 1 << 17;

    static SHELL_BRIDGE_WINDOW: AtomicUsize = AtomicUsize::new(0);
    static SHELL_BRIDGE_ACTIVE: AtomicBool = AtomicBool::new(false);
    static SHELL_BRIDGE_WIN_DOWN: AtomicBool = AtomicBool::new(false);
    static SHELL_BRIDGE_WIN_CHORDED: AtomicBool = AtomicBool::new(false);
    static SHELL_BRIDGE_ARMED_UNTIL: AtomicU64 = AtomicU64::new(0);
    static SHELL_BRIDGE_SCOPE: Mutex<Option<String>> = Mutex::new(None);

    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
    const BASE_DPI: u32 = 96;
    const ERROR_ALREADY_EXISTS: u32 = 183;

    const LWA_ALPHA: u32 = 0x0000_0002;
    const SPI_SETWORKAREA: u32 = 0x002F;
    const SPI_GETWORKAREA: u32 = 0x0030;
    const MONITOR_DEFAULTTONEAREST: u32 = 0x0000_0002;
    const SWP_NOZORDER: u32 = 0x0004;

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWA_BORDER_COLOR: u32 = 34;
    const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
    const DWMWCP_ROUND: i32 = 2;
    const DWMSBT_AUTO: i32 = 0;
    const DWMSBT_NONE: i32 = 1;
    const DWMSBT_MAINWINDOW: i32 = 2;
    const DWMSBT_TRANSIENTWINDOW: i32 = 3;

    const RRF_RT_REG_DWORD: u32 = 0x0000_0018;
    const LOGPIXELSX: i32 = 88;

    const MF_STRING: u32 = 0x0000;
    const MF_SEPARATOR: u32 = 0x0800;
    const MF_CHECKED: u32 = 0x0008;
    const TPM_RIGHTBUTTON: u32 = 0x0002;
    const TPM_RETURNCMD: u32 = 0x0100;
    const CC_RGBINIT: u32 = 0x0000_0001;
    const CC_FULLOPEN: u32 = 0x0000_0002;
    const OFN_FILEMUSTEXIST: u32 = 0x0000_1000;
    const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
    const OFN_EXPLORER: u32 = 0x0008_0000;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    const SWP_SHOWWINDOW: u32 = 0x0040;

    const ID_EDIT: usize = 1;
    const ID_LIST: usize = 2;
    const ID_TITLE: usize = 3;
    const ID_STATUS: usize = 4;
    const ID_SUBTITLE: usize = 5;
    const ID_ALL: usize = 10;
    const ID_FILES: usize = 11;
    const ID_FOLDERS: usize = 12;
    const ID_CONTENT: usize = 13;
    const ID_THEME: usize = 14;
    const CMD_THEME_SYSTEM: usize = 2101;
    const CMD_THEME_DARK: usize = 2102;
    const CMD_THEME_LIGHT: usize = 2103;
    const CMD_BACKDROP_AUTO: usize = 2110;
    const CMD_BACKDROP_ACRYLIC: usize = 2111;
    const CMD_BACKDROP_MICA: usize = 2112;
    const CMD_BACKDROP_NONE: usize = 2113;
    const CMD_OPACITY_60: usize = 2120;
    const CMD_OPACITY_75: usize = 2121;
    const CMD_OPACITY_90: usize = 2122;
    const CMD_OPACITY_100: usize = 2123;
    const CMD_ACCENT: usize = 2130;
    const CMD_DEFAULT_APPS: usize = 2140;
    const CMD_ADVANCED_THEME: usize = 2141;
    const CMD_PRESET_SIGNATURE: usize = 2150;
    const CMD_PRESET_MIDNIGHT: usize = 2151;
    const CMD_PRESET_GRAPHITE: usize = 2152;
    const CMD_PRESET_FROST: usize = 2153;
    const CMD_PRESET_NATIVE: usize = 2154;
    const CMD_BACKGROUND_COLOR: usize = 2160;
    const CMD_SURFACE_COLOR: usize = 2161;
    const CMD_TEXT_COLOR: usize = 2162;
    const CMD_MUTED_COLOR: usize = 2163;
    const CMD_RESET_PALETTE: usize = 2164;
    const CMD_DENSITY_COMPACT: usize = 2170;
    const CMD_DENSITY_COMFORTABLE: usize = 2171;
    const CMD_DENSITY_SPACIOUS: usize = 2172;
    const CMD_SIZE_COMPACT: usize = 2180;
    const CMD_SIZE_STANDARD: usize = 2181;
    const CMD_SIZE_WIDE: usize = 2182;
    const CMD_BACKGROUND_IMAGE: usize = 2190;
    const CMD_BACKGROUND_IMAGE_CLEAR: usize = 2191;
    const CMD_BACKGROUND_FIT_FILL: usize = 2192;
    const CMD_BACKGROUND_FIT_FIT: usize = 2193;
    const CMD_BACKGROUND_FIT_STRETCH: usize = 2194;
    const CMD_BACKGROUND_IMAGE_OPACITY_20: usize = 2195;
    const CMD_BACKGROUND_IMAGE_OPACITY_35: usize = 2196;
    const CMD_BACKGROUND_IMAGE_OPACITY_60: usize = 2197;
    const CMD_BACKGROUND_IMAGE_OPACITY_100: usize = 2198;

    const GDIP_UNIT_PIXEL: i32 = 2;
    const GDIP_COLOR_ADJUST_DEFAULT: i32 = 0;
    const GDIP_COLOR_MATRIX_FLAGS_DEFAULT: i32 = 0;

    const MARGIN: i32 = 20;
    const TITLE_HEIGHT: i32 = 28;
    const SEARCH_HEIGHT: i32 = 44;
    const TAB_HEIGHT: i32 = 32;
    const STATUS_HEIGHT: i32 = 24;

    #[repr(C)]
    struct WndClassExW {
        cb_size: u32,
        style: u32,
        wnd_proc: Option<unsafe extern "system" fn(Hwnd, u32, Wparam, Lparam) -> Lresult>,
        cls_extra: i32,
        wnd_extra: i32,
        instance: Hinstance,
        icon: Hicon,
        cursor: Hcursor,
        background: Hbrush,
        menu_name: *const u16,
        class_name: *const u16,
        icon_small: Hicon,
    }

    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: u32,
        w_param: Wparam,
        l_param: Lparam,
        time: u32,
        pt_x: i32,
        pt_y: i32,
        private: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct KbdLlHookStruct {
        vk_code: u32,
        scan_code: u32,
        flags: u32,
        time: u32,
        extra_info: usize,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct GuiThreadInfo {
        cb_size: u32,
        flags: u32,
        active: Hwnd,
        focus: Hwnd,
        capture: Hwnd,
        menu_owner: Hwnd,
        move_size: Hwnd,
        caret: Hwnd,
        caret_rect: Rect,
    }

    #[repr(C)]
    struct OsVersionInfoW {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform_id: u32,
        csd_version: [u16; 128],
    }

    #[repr(C)]
    struct MonitorInfo {
        cb_size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
    }

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct CreateStructW {
        create_params: *mut c_void,
        instance: Hinstance,
        menu: *mut c_void,
        parent: Hwnd,
        cy: i32,
        cx: i32,
        y: i32,
        x: i32,
        style: i32,
        name: *const u16,
        class: *const u16,
        ex_style: u32,
    }

    #[repr(C)]
    struct CopyDataStruct {
        dw_data: usize,
        cb_data: u32,
        lp_data: *const c_void,
    }

    #[repr(C)]
    struct DrawItemStruct {
        ctl_type: u32,
        ctl_id: u32,
        item_id: u32,
        item_action: u32,
        item_state: u32,
        hwnd_item: Hwnd,
        hdc: Hdc,
        rc_item: Rect,
        item_data: usize,
    }

    #[repr(C)]
    struct MeasureItemStruct {
        ctl_type: u32,
        ctl_id: u32,
        item_id: u32,
        item_width: u32,
        item_height: u32,
        item_data: usize,
    }

    #[repr(C)]
    struct ChooseColorW {
        struct_size: u32,
        owner: Hwnd,
        instance: Hinstance,
        rgb_result: u32,
        custom_colors: *mut u32,
        flags: u32,
        custom_data: Lparam,
        hook: *mut c_void,
        template_name: *const u16,
    }

    #[repr(C)]
    struct GdiplusStartupInput {
        version: u32,
        debug_event_callback: *mut c_void,
        suppress_background_thread: i32,
        suppress_external_codecs: i32,
    }

    #[repr(C)]
    struct ColorMatrix {
        values: [[f32; 5]; 5],
    }

    #[repr(C)]
    struct OpenFileNameW {
        struct_size: u32,
        owner: Hwnd,
        instance: Hinstance,
        filter: *const u16,
        custom_filter: *mut u16,
        max_custom_filter: u32,
        filter_index: u32,
        file: *mut u16,
        max_file: u32,
        file_title: *mut u16,
        max_file_title: u32,
        initial_dir: *const u16,
        title: *const u16,
        flags: u32,
        file_offset: u16,
        file_extension: u16,
        default_extension: *const u16,
        custom_data: Lparam,
        hook: *mut c_void,
        template_name: *const u16,
        reserved: *mut c_void,
        reserved_dword: u32,
        flags_ex: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        #[link_name = "GetModuleHandleW"]
        fn get_module_handle_w(module_name: *const u16) -> Hinstance;
        #[link_name = "GetProcAddress"]
        fn get_proc_address(module: Hinstance, proc_name: *const c_char) -> *mut c_void;
        #[link_name = "CreateMutexW"]
        fn create_mutex_w(
            security_attributes: *mut c_void,
            initial_owner: i32,
            name: *const u16,
        ) -> *mut c_void;
        #[link_name = "GetLastError"]
        fn get_last_error() -> u32;
        #[link_name = "GetTickCount64"]
        fn get_tick_count64() -> u64;
        #[link_name = "OpenProcess"]
        fn open_process(access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
        #[link_name = "QueryFullProcessImageNameW"]
        fn query_full_process_image_name_w(
            process: *mut c_void,
            flags: u32,
            image_name: *mut u16,
            size: *mut u32,
        ) -> i32;
        #[link_name = "CloseHandle"]
        fn close_handle(handle: *mut c_void) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        #[link_name = "RegisterClassExW"]
        fn register_class_ex_w(class: *const WndClassExW) -> u16;
        #[link_name = "CreateWindowExW"]
        fn create_window_ex_w(
            ex_style: u32,
            class_name: *const u16,
            window_name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Hwnd,
            menu: *mut c_void,
            instance: Hinstance,
            param: *mut c_void,
        ) -> Hwnd;
        #[link_name = "DefWindowProcW"]
        fn def_window_proc_w(hwnd: Hwnd, msg: u32, w_param: Wparam, l_param: Lparam) -> Lresult;
        #[link_name = "ShowWindow"]
        fn show_window(hwnd: Hwnd, command: i32) -> i32;
        #[link_name = "UpdateWindow"]
        fn update_window(hwnd: Hwnd) -> i32;
        #[link_name = "DestroyWindow"]
        fn destroy_window(hwnd: Hwnd) -> i32;
        #[link_name = "GetMessageW"]
        fn get_message_w(msg: *mut Msg, hwnd: Hwnd, min: u32, max: u32) -> i32;
        #[link_name = "TranslateMessage"]
        fn translate_message(msg: *const Msg) -> i32;
        #[link_name = "DispatchMessageW"]
        fn dispatch_message_w(msg: *const Msg) -> Lresult;
        #[link_name = "PostQuitMessage"]
        fn post_quit_message(exit_code: i32);
        #[link_name = "LoadCursorW"]
        fn load_cursor_w(instance: Hinstance, cursor_name: *const u16) -> Hcursor;
        #[link_name = "SetWindowLongPtrW"]
        fn set_window_long_ptr_w(hwnd: Hwnd, index: i32, value: isize) -> isize;
        #[link_name = "GetWindowLongPtrW"]
        fn get_window_long_ptr_w(hwnd: Hwnd, index: i32) -> isize;
        #[link_name = "GetClientRect"]
        fn get_client_rect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        #[link_name = "GetWindowRect"]
        fn get_window_rect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        #[link_name = "MoveWindow"]
        fn move_window(hwnd: Hwnd, x: i32, y: i32, width: i32, height: i32, repaint: i32) -> i32;
        #[link_name = "GetWindowTextLengthW"]
        fn get_window_text_length_w(hwnd: Hwnd) -> i32;
        #[link_name = "GetWindowTextW"]
        fn get_window_text_w(hwnd: Hwnd, buffer: *mut u16, max_count: i32) -> i32;
        #[link_name = "SetWindowTextW"]
        fn set_window_text_w(hwnd: Hwnd, text: *const u16) -> i32;
        #[link_name = "SendMessageW"]
        fn send_message_w(hwnd: Hwnd, msg: u32, w_param: Wparam, l_param: Lparam) -> Lresult;
        #[link_name = "SetFocus"]
        fn set_focus(hwnd: Hwnd) -> Hwnd;
        #[link_name = "GetFocus"]
        fn get_focus() -> Hwnd;
        #[link_name = "MessageBoxW"]
        fn message_box_w(hwnd: Hwnd, text: *const u16, caption: *const u16, kind: u32) -> i32;
        #[link_name = "RegisterHotKey"]
        fn register_hot_key(hwnd: Hwnd, id: i32, modifiers: u32, virtual_key: u32) -> i32;
        #[link_name = "UnregisterHotKey"]
        fn unregister_hot_key(hwnd: Hwnd, id: i32) -> i32;
        #[link_name = "SetWindowsHookExW"]
        fn set_windows_hook_ex_w(
            hook_id: i32,
            hook_proc: Option<unsafe extern "system" fn(i32, Wparam, Lparam) -> Lresult>,
            instance: Hinstance,
            thread_id: u32,
        ) -> *mut c_void;
        #[link_name = "UnhookWindowsHookEx"]
        fn unhook_windows_hook_ex(hook: *mut c_void) -> i32;
        #[link_name = "CallNextHookEx"]
        fn call_next_hook_ex(
            hook: *mut c_void,
            code: i32,
            w_param: Wparam,
            l_param: Lparam,
        ) -> Lresult;
        #[link_name = "PostMessageW"]
        fn post_message_w(hwnd: Hwnd, msg: u32, w_param: Wparam, l_param: Lparam) -> i32;
        #[link_name = "GetForegroundWindow"]
        fn get_foreground_window() -> Hwnd;
        #[link_name = "GetWindowThreadProcessId"]
        fn get_window_thread_process_id(hwnd: Hwnd, process_id: *mut u32) -> u32;
        #[link_name = "GetGUIThreadInfo"]
        fn get_gui_thread_info(thread_id: u32, info: *mut GuiThreadInfo) -> i32;
        #[link_name = "GetClassNameW"]
        fn get_class_name_w(hwnd: Hwnd, class_name: *mut u16, max_count: i32) -> i32;
        #[link_name = "GetParent"]
        fn get_parent(hwnd: Hwnd) -> Hwnd;
        #[link_name = "EnumChildWindows"]
        fn enum_child_windows(
            parent: Hwnd,
            callback: Option<unsafe extern "system" fn(Hwnd, Lparam) -> i32>,
            l_param: Lparam,
        ) -> i32;
        #[link_name = "GetAsyncKeyState"]
        fn get_async_key_state(virtual_key: i32) -> i16;
        #[link_name = "GetKeyboardState"]
        fn get_keyboard_state(state: *mut u8) -> i32;
        #[link_name = "GetKeyboardLayout"]
        fn get_keyboard_layout(thread_id: u32) -> Hkl;
        #[link_name = "ToUnicodeEx"]
        fn to_unicode_ex(
            virtual_key: u32,
            scan_code: u32,
            key_state: *const u8,
            buffer: *mut u16,
            buffer_len: i32,
            flags: u32,
            keyboard_layout: Hkl,
        ) -> i32;
        #[link_name = "FindWindowW"]
        fn find_window_w(class_name: *const u16, window_name: *const u16) -> Hwnd;
        #[link_name = "SetForegroundWindow"]
        fn set_foreground_window(hwnd: Hwnd) -> i32;
        #[link_name = "SetProcessDPIAware"]
        fn set_process_dpi_aware() -> i32;
        #[link_name = "GetDC"]
        fn get_dc(hwnd: Hwnd) -> Hdc;
        #[link_name = "ReleaseDC"]
        fn release_dc(hwnd: Hwnd, hdc: Hdc) -> i32;
        #[link_name = "MonitorFromWindow"]
        fn monitor_from_window(hwnd: Hwnd, flags: u32) -> Hmonitor;
        #[link_name = "GetMonitorInfoW"]
        fn get_monitor_info_w(monitor: Hmonitor, info: *mut MonitorInfo) -> i32;
        #[link_name = "SetLayeredWindowAttributes"]
        fn set_layered_window_attributes(hwnd: Hwnd, color: u32, alpha: u8, flags: u32) -> i32;
        #[link_name = "SystemParametersInfoW"]
        fn system_parameters_info_w(action: u32, param: u32, data: *mut c_void, flags: u32) -> i32;
        #[link_name = "SetWindowPos"]
        fn set_window_pos(
            hwnd: Hwnd,
            insert_after: Hwnd,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
        #[link_name = "IsWindowVisible"]
        fn is_window_visible(hwnd: Hwnd) -> i32;
        #[link_name = "InvalidateRect"]
        fn invalidate_rect(hwnd: Hwnd, rect: *const Rect, erase: i32) -> i32;
        #[link_name = "FillRect"]
        fn fill_rect(hdc: Hdc, rect: *const Rect, brush: Hbrush) -> i32;
        #[link_name = "DrawTextW"]
        fn draw_text_w(hdc: Hdc, text: *const u16, count: i32, rect: *mut Rect, format: u32)
            -> i32;
        #[link_name = "ScreenToClient"]
        fn screen_to_client(hwnd: Hwnd, point: *mut Point) -> i32;
        #[link_name = "CreatePopupMenu"]
        fn create_popup_menu() -> Hmenu;
        #[link_name = "AppendMenuW"]
        fn append_menu_w(menu: Hmenu, flags: u32, id: usize, text: *const u16) -> i32;
        #[link_name = "TrackPopupMenu"]
        fn track_popup_menu(
            menu: Hmenu,
            flags: u32,
            x: i32,
            y: i32,
            reserved: i32,
            hwnd: Hwnd,
            rect: *const Rect,
        ) -> i32;
        #[link_name = "DestroyMenu"]
        fn destroy_menu(menu: Hmenu) -> i32;
        #[link_name = "GetCursorPos"]
        fn get_cursor_pos(point: *mut Point) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        #[link_name = "CreateSolidBrush"]
        fn create_solid_brush(color: u32) -> Hbrush;
        #[link_name = "DeleteObject"]
        fn delete_object(object: Hgdiobj) -> i32;
        #[link_name = "SetTextColor"]
        fn set_text_color(hdc: Hdc, color: u32) -> u32;
        #[link_name = "SetBkColor"]
        fn set_bk_color(hdc: Hdc, color: u32) -> u32;
        #[link_name = "SetBkMode"]
        fn set_bk_mode(hdc: Hdc, mode: i32) -> i32;
        #[link_name = "SelectObject"]
        fn select_object(hdc: Hdc, object: Hgdiobj) -> Hgdiobj;
        #[link_name = "GetDeviceCaps"]
        fn get_device_caps(hdc: Hdc, index: i32) -> i32;
        #[link_name = "CreateFontW"]
        fn create_font_w(
            height: i32,
            width: i32,
            escapement: i32,
            orientation: i32,
            weight: i32,
            italic: u32,
            underline: u32,
            strike_out: u32,
            char_set: u32,
            out_precision: u32,
            clip_precision: u32,
            quality: u32,
            pitch_and_family: u32,
            face: *const u16,
        ) -> Hfont;
    }

    #[link(name = "dwmapi")]
    extern "system" {
        #[link_name = "DwmSetWindowAttribute"]
        fn dwm_set_window_attribute(
            hwnd: Hwnd,
            attribute: u32,
            value: *const c_void,
            size: u32,
        ) -> i32;
    }

    #[link(name = "uxtheme")]
    extern "system" {
        #[link_name = "SetWindowTheme"]
        fn set_window_theme(hwnd: Hwnd, sub_app_name: *const u16, sub_id_list: *const u16) -> i32;
    }

    #[link(name = "shell32")]
    extern "system" {
        #[link_name = "ShellExecuteW"]
        fn shell_execute_w(
            hwnd: Hwnd,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show_command: i32,
        ) -> *mut c_void;
    }

    #[link(name = "comdlg32")]
    extern "system" {
        #[link_name = "ChooseColorW"]
        fn choose_color_w(value: *mut ChooseColorW) -> i32;
        #[link_name = "GetOpenFileNameW"]
        fn get_open_file_name_w(value: *mut OpenFileNameW) -> i32;
    }

    #[link(name = "ntdll")]
    extern "system" {
        #[link_name = "RtlGetVersion"]
        fn rtl_get_version(info: *mut OsVersionInfoW) -> i32;
    }

    #[link(name = "gdiplus")]
    extern "system" {
        #[link_name = "GdiplusStartup"]
        fn gdiplus_startup(
            token: *mut usize,
            input: *const GdiplusStartupInput,
            output: *mut c_void,
        ) -> i32;
        #[link_name = "GdiplusShutdown"]
        fn gdiplus_shutdown(token: usize);
        #[link_name = "GdipLoadImageFromFile"]
        fn gdip_load_image_from_file(filename: *const u16, image: *mut GpImage) -> i32;
        #[link_name = "GdipDisposeImage"]
        fn gdip_dispose_image(image: GpImage) -> i32;
        #[link_name = "GdipCreateFromHDC"]
        fn gdip_create_from_hdc(hdc: Hdc, graphics: *mut GpGraphics) -> i32;
        #[link_name = "GdipDeleteGraphics"]
        fn gdip_delete_graphics(graphics: GpGraphics) -> i32;
        #[link_name = "GdipGetImageWidth"]
        fn gdip_get_image_width(image: GpImage, width: *mut u32) -> i32;
        #[link_name = "GdipGetImageHeight"]
        fn gdip_get_image_height(image: GpImage, height: *mut u32) -> i32;
        #[link_name = "GdipCreateImageAttributes"]
        fn gdip_create_image_attributes(attributes: *mut GpImageAttributes) -> i32;
        #[link_name = "GdipDisposeImageAttributes"]
        fn gdip_dispose_image_attributes(attributes: GpImageAttributes) -> i32;
        #[link_name = "GdipSetImageAttributesColorMatrix"]
        fn gdip_set_image_attributes_color_matrix(
            attributes: GpImageAttributes,
            adjust_type: i32,
            enable: i32,
            color_matrix: *const ColorMatrix,
            gray_matrix: *const ColorMatrix,
            flags: i32,
        ) -> i32;
        #[link_name = "GdipDrawImageRectRectI"]
        fn gdip_draw_image_rect_rect_i(
            graphics: GpGraphics,
            image: GpImage,
            dst_x: i32,
            dst_y: i32,
            dst_width: i32,
            dst_height: i32,
            src_x: i32,
            src_y: i32,
            src_width: i32,
            src_height: i32,
            src_unit: i32,
            attributes: GpImageAttributes,
            callback: *mut c_void,
            callback_data: *mut c_void,
        ) -> i32;
    }

    #[link(name = "advapi32")]
    extern "system" {
        #[link_name = "RegGetValueW"]
        fn reg_get_value_w(
            hkey: *mut c_void,
            sub_key: *const u16,
            value: *const u16,
            flags: u32,
            value_type: *mut u32,
            data: *mut c_void,
            data_size: *mut u32,
        ) -> i32;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SearchMode {
        All,
        Files,
        Folders,
        Content,
    }

    struct ResultRow {
        name: String,
        path: String,
        is_directory: bool,
    }

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    struct SearchRequest {
        query: Option<String>,
        scope: Option<String>,
    }

    impl SearchRequest {
        fn is_empty(&self) -> bool {
            self.query.as_deref().is_none_or(str::is_empty)
                && self.scope.as_deref().is_none_or(str::is_empty)
        }
    }

    struct State {
        store: MultiLiveSearchStore,
        edit: Hwnd,
        list: Hwnd,
        title: Hwnd,
        subtitle: Hwnd,
        status: Hwnd,
        tabs: [Hwnd; 4],
        theme_button: Hwnd,
        resident: bool,
        hotkey_registered: bool,
        intent_model: Option<TinyIntentModel>,
        model_path: PathBuf,
        initial_request: Option<SearchRequest>,
        scope: Option<String>,
        mode: SearchMode,
        results: Vec<ResultRow>,
        theme: UiTheme,
        palette: Palette,
        dark: bool,
        background_brush: Hbrush,
        surface_brush: Hbrush,
        accent_brush: Hbrush,
        gdiplus_token: usize,
        background_image: GpImage,
        ui_font: Hfont,
        title_font: Hfont,
        small_font: Hfont,
        dpi: u32,
        os_build: u32,
    }

    impl Drop for State {
        fn drop(&mut self) {
            unsafe {
                if !self.background_image.is_null() {
                    let _ = gdip_dispose_image(self.background_image);
                    self.background_image = null_mut();
                }
                if self.gdiplus_token != 0 {
                    gdiplus_shutdown(self.gdiplus_token);
                    self.gdiplus_token = 0;
                }
                for object in [
                    self.background_brush as Hgdiobj,
                    self.surface_brush as Hgdiobj,
                    self.accent_brush as Hgdiobj,
                    self.ui_font as Hgdiobj,
                    self.title_font as Hgdiobj,
                    self.small_font as Hgdiobj,
                ] {
                    if !object.is_null() {
                        let _ = delete_object(object);
                    }
                }
            }
        }
    }

    fn normalize_dpi(dpi: u32) -> u32 {
        if dpi == 0 {
            BASE_DPI
        } else {
            dpi
        }
    }

    unsafe fn set_process_dpi_awareness_compat() {
        let module_name = wide("user32.dll");
        let module = get_module_handle_w(module_name.as_ptr());
        if !module.is_null() {
            let proc = get_proc_address(module, c"SetProcessDpiAwarenessContext".as_ptr());
            if !proc.is_null() {
                let set_context: unsafe extern "system" fn(isize) -> i32 = std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "system" fn(isize) -> i32,
                >(proc);
                if set_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) != 0 {
                    return;
                }
            }
        }
        let _ = set_process_dpi_aware();
    }

    unsafe fn get_dpi_for_window_compat(hwnd: Hwnd) -> u32 {
        let module_name = wide("user32.dll");
        let module = get_module_handle_w(module_name.as_ptr());
        if !module.is_null() {
            let proc = get_proc_address(module, c"GetDpiForWindow".as_ptr());
            if !proc.is_null() {
                let get_dpi: unsafe extern "system" fn(Hwnd) -> u32 = std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "system" fn(Hwnd) -> u32,
                >(proc);
                let dpi = get_dpi(hwnd);
                if dpi != 0 {
                    return dpi;
                }
            }
        }
        fallback_system_dpi()
    }

    unsafe fn fallback_system_dpi() -> u32 {
        let hdc = get_dc(null_mut());
        if hdc.is_null() {
            return BASE_DPI;
        }
        let dpi = get_device_caps(hdc, LOGPIXELSX);
        let _ = release_dc(null_mut(), hdc);
        if dpi > 0 {
            dpi as u32
        } else {
            BASE_DPI
        }
    }

    unsafe fn windows_build_number() -> u32 {
        let mut info = OsVersionInfoW {
            size: std::mem::size_of::<OsVersionInfoW>() as u32,
            major: 0,
            minor: 0,
            build: 0,
            platform_id: 0,
            csd_version: [0; 128],
        };
        if rtl_get_version(&mut info) == 0 {
            info.build
        } else {
            0
        }
    }

    fn supports_modern_frame(build: u32) -> bool {
        build >= 22_000
    }

    fn supports_system_backdrop(build: u32) -> bool {
        build >= 22_621
    }

    fn platform_label(build: u32) -> &'static str {
        if build >= 22_000 {
            "WINDOWS 11"
        } else if build > 0 {
            "WINDOWS 10"
        } else {
            "WINDOWS COMPAT"
        }
    }

    unsafe fn start_gdiplus() -> usize {
        let input = GdiplusStartupInput {
            version: 1,
            debug_event_callback: null_mut(),
            suppress_background_thread: 0,
            suppress_external_codecs: 0,
        };
        let mut token = 0_usize;
        if gdiplus_startup(&mut token, &input, null_mut()) == 0 {
            token
        } else {
            0
        }
    }

    unsafe fn load_theme_background(theme: &UiTheme, gdiplus_token: usize) -> GpImage {
        if gdiplus_token == 0 {
            return null_mut();
        }
        let Some(path) = theme.background_image.as_ref() else {
            return null_mut();
        };
        if !path.is_file() {
            return null_mut();
        }
        let path = wide(&path.to_string_lossy());
        let mut image = null_mut();
        if gdip_load_image_from_file(path.as_ptr(), &mut image) == 0 {
            image
        } else {
            null_mut()
        }
    }

    unsafe fn reload_background_image(state: &mut State) {
        if !state.background_image.is_null() {
            let _ = gdip_dispose_image(state.background_image);
            state.background_image = null_mut();
        }
        state.background_image = load_theme_background(&state.theme, state.gdiplus_token);
    }

    fn image_destination_rect(
        image_width: u32,
        image_height: u32,
        bounds: Rect,
        fit: BackgroundFit,
    ) -> Rect {
        let target_width = (bounds.right - bounds.left).max(1);
        let target_height = (bounds.bottom - bounds.top).max(1);
        if image_width == 0 || image_height == 0 || fit == BackgroundFit::Stretch {
            return bounds;
        }

        let sx = target_width as f64 / image_width as f64;
        let sy = target_height as f64 / image_height as f64;
        let scale = match fit {
            BackgroundFit::Fit => sx.min(sy),
            BackgroundFit::Fill => sx.max(sy),
            BackgroundFit::Stretch => 1.0,
        };
        let width = ((image_width as f64 * scale).round() as i32).max(1);
        let height = ((image_height as f64 * scale).round() as i32).max(1);
        let left = bounds.left + (target_width - width) / 2;
        let top = bounds.top + (target_height - height) / 2;
        Rect {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    unsafe fn draw_background_image(state: &State, hdc: Hdc, bounds: Rect) {
        if state.background_image.is_null() {
            return;
        }
        let mut image_width = 0_u32;
        let mut image_height = 0_u32;
        if gdip_get_image_width(state.background_image, &mut image_width) != 0
            || gdip_get_image_height(state.background_image, &mut image_height) != 0
            || image_width == 0
            || image_height == 0
        {
            return;
        }

        let mut graphics = null_mut();
        if gdip_create_from_hdc(hdc, &mut graphics) != 0 || graphics.is_null() {
            return;
        }

        let destination = image_destination_rect(
            image_width,
            image_height,
            bounds,
            state.theme.background_fit,
        );
        let mut attributes = null_mut();
        if state.theme.background_image_opacity < 100
            && gdip_create_image_attributes(&mut attributes) == 0
            && !attributes.is_null()
        {
            let alpha = state.theme.background_image_opacity as f32 / 100.0;
            let matrix = ColorMatrix {
                values: [
                    [1.0, 0.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0, alpha, 0.0],
                    [0.0, 0.0, 0.0, 0.0, 1.0],
                ],
            };
            let _ = gdip_set_image_attributes_color_matrix(
                attributes,
                GDIP_COLOR_ADJUST_DEFAULT,
                1,
                &matrix,
                null_mut(),
                GDIP_COLOR_MATRIX_FLAGS_DEFAULT,
            );
        }

        let _ = gdip_draw_image_rect_rect_i(
            graphics,
            state.background_image,
            destination.left,
            destination.top,
            (destination.right - destination.left).max(1),
            (destination.bottom - destination.top).max(1),
            0,
            0,
            image_width.min(i32::MAX as u32) as i32,
            image_height.min(i32::MAX as u32) as i32,
            GDIP_UNIT_PIXEL,
            attributes,
            null_mut(),
            null_mut(),
        );

        if !attributes.is_null() {
            let _ = gdip_dispose_image_attributes(attributes);
        }
        let _ = gdip_delete_graphics(graphics);
    }

    fn scale_px(value: i32, dpi: u32) -> i32 {
        let dpi = normalize_dpi(dpi) as i64;
        let value = value.max(0) as i64;
        ((value * dpi + (BASE_DPI as i64 / 2)) / BASE_DPI as i64).clamp(0, i32::MAX as i64) as i32
    }

    fn centered_window_rect(work: Rect, logical_width: i32, logical_height: i32, dpi: u32) -> Rect {
        let available_width = (work.right - work.left).max(1);
        let available_height = (work.bottom - work.top).max(1);
        let width = scale_px(logical_width, dpi).min(available_width).max(1);
        let height = scale_px(logical_height, dpi).min(available_height).max(1);
        let x = work.left + (available_width - width) / 2;
        let preferred_top = ((available_height - height) / 5).max(scale_px(24, dpi));
        let y = (work.top + preferred_top)
            .min(work.bottom - height)
            .max(work.top);
        Rect {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    fn clamp_window_rect(window: Rect, work: Rect) -> Rect {
        let available_width = (work.right - work.left).max(1);
        let available_height = (work.bottom - work.top).max(1);
        let width = (window.right - window.left).max(1).min(available_width);
        let height = (window.bottom - window.top).max(1).min(available_height);
        let max_left = work.right - width;
        let max_top = work.bottom - height;
        let left = window.left.clamp(work.left, max_left);
        let top = window.top.clamp(work.top, max_top);
        Rect {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    unsafe fn create_fonts_for_dpi(dpi: u32, os_build: u32) -> Option<(Hfont, Hfont, Hfont)> {
        let variable_ui = os_build >= 22_000;
        let font_face = wide(if variable_ui {
            "Segoe UI Variable Text"
        } else {
            "Segoe UI"
        });
        let title_face = wide(if variable_ui {
            "Segoe UI Variable Display"
        } else {
            "Segoe UI Semibold"
        });
        let ui_font = create_font_w(
            -scale_px(18, dpi),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            font_face.as_ptr(),
        );
        let title_font = create_font_w(
            -scale_px(22, dpi),
            0,
            0,
            0,
            600,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            title_face.as_ptr(),
        );
        let small_font = create_font_w(
            -scale_px(14, dpi),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            font_face.as_ptr(),
        );
        if ui_font.is_null() || title_font.is_null() || small_font.is_null() {
            for font in [ui_font, title_font, small_font] {
                if !font.is_null() {
                    let _ = delete_object(font as Hgdiobj);
                }
            }
            return None;
        }
        Some((ui_font, title_font, small_font))
    }

    unsafe fn monitor_work_area(hwnd: Hwnd) -> Option<Rect> {
        let monitor = monitor_from_window(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return None;
        }
        let mut info = MonitorInfo {
            cb_size: std::mem::size_of::<MonitorInfo>() as u32,
            monitor: Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            work: Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            flags: 0,
        };
        (get_monitor_info_w(monitor, &mut info) != 0).then_some(info.work)
    }

    unsafe fn effective_window_dpi(hwnd: Hwnd) -> u32 {
        normalize_dpi(get_dpi_for_window_compat(hwnd))
    }

    fn is_shell_search_process_name(value: &str) -> bool {
        let name = Path::new(value)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(value);
        [
            "SearchApp.exe",
            "SearchHost.exe",
            "SearchUI.exe",
            "StartMenuExperienceHost.exe",
            "ShellExperienceHost.exe",
        ]
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
    }

    fn is_typing_virtual_key(vk: u32) -> bool {
        matches!(
            vk,
            0x20
                | 0x30..=0x5A
                | 0x60..=0x69
                | 0x6A..=0x6F
                | 0xBA..=0xC0
                | 0xDB..=0xE2
                | 0xE7
        )
    }

    fn is_bridge_routable_key(vk: u32) -> bool {
        vk == VK_BACK
            || is_typing_virtual_key(vk)
            || matches!(
                vk,
                VK_TAB
                    | VK_HOME
                    | VK_LEFT
                    | VK_UP
                    | VK_RIGHT
                    | 0x28
                    | VK_END
                    | VK_DELETE
                    | 0x0D
                    | 0x1B
            )
    }

    fn is_explorer_process_name(value: &str) -> bool {
        Path::new(value)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("explorer.exe"))
    }

    fn is_explorer_search_class(value: &str) -> bool {
        [
            "SearchEditBoxWrapperClass",
            "Search Box",
            "UniversalSearchBand",
        ]
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
    }

    fn extract_explorer_scope_from_toolbar_text(text: &str) -> Option<String> {
        let bytes = text.as_bytes();
        for index in 0..bytes.len().saturating_sub(2) {
            if bytes[index].is_ascii_alphabetic()
                && bytes.get(index + 1) == Some(&b':')
                && matches!(bytes.get(index + 2), Some(b'\\' | b'/'))
            {
                return normalize_scope(&text[index..]);
            }
        }
        text.find(r"\\")
            .and_then(|index| normalize_scope(&text[index..]))
    }

    unsafe fn window_process_path(hwnd: Hwnd) -> Option<String> {
        if hwnd.is_null() {
            return None;
        }
        let mut process_id = 0u32;
        let _ = get_window_thread_process_id(hwnd, &mut process_id);
        if process_id == 0 {
            return None;
        }
        let process = open_process(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process.is_null() {
            return None;
        }
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let ok = query_full_process_image_name_w(process, 0, buffer.as_mut_ptr(), &mut size);
        let _ = close_handle(process);
        if ok == 0 || size == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..size as usize]))
    }

    unsafe fn window_class_name(hwnd: Hwnd) -> Option<String> {
        if hwnd.is_null() {
            return None;
        }
        let mut buffer = [0u16; 256];
        let count = get_class_name_w(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        (count > 0).then(|| String::from_utf16_lossy(&buffer[..count as usize]))
    }

    unsafe fn foreground_is_windows_search_surface() -> bool {
        let foreground = get_foreground_window();
        window_process_path(foreground)
            .as_deref()
            .is_some_and(is_shell_search_process_name)
    }

    unsafe fn explorer_search_focus_active(foreground: Hwnd) -> bool {
        let mut process_id = 0u32;
        let thread_id = get_window_thread_process_id(foreground, &mut process_id);
        if thread_id == 0 {
            return false;
        }
        let mut info = GuiThreadInfo {
            cb_size: std::mem::size_of::<GuiThreadInfo>() as u32,
            flags: 0,
            active: null_mut(),
            focus: null_mut(),
            capture: null_mut(),
            menu_owner: null_mut(),
            move_size: null_mut(),
            caret: null_mut(),
            caret_rect: Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
        };
        if get_gui_thread_info(thread_id, &mut info) == 0 || info.focus.is_null() {
            return false;
        }

        let mut current = info.focus;
        for _ in 0..8 {
            if current.is_null() {
                break;
            }
            if window_class_name(current)
                .as_deref()
                .is_some_and(is_explorer_search_class)
            {
                return true;
            }
            current = get_parent(current);
        }
        false
    }

    struct ExplorerScopeProbe {
        scope: Option<String>,
    }

    unsafe extern "system" fn enum_explorer_scope(hwnd: Hwnd, l_param: Lparam) -> i32 {
        let probe = &mut *(l_param as *mut ExplorerScopeProbe);
        if probe.scope.is_some() {
            return 0;
        }
        if !window_class_name(hwnd)
            .as_deref()
            .is_some_and(|class| class.eq_ignore_ascii_case("ToolbarWindow32"))
        {
            return 1;
        }

        let len = get_window_text_length_w(hwnd);
        if len <= 0 || len > 32_768 {
            return 1;
        }
        let mut buffer = vec![0u16; len as usize + 1];
        let copied = get_window_text_w(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        if copied <= 0 {
            return 1;
        }
        let text = String::from_utf16_lossy(&buffer[..copied as usize]);
        if let Some(scope) = extract_explorer_scope_from_toolbar_text(&text) {
            probe.scope = Some(scope);
            return 0;
        }
        1
    }

    unsafe fn explorer_scope_from_window(foreground: Hwnd) -> Option<String> {
        let mut probe = ExplorerScopeProbe { scope: None };
        let _ = enum_child_windows(
            foreground,
            Some(enum_explorer_scope),
            (&mut probe as *mut ExplorerScopeProbe) as Lparam,
        );
        probe.scope
    }

    unsafe fn foreground_explorer_search_scope() -> Option<Option<String>> {
        let foreground = get_foreground_window();
        if !window_process_path(foreground)
            .as_deref()
            .is_some_and(is_explorer_process_name)
            || !explorer_search_focus_active(foreground)
        {
            return None;
        }
        Some(explorer_scope_from_window(foreground))
    }

    unsafe fn foreground_bridge_scope() -> Option<Option<String>> {
        if foreground_is_windows_search_surface() {
            Some(None)
        } else {
            foreground_explorer_search_scope()
        }
    }

    fn ctrl_down() -> bool {
        unsafe { get_async_key_state(VK_CONTROL) < 0 }
    }

    fn alt_down() -> bool {
        unsafe { get_async_key_state(VK_MENU) < 0 }
    }

    fn ctrl_or_alt_down() -> bool {
        ctrl_down() || alt_down()
    }

    fn blocks_text_takeover() -> bool {
        let ctrl = ctrl_down();
        let alt = alt_down();
        ctrl ^ alt
    }

    fn is_supported_control_shortcut(vk: u32) -> bool {
        matches!(vk, 0x41 | 0x43 | 0x56 | 0x58 | 0x5A)
    }

    unsafe fn post_bridge_begin(scope: Option<String>) -> bool {
        let Ok(mut pending_scope) = SHELL_BRIDGE_SCOPE.lock() else {
            return false;
        };
        *pending_scope = scope;
        drop(pending_scope);

        let hwnd = SHELL_BRIDGE_WINDOW.load(Ordering::Acquire) as Hwnd;
        if hwnd.is_null() || post_message_w(hwnd, WM_SHELL_BRIDGE_BEGIN, 0, 0) == 0 {
            if let Ok(mut pending_scope) = SHELL_BRIDGE_SCOPE.lock() {
                *pending_scope = None;
            }
            return false;
        }
        true
    }

    fn take_bridge_scope() -> Option<String> {
        SHELL_BRIDGE_SCOPE
            .lock()
            .ok()
            .and_then(|mut scope| scope.take())
    }

    unsafe fn post_bridge_char(ch: u16) -> bool {
        let hwnd = SHELL_BRIDGE_WINDOW.load(Ordering::Acquire) as Hwnd;
        !hwnd.is_null() && post_message_w(hwnd, WM_SHELL_BRIDGE_CHAR, ch as usize, 0) != 0
    }

    unsafe fn post_bridge_key(vk: u32, scan_code: u32, extended: bool, control: bool) -> bool {
        let hwnd = SHELL_BRIDGE_WINDOW.load(Ordering::Acquire) as Hwnd;
        if hwnd.is_null() {
            return false;
        }
        let mut packed = (scan_code as isize) & 0xFFFF;
        if extended {
            packed |= 1 << 16;
        }
        if control {
            packed |= SHELL_BRIDGE_KEY_CONTROL;
        }
        post_message_w(hwnd, WM_SHELL_BRIDGE_KEY, vk as usize, packed) != 0
    }

    unsafe fn translate_bridge_chars(vk: u32, scan_code: u32) -> Vec<u16> {
        let mut key_state = [0u8; 256];
        if get_keyboard_state(key_state.as_mut_ptr()) == 0 {
            return Vec::new();
        }
        if let Some(slot) = key_state.get_mut(vk as usize) {
            *slot |= 0x80;
        }
        let foreground = get_foreground_window();
        let mut process_id = 0u32;
        let layout_thread = if foreground.is_null() {
            0
        } else {
            get_window_thread_process_id(foreground, &mut process_id)
        };
        let mut buffer = [0u16; 8];
        let count = to_unicode_ex(
            vk,
            scan_code,
            key_state.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            0,
            get_keyboard_layout(layout_thread),
        );
        if count > 0 {
            buffer[..count as usize].to_vec()
        } else {
            Vec::new()
        }
    }

    unsafe fn post_translated_bridge_chars(vk: u32, scan_code: u32) -> bool {
        for ch in translate_bridge_chars(vk, scan_code) {
            if !post_bridge_char(ch) {
                return false;
            }
        }
        true
    }

    fn topmost_window() -> Hwnd {
        std::ptr::with_exposed_provenance_mut::<c_void>(usize::MAX)
    }

    fn notopmost_window() -> Hwnd {
        std::ptr::with_exposed_provenance_mut::<c_void>(usize::MAX - 1)
    }

    unsafe fn bridge_selection(edit: Hwnd) -> (u32, u32) {
        let mut start = 0u32;
        let mut end = 0u32;
        let _ = send_message_w(
            edit,
            EM_GETSEL,
            (&mut start as *mut u32) as Wparam,
            (&mut end as *mut u32) as Lparam,
        );
        (start, end)
    }

    unsafe fn set_bridge_selection(edit: Hwnd, start: u32, end: i32) {
        let _ = send_message_w(edit, EM_SETSEL, start as Wparam, end as Lparam);
    }

    unsafe fn replace_bridge_selection(edit: Hwnd, units: &[u16]) {
        let mut replacement = Vec::with_capacity(units.len() + 1);
        replacement.extend_from_slice(units);
        replacement.push(0);
        let _ = send_message_w(edit, EM_REPLACESEL, 1, replacement.as_ptr() as Lparam);
    }

    unsafe fn read_bridge_text(edit: Hwnd) -> Vec<u16> {
        let len = get_window_text_length_w(edit).max(0) as usize;
        let mut buffer = vec![0u16; len.saturating_add(1)];
        let copied =
            get_window_text_w(edit, buffer.as_mut_ptr(), buffer.len() as i32).max(0) as usize;
        buffer.truncate(copied);
        buffer
    }

    fn previous_utf16_boundary(text: &[u16], caret: usize) -> usize {
        if caret == 0 {
            return 0;
        }
        let mut previous = caret - 1;
        if text
            .get(previous)
            .is_some_and(|unit| (0xDC00..=0xDFFF).contains(unit))
            && previous > 0
            && text
                .get(previous - 1)
                .is_some_and(|unit| (0xD800..=0xDBFF).contains(unit))
        {
            previous -= 1;
        }
        previous
    }

    unsafe fn append_bridge_utf16(edit: Hwnd, unit: u16) {
        let (selection_start, selection_end) = bridge_selection(edit);

        if unit == VK_BACK as u16 {
            if selection_start != selection_end {
                replace_bridge_selection(edit, &[]);
                return;
            }
            if selection_start == 0 {
                return;
            }
            let text = read_bridge_text(edit);
            let caret = (selection_start as usize).min(text.len());
            let previous = previous_utf16_boundary(&text, caret);
            set_bridge_selection(edit, previous as u32, caret as i32);
            replace_bridge_selection(edit, &[]);
            return;
        }

        let text_len = get_window_text_length_w(edit).max(0) as usize;
        let selected = selection_end.saturating_sub(selection_start) as usize;
        if text_len.saturating_sub(selected) >= MAX_QUERY_U16.saturating_sub(1) as usize {
            return;
        }
        replace_bridge_selection(edit, &[unit]);
    }

    unsafe fn bridge_control_shortcut(edit: Hwnd, vk: u32) {
        match vk {
            value if value == b'A' as u32 => set_bridge_selection(edit, 0, -1),
            value if value == b'C' as u32 => {
                let _ = send_message_w(edit, WM_COPY, 0, 0);
            }
            value if value == b'V' as u32 => {
                let _ = send_message_w(edit, WM_PASTE, 0, 0);
            }
            value if value == b'X' as u32 => {
                let _ = send_message_w(edit, WM_CUT, 0, 0);
            }
            value if value == b'Z' as u32 => {
                let _ = send_message_w(edit, WM_UNDO, 0, 0);
            }
            _ => {}
        }
    }

    unsafe fn show_bridge_overlay(hwnd: Hwnd, state: &mut State) {
        center_search_window(hwnd, state.theme.width, state.theme.height, state.dpi);
        let _ = set_window_pos(
            hwnd,
            topmost_window(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        update_window(hwnd);
    }

    unsafe fn hide_bridge_overlay(hwnd: Hwnd, state: &mut State) {
        SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
        if let Ok(mut pending_scope) = SHELL_BRIDGE_SCOPE.lock() {
            *pending_scope = None;
        }
        let _ = set_window_pos(
            hwnd,
            notopmost_window(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        show_window(hwnd, SW_HIDE);
        state.intent_model = None;
    }

    unsafe extern "system" fn shell_keyboard_proc(
        code: i32,
        w_param: Wparam,
        l_param: Lparam,
    ) -> Lresult {
        if code != HC_ACTION || l_param == 0 {
            return call_next_hook_ex(null_mut(), code, w_param, l_param);
        }

        let event = &*(l_param as *const KbdLlHookStruct);
        let message = w_param as u32;
        let key_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
        let key_up = message == WM_KEYUP || message == WM_SYSKEYUP;
        if !key_down && !key_up {
            return call_next_hook_ex(null_mut(), code, w_param, l_param);
        }

        let vk = event.vk_code;
        let now = get_tick_count64();

        if SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire) {
            if vk == VK_LWIN || vk == VK_RWIN {
                if key_down {
                    SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                    let _ = post_bridge_key(VK_ESCAPE as u32, 0, false, false);
                }
                return call_next_hook_ex(null_mut(), code, w_param, l_param);
            }

            if vk == VK_ESCAPE as u32 {
                if key_down {
                    SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                    let _ = post_bridge_key(vk, event.scan_code, false, false);
                }
                return call_next_hook_ex(null_mut(), code, w_param, l_param);
            }

            let ctrl = ctrl_down();
            let alt = alt_down();
            if ctrl && !alt && is_supported_control_shortcut(vk) {
                if key_down
                    && !post_bridge_key(
                        vk,
                        event.scan_code,
                        event.flags & LLKHF_EXTENDED != 0,
                        true,
                    )
                {
                    SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                    return call_next_hook_ex(null_mut(), code, w_param, l_param);
                }
                return 1;
            }

            if (ctrl ^ alt) && vk != VK_CONTROL as u32 && vk != VK_MENU as u32 {
                if key_down {
                    SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                    let _ = post_bridge_key(VK_ESCAPE as u32, 0, false, false);
                }
                return call_next_hook_ex(null_mut(), code, w_param, l_param);
            }

            if is_typing_virtual_key(vk) {
                if key_down && !post_translated_bridge_chars(vk, event.scan_code) {
                    SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                    return call_next_hook_ex(null_mut(), code, w_param, l_param);
                }
                return 1;
            }

            if vk == VK_BACK {
                if key_down && !post_bridge_char(VK_BACK as u16) {
                    SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                    return call_next_hook_ex(null_mut(), code, w_param, l_param);
                }
                return 1;
            }

            if is_bridge_routable_key(vk) {
                if key_down
                    && !post_bridge_key(
                        vk,
                        event.scan_code,
                        event.flags & LLKHF_EXTENDED != 0,
                        false,
                    )
                {
                    SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                    return call_next_hook_ex(null_mut(), code, w_param, l_param);
                }
                return 1;
            }
            return call_next_hook_ex(null_mut(), code, w_param, l_param);
        }

        if vk == VK_LWIN || vk == VK_RWIN {
            if key_down {
                if !SHELL_BRIDGE_WIN_DOWN.swap(true, Ordering::AcqRel) {
                    SHELL_BRIDGE_WIN_CHORDED.store(false, Ordering::Release);
                }
                SHELL_BRIDGE_ARMED_UNTIL.store(0, Ordering::Release);
            } else if SHELL_BRIDGE_WIN_DOWN.swap(false, Ordering::AcqRel) {
                let chorded = SHELL_BRIDGE_WIN_CHORDED.swap(false, Ordering::AcqRel);
                if !chorded && !ctrl_or_alt_down() {
                    SHELL_BRIDGE_ARMED_UNTIL
                        .store(now.saturating_add(SHELL_BRIDGE_ARM_MS), Ordering::Release);
                }
            }
            return call_next_hook_ex(null_mut(), code, w_param, l_param);
        }

        if SHELL_BRIDGE_WIN_DOWN.load(Ordering::Acquire) && key_down {
            SHELL_BRIDGE_WIN_CHORDED.store(true, Ordering::Release);
            SHELL_BRIDGE_ARMED_UNTIL.store(0, Ordering::Release);
            return call_next_hook_ex(null_mut(), code, w_param, l_param);
        }

        if key_down && (vk == VK_CONTROL as u32 || vk == VK_MENU as u32) {
            SHELL_BRIDGE_ARMED_UNTIL.store(0, Ordering::Release);
            return call_next_hook_ex(null_mut(), code, w_param, l_param);
        }

        let armed_until = SHELL_BRIDGE_ARMED_UNTIL.load(Ordering::Acquire);
        let bridge_scope = if key_down && is_typing_virtual_key(vk) && !blocks_text_takeover() {
            foreground_bridge_scope()
        } else {
            None
        };
        if let Some(scope) = bridge_scope {
            SHELL_BRIDGE_ARMED_UNTIL.store(0, Ordering::Release);
            SHELL_BRIDGE_ACTIVE.store(true, Ordering::Release);
            if post_bridge_begin(scope) && post_translated_bridge_chars(vk, event.scan_code) {
                return 1;
            }
            SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
        } else if key_down && armed_until >= now && !is_typing_virtual_key(vk) {
            SHELL_BRIDGE_ARMED_UNTIL.store(0, Ordering::Release);
        }

        call_next_hook_ex(null_mut(), code, w_param, l_param)
    }

    fn start_shell_keyboard_bridge(hwnd: Hwnd) {
        SHELL_BRIDGE_WINDOW.store(hwnd as usize, Ordering::Release);
        let instance = unsafe { get_module_handle_w(null_mut()) } as usize;
        thread::spawn(move || unsafe {
            let hook = set_windows_hook_ex_w(
                WH_KEYBOARD_LL,
                Some(shell_keyboard_proc),
                instance as Hinstance,
                0,
            );
            if hook.is_null() {
                return;
            }
            let mut msg = std::mem::zeroed::<Msg>();
            loop {
                let status = get_message_w(&mut msg, null_mut(), 0, 0);
                if status <= 0 {
                    break;
                }
                let _ = translate_message(&msg);
                let _ = dispatch_message_w(&msg);
            }
            let _ = unhook_windows_hook_ex(hook);
        });
    }

    struct MutexGuard(*mut c_void);

    impl Drop for MutexGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                let _ = unsafe { close_handle(self.0) };
            }
        }
    }

    pub fn run() -> io::Result<()> {
        unsafe {
            set_process_dpi_awareness_compat();
        }

        theme::ensure_default_config();
        let ui_theme = UiTheme::load();
        let dark = match ui_theme.mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => system_prefers_dark(),
        };
        let palette = ui_theme.palette(dark);
        let os_build = unsafe { windows_build_number() };

        let mut resident = false;
        let mut shell_bridge = true;
        let mut smoke = false;
        let mut ui_preview = false;
        let mut index_source = None;
        let mut initial_request = SearchRequest::default();
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--resident" => resident = true,
                "--no-shell-bridge" => shell_bridge = false,
                "--smoke" => smoke = true,
                "--ui-preview" => ui_preview = true,
                "--query" => {
                    if let Some(value) = args.next() {
                        let value = value.trim().to_string();
                        if !value.is_empty() {
                            initial_request.query = Some(value);
                        }
                    }
                }
                "--scope" => {
                    if let Some(value) = args.next() {
                        initial_request.scope = normalize_scope(value);
                    }
                }
                "--search-uri" => {
                    if let Some(value) = args.next() {
                        if let Some(request) = parse_search_uri(&value) {
                            initial_request = request;
                        }
                    }
                }
                _ if is_search_uri(&arg) => {
                    if let Some(request) = parse_search_uri(&arg) {
                        initial_request = request;
                    }
                }
                _ if index_source.is_none() => index_source = Some(PathBuf::from(arg)),
                _ => {}
            }
        }
        let initial_request = (!initial_request.is_empty()).then_some(initial_request);
        let index_source = index_source.unwrap_or_else(default_index_dir);

        let mutex_name = wide(r"Local\SearchToolGui");
        let mutex = unsafe { create_mutex_w(null_mut(), 0, mutex_name.as_ptr()) };
        if mutex.is_null() {
            return Err(io::Error::last_os_error());
        }
        let mutex = MutexGuard(mutex);
        if unsafe { get_last_error() } == ERROR_ALREADY_EXISTS {
            let class_name = wide("SearchToolWindow");
            let existing = unsafe { find_window_w(class_name.as_ptr(), null_mut()) };
            if !existing.is_null() {
                if let Some(request) = initial_request.as_ref() {
                    unsafe { send_request_to_existing(existing, request) };
                } else if !resident {
                    unsafe { send_request_to_existing(existing, &SearchRequest::default()) };
                }
                unsafe {
                    show_window(existing, SW_RESTORE);
                    set_foreground_window(existing);
                }
            }
            drop(mutex);
            return Ok(());
        }

        let store = if smoke || ui_preview {
            MultiLiveSearchStore::default()
        } else if index_source.is_dir() {
            MultiLiveSearchStore::open_index_directory(&index_source)?
        } else {
            MultiLiveSearchStore::open_index(&index_source)?
        };

        let (ui_font, title_font, small_font) = unsafe { create_fonts_for_dpi(BASE_DPI, os_build) }
            .ok_or_else(io::Error::last_os_error)?;

        let background_brush = unsafe { create_solid_brush(palette.background.colorref()) };
        let surface_brush = unsafe { create_solid_brush(palette.surface.colorref()) };
        let accent_brush = unsafe { create_solid_brush(palette.accent.colorref()) };
        if background_brush.is_null() || surface_brush.is_null() || accent_brush.is_null() {
            return Err(io::Error::last_os_error());
        }
        let gdiplus_token = unsafe { start_gdiplus() };
        let background_image = unsafe { load_theme_background(&ui_theme, gdiplus_token) };

        let mut state = Box::new(State {
            store,
            edit: null_mut(),
            list: null_mut(),
            title: null_mut(),
            subtitle: null_mut(),
            status: null_mut(),
            tabs: [null_mut(); 4],
            theme_button: null_mut(),
            resident,
            hotkey_registered: false,
            intent_model: None,
            model_path: default_model_path(),
            initial_request,
            scope: None,
            mode: SearchMode::All,
            results: Vec::new(),
            theme: ui_theme,
            palette,
            dark,
            background_brush,
            surface_brush,
            accent_brush,
            gdiplus_token,
            background_image,
            ui_font,
            title_font,
            small_font,
            dpi: BASE_DPI,
            os_build,
        });

        let instance = unsafe { get_module_handle_w(null_mut()) };
        if instance.is_null() {
            return Err(io::Error::last_os_error());
        }

        let class_name = wide("SearchToolWindow");
        let class = WndClassExW {
            cb_size: std::mem::size_of::<WndClassExW>() as u32,
            style: 0,
            wnd_proc: Some(window_proc),
            cls_extra: 0,
            wnd_extra: 0,
            instance,
            icon: null_mut(),
            cursor: unsafe { load_cursor_w(null_mut(), IDC_ARROW as *const u16) },
            background: state.background_brush,
            menu_name: null_mut(),
            class_name: class_name.as_ptr(),
            icon_small: null_mut(),
        };
        if unsafe { register_class_ex_w(&class) } == 0 {
            return Err(io::Error::last_os_error());
        }

        let title = wide("Search Tool");
        let raw_state: *mut State = &mut *state;
        let ex_style = (if state.theme.alpha() < 255 {
            WS_EX_LAYERED
        } else {
            0
        }) | if resident {
            WS_EX_TOOLWINDOW
        } else {
            WS_EX_APPWINDOW
        };
        let hwnd = unsafe {
            create_window_ex_w(
                ex_style,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_THICKFRAME | WS_CLIPCHILDREN,
                0,
                0,
                state.theme.width,
                state.theme.height,
                null_mut(),
                null_mut(),
                instance,
                raw_state.cast(),
            )
        };
        if hwnd.is_null() {
            return Err(io::Error::last_os_error());
        }

        std::mem::forget(state);
        unsafe {
            apply_window_composition(hwnd, raw_state);
            let initial_dpi = effective_window_dpi(hwnd);
            apply_dpi(hwnd, &mut *raw_state, initial_dpi);
            center_search_window(
                hwnd,
                (*raw_state).theme.width,
                (*raw_state).theme.height,
                (*raw_state).dpi,
            );

            if smoke || ((*raw_state).resident && (*raw_state).initial_request.is_none()) {
                show_window(hwnd, SW_HIDE);
            } else {
                show_window(hwnd, SW_SHOW);
                update_window(hwnd);
            }
        }
        if smoke {
            if unsafe { destroy_window(hwnd) } == 0 {
                return Err(io::Error::last_os_error());
            }
            return Ok(());
        }
        if resident && shell_bridge {
            start_shell_keyboard_bridge(hwnd);
        }

        let mut msg = Msg {
            hwnd: null_mut(),
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            pt_x: 0,
            pt_y: 0,
            private: 0,
        };
        loop {
            let status = unsafe { get_message_w(&mut msg, null_mut(), 0, 0) };
            if status == -1 {
                return Err(io::Error::last_os_error());
            }
            if status == 0 {
                break;
            }

            if msg.message == WM_KEYDOWN {
                match msg.w_param {
                    VK_ESCAPE => unsafe {
                        let state_ptr = get_window_long_ptr_w(hwnd, GWLP_USERDATA) as *mut State;
                        if !state_ptr.is_null() && (*state_ptr).resident {
                            hide_bridge_overlay(hwnd, &mut *state_ptr);
                            continue;
                        }
                    },
                    VK_RETURN => unsafe {
                        let state_ptr = get_window_long_ptr_w(hwnd, GWLP_USERDATA) as *mut State;
                        if !state_ptr.is_null() && open_selected(hwnd, &mut *state_ptr) {
                            continue;
                        }
                    },
                    VK_DOWN => unsafe {
                        let state_ptr = get_window_long_ptr_w(hwnd, GWLP_USERDATA) as *mut State;
                        if !state_ptr.is_null()
                            && get_focus() == (*state_ptr).edit
                            && !(*state_ptr).results.is_empty()
                        {
                            send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0);
                            set_focus((*state_ptr).list);
                            continue;
                        }
                    },
                    _ => {}
                }
            }

            unsafe {
                translate_message(&msg);
                dispatch_message_w(&msg);
            }
        }
        Ok(())
    }

    unsafe extern "system" fn window_proc(
        hwnd: Hwnd,
        msg: u32,
        w_param: Wparam,
        l_param: Lparam,
    ) -> Lresult {
        if msg == WM_NCCREATE {
            let create = &*(l_param as *const CreateStructW);
            set_window_long_ptr_w(hwnd, GWLP_USERDATA, create.create_params as isize);
        }
        let state_ptr = get_window_long_ptr_w(hwnd, GWLP_USERDATA) as *mut State;

        match msg {
            WM_CREATE if !state_ptr.is_null() => {
                let state = &mut *state_ptr;
                if create_controls(hwnd, state) != 0 {
                    return -1;
                }
                apply_control_theme(state);
                resize_controls(hwnd, state);

                if state.resident {
                    let alt_space =
                        register_hot_key(hwnd, HOTKEY_ID, MOD_ALT | MOD_NOREPEAT, VK_SPACE);
                    let fallback = if alt_space == 0 {
                        register_hot_key(
                            hwnd,
                            HOTKEY_ID,
                            MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                            VK_SPACE,
                        )
                    } else {
                        alt_space
                    };
                    state.hotkey_registered = fallback != 0;
                }

                if let Some(request) = state.initial_request.take() {
                    apply_search_request(state, request);
                }
                set_focus(state.edit);
                0
            }
            WM_SIZE if !state_ptr.is_null() => {
                resize_controls(hwnd, &mut *state_ptr);
                0
            }
            WM_DPICHANGED if !state_ptr.is_null() => {
                let state = &mut *state_ptr;
                let new_dpi = normalize_dpi(w_param as u32 & 0xffff);
                apply_dpi(hwnd, state, new_dpi);
                if l_param != 0 {
                    let suggested = *(l_param as *const Rect);
                    let _ = set_window_pos(
                        hwnd,
                        null_mut(),
                        suggested.left,
                        suggested.top,
                        (suggested.right - suggested.left).max(1),
                        (suggested.bottom - suggested.top).max(1),
                        SWP_NOZORDER,
                    );
                } else {
                    recover_window_to_monitor(hwnd, state);
                }
                resize_controls(hwnd, state);
                0
            }
            WM_DISPLAYCHANGE if !state_ptr.is_null() => {
                recover_window_to_monitor(hwnd, &mut *state_ptr);
                0
            }
            WM_SETTINGCHANGE if !state_ptr.is_null() && (w_param as u32 == SPI_SETWORKAREA) => {
                recover_window_to_monitor(hwnd, &mut *state_ptr);
                0
            }
            WM_COMMAND if !state_ptr.is_null() => {
                let notification = (w_param >> 16) & 0xffff;
                let source = l_param as Hwnd;
                let state = &mut *state_ptr;
                if source == state.edit && notification == EN_CHANGE {
                    refresh_results(state);
                    return 0;
                }
                if notification == BN_CLICKED {
                    let mode = if source == state.tabs[0] {
                        Some(SearchMode::All)
                    } else if source == state.tabs[1] {
                        Some(SearchMode::Files)
                    } else if source == state.tabs[2] {
                        Some(SearchMode::Folders)
                    } else if source == state.tabs[3] {
                        Some(SearchMode::Content)
                    } else {
                        None
                    };
                    if let Some(mode) = mode {
                        state.mode = mode;
                        update_tab_labels(state);
                        refresh_results(state);
                        return 0;
                    }
                    if source == state.theme_button {
                        show_theme_menu(hwnd, state);
                        return 0;
                    }
                }
                if source == state.list && notification == LBN_DBLCLK {
                    let _ = open_selected(hwnd, state);
                }
                0
            }
            WM_SHELL_BRIDGE_BEGIN if !state_ptr.is_null() => {
                let state = &mut *state_ptr;
                state.scope = take_bridge_scope();
                let empty = wide("");
                let _ = set_window_text_w(state.edit, empty.as_ptr());
                state.intent_model = None;
                show_bridge_overlay(hwnd, state);
                0
            }
            WM_SHELL_BRIDGE_CHAR if !state_ptr.is_null() => {
                append_bridge_utf16((*state_ptr).edit, w_param as u16);
                0
            }
            WM_SHELL_BRIDGE_KEY if !state_ptr.is_null() => {
                let state = &mut *state_ptr;
                let scan_code = (l_param as u32) & 0xFFFF;
                let extended = ((l_param as u32) & (1 << 16)) != 0;
                let control = (l_param & SHELL_BRIDGE_KEY_CONTROL) != 0;
                let vk = w_param as u32;
                let key_lparam = 1isize
                    | ((scan_code as isize & 0xFF) << 16)
                    | if extended { 1isize << 24 } else { 0 };

                if vk == VK_ESCAPE as u32 {
                    hide_bridge_overlay(hwnd, state);
                } else if control {
                    bridge_control_shortcut(state.edit, vk);
                } else if vk == VK_RETURN as u32 {
                    let _ = open_selected(hwnd, state);
                } else {
                    let _ = send_message_w(state.edit, WM_KEYDOWN, vk as usize, key_lparam);
                }
                0
            }
            WM_HOTKEY if !state_ptr.is_null() && w_param as i32 == HOTKEY_ID => {
                let state = &mut *state_ptr;
                if is_window_visible(hwnd) != 0 {
                    hide_bridge_overlay(hwnd, state);
                } else {
                    SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                    state.scope = None;
                    center_search_window(hwnd, state.theme.width, state.theme.height, state.dpi);
                    show_window(hwnd, SW_RESTORE);
                    set_foreground_window(hwnd);
                    refresh_results(state);
                    set_focus(state.edit);
                }
                0
            }
            WM_COPYDATA if !state_ptr.is_null() => {
                let state = &mut *state_ptr;
                let copy = &*(l_param as *const CopyDataStruct);
                let bytes = copy.cb_data as usize;
                if !copy.lp_data.is_null() && bytes >= 2 && bytes.is_multiple_of(2) {
                    let words = slice::from_raw_parts(copy.lp_data as *const u16, bytes / 2);
                    let request = match copy.dw_data {
                        1 if bytes <= ((MAX_QUERY_U16 as usize + 1) * 2) => {
                            let end = words
                                .iter()
                                .position(|&value| value == 0)
                                .unwrap_or(words.len());
                            let query = String::from_utf16_lossy(&words[..end]);
                            Some(SearchRequest {
                                query: (!query.trim().is_empty()).then(|| query.trim().to_string()),
                                scope: None,
                            })
                        }
                        2 if words.len() <= 65_536 => decode_ipc_request(words),
                        _ => None,
                    };
                    if let Some(request) = request {
                        apply_search_request(state, request);
                        center_search_window(
                            hwnd,
                            state.theme.width,
                            state.theme.height,
                            state.dpi,
                        );
                        show_window(hwnd, SW_RESTORE);
                        set_foreground_window(hwnd);
                        set_focus(state.edit);
                        return 1;
                    }
                }
                0
            }
            WM_MEASUREITEM if !state_ptr.is_null() => {
                let measure = &mut *(l_param as *mut MeasureItemStruct);
                if measure.ctl_id as usize == ID_LIST {
                    measure.item_height =
                        scale_px((*state_ptr).theme.result_row_height(), (*state_ptr).dpi).max(1)
                            as u32;
                    return 1;
                }
                0
            }
            WM_DRAWITEM if !state_ptr.is_null() => {
                let draw = &*(l_param as *const DrawItemStruct);
                if draw.ctl_id as usize == ID_LIST {
                    draw_result_row(&*state_ptr, draw);
                    return 1;
                }
                if [ID_ALL, ID_FILES, ID_FOLDERS, ID_CONTENT].contains(&(draw.ctl_id as usize)) {
                    draw_filter_chip(&*state_ptr, draw);
                    return 1;
                }
                0
            }
            WM_ERASEBKGND if !state_ptr.is_null() => {
                let state = &*state_ptr;
                let mut rect = Rect {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                if get_client_rect(hwnd, &mut rect) != 0 {
                    fill_rect(w_param as Hdc, &rect, state.background_brush);
                    draw_background_image(state, w_param as Hdc, rect);
                    return 1;
                }
                0
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN if !state_ptr.is_null() => {
                let state = &*state_ptr;
                let hdc = w_param as Hdc;
                set_text_color(hdc, state.palette.text.colorref());
                set_bk_color(hdc, state.palette.background.colorref());
                set_bk_mode(hdc, TRANSPARENT);
                state.background_brush as Lresult
            }
            WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX if !state_ptr.is_null() => {
                let state = &*state_ptr;
                let hdc = w_param as Hdc;
                set_text_color(hdc, state.palette.text.colorref());
                set_bk_color(hdc, state.palette.surface.colorref());
                state.surface_brush as Lresult
            }
            WM_NCHITTEST => {
                let result = def_window_proc_w(hwnd, msg, w_param, l_param);
                if result != 1 {
                    return result;
                }
                let mut point = Point {
                    x: (l_param as i16) as i32,
                    y: ((l_param >> 16) as i16) as i32,
                };
                if screen_to_client(hwnd, &mut point) != 0
                    && point.y >= 0
                    && point.y
                        < scale_px(
                            38,
                            if state_ptr.is_null() {
                                BASE_DPI
                            } else {
                                (*state_ptr).dpi
                            },
                        )
                {
                    return 2;
                }
                result
            }
            WM_CLOSE if !state_ptr.is_null() && (*state_ptr).resident => {
                hide_bridge_overlay(hwnd, &mut *state_ptr);
                0
            }
            WM_DESTROY => {
                post_quit_message(0);
                0
            }
            WM_NCDESTROY => {
                SHELL_BRIDGE_WINDOW.store(0, Ordering::Release);
                SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                SHELL_BRIDGE_ARMED_UNTIL.store(0, Ordering::Release);
                if let Ok(mut pending_scope) = SHELL_BRIDGE_SCOPE.lock() {
                    *pending_scope = None;
                }
                if !state_ptr.is_null() {
                    let state = &mut *state_ptr;
                    if state.hotkey_registered {
                        let _ = unregister_hot_key(hwnd, HOTKEY_ID);
                        state.hotkey_registered = false;
                    }
                    set_window_long_ptr_w(hwnd, GWLP_USERDATA, 0);
                    drop(Box::from_raw(state_ptr));
                }
                def_window_proc_w(hwnd, msg, w_param, l_param)
            }
            _ => def_window_proc_w(hwnd, msg, w_param, l_param),
        }
    }

    unsafe fn create_controls(hwnd: Hwnd, state: &mut State) -> Lresult {
        let instance = get_module_handle_w(null_mut());
        let static_class = wide("STATIC");
        let edit_class = wide("EDIT");
        let list_class = wide("LISTBOX");
        let button_class = wide("BUTTON");
        let empty = wide("");

        state.title = create_window_ex_w(
            0,
            static_class.as_ptr(),
            wide("SEARCH TOOL").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            0,
            0,
            100,
            20,
            hwnd,
            menu_id(ID_TITLE),
            instance,
            null_mut(),
        );
        let subtitle = wide(&format!(
            "LOCAL  •  INSTANT  •  PRIVATE  •  {}",
            platform_label(state.os_build)
        ));
        state.subtitle = create_window_ex_w(
            0,
            static_class.as_ptr(),
            subtitle.as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            0,
            0,
            100,
            18,
            hwnd,
            menu_id(ID_SUBTITLE),
            instance,
            null_mut(),
        );
        state.edit = create_window_ex_w(
            0,
            edit_class.as_ptr(),
            empty.as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL,
            0,
            0,
            100,
            SEARCH_HEIGHT,
            hwnd,
            menu_id(ID_EDIT),
            instance,
            null_mut(),
        );
        state.list = create_window_ex_w(
            0,
            list_class.as_ptr(),
            empty.as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_VSCROLL
                | LBS_NOTIFY
                | LBS_OWNERDRAWFIXED
                | LBS_HASSTRINGS
                | LBS_NOINTEGRALHEIGHT,
            0,
            0,
            100,
            100,
            hwnd,
            menu_id(ID_LIST),
            instance,
            null_mut(),
        );
        state.status = create_window_ex_w(
            0,
            static_class.as_ptr(),
            wide("Hazır  •  Yerel index  •  Bulut yok").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            0,
            0,
            100,
            STATUS_HEIGHT,
            hwnd,
            menu_id(ID_STATUS),
            instance,
            null_mut(),
        );

        for (index, id) in [ID_ALL, ID_FILES, ID_FOLDERS, ID_CONTENT]
            .into_iter()
            .enumerate()
        {
            state.tabs[index] = create_window_ex_w(
                0,
                button_class.as_ptr(),
                empty.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
                0,
                0,
                90,
                TAB_HEIGHT,
                hwnd,
                menu_id(id),
                instance,
                null_mut(),
            );
        }

        state.theme_button = create_window_ex_w(
            0,
            button_class.as_ptr(),
            wide("Görünüm").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            0,
            0,
            84,
            TITLE_HEIGHT,
            hwnd,
            menu_id(ID_THEME),
            instance,
            null_mut(),
        );

        if state.title.is_null()
            || state.subtitle.is_null()
            || state.edit.is_null()
            || state.list.is_null()
            || state.status.is_null()
            || state.theme_button.is_null()
            || state.tabs.iter().any(|hwnd| hwnd.is_null())
        {
            return -1;
        }

        for control in [
            state.edit,
            state.list,
            state.status,
            state.tabs[0],
            state.tabs[1],
            state.tabs[2],
            state.tabs[3],
            state.theme_button,
        ] {
            send_message_w(control, WM_SETFONT, state.ui_font as Wparam, 1);
        }
        send_message_w(state.title, WM_SETFONT, state.title_font as Wparam, 1);
        send_message_w(state.subtitle, WM_SETFONT, state.small_font as Wparam, 1);
        send_message_w(state.status, WM_SETFONT, state.small_font as Wparam, 1);

        let cue = wide("Her şeyi ara — dosya, klasör, uygulama veya içerik");
        send_message_w(state.edit, EM_SETCUEBANNER, 1, cue.as_ptr() as Lparam);
        let edit_margin = scale_px(14, state.dpi).clamp(0, u16::MAX as i32) as u32;
        let edit_margins = edit_margin | (edit_margin << 16);
        send_message_w(
            state.edit,
            EM_SETMARGINS,
            EC_LEFTMARGIN | EC_RIGHTMARGIN,
            edit_margins as Lparam,
        );
        update_tab_labels(state);
        0
    }

    unsafe fn apply_control_theme(state: &State) {
        let explorer = wide(if state.dark {
            "DarkMode_Explorer"
        } else {
            "Explorer"
        });
        for hwnd in [
            state.edit,
            state.list,
            state.tabs[0],
            state.tabs[1],
            state.tabs[2],
            state.tabs[3],
            state.theme_button,
        ] {
            let _ = set_window_theme(hwnd, explorer.as_ptr(), null_mut());
        }
    }

    unsafe fn apply_window_composition(hwnd: Hwnd, state_ptr: *mut State) {
        let state = &*state_ptr;

        if supports_modern_frame(state.os_build) {
            let dark: i32 = if state.dark { 1 } else { 0 };
            let _ = dwm_set_window_attribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&dark as *const i32).cast(),
                std::mem::size_of::<i32>() as u32,
            );

            let corners = DWMWCP_ROUND;
            let _ = dwm_set_window_attribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                (&corners as *const i32).cast(),
                std::mem::size_of::<i32>() as u32,
            );

            let border = state.palette.accent.colorref();
            let _ = dwm_set_window_attribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                (&border as *const u32).cast(),
                std::mem::size_of::<u32>() as u32,
            );
        }

        if supports_system_backdrop(state.os_build) {
            let backdrop = match state.theme.backdrop {
                Backdrop::Auto => DWMSBT_AUTO,
                Backdrop::Mica => DWMSBT_MAINWINDOW,
                Backdrop::Acrylic => DWMSBT_TRANSIENTWINDOW,
                Backdrop::None => DWMSBT_NONE,
            };
            let _ = dwm_set_window_attribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                (&backdrop as *const i32).cast(),
                std::mem::size_of::<i32>() as u32,
            );
        }

        let alpha = state.theme.alpha();
        let current_ex_style = get_window_long_ptr_w(hwnd, GWL_EXSTYLE) as u32;
        if alpha < 255 {
            if current_ex_style & WS_EX_LAYERED == 0 {
                set_window_long_ptr_w(
                    hwnd,
                    GWL_EXSTYLE,
                    (current_ex_style | WS_EX_LAYERED) as isize,
                );
                let _ = set_window_pos(
                    hwnd,
                    null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
                );
            }
            let _ = set_layered_window_attributes(hwnd, 0, alpha, LWA_ALPHA);
        } else if current_ex_style & WS_EX_LAYERED != 0 {
            set_window_long_ptr_w(
                hwnd,
                GWL_EXSTYLE,
                (current_ex_style & !WS_EX_LAYERED) as isize,
            );
            let _ = set_window_pos(
                hwnd,
                null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
    }

    unsafe fn center_search_window(hwnd: Hwnd, logical_width: i32, logical_height: i32, dpi: u32) {
        let work = monitor_work_area(hwnd).or_else(|| {
            let mut work = Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            (system_parameters_info_w(SPI_GETWORKAREA, 0, (&mut work as *mut Rect).cast(), 0) != 0)
                .then_some(work)
        });
        let Some(work) = work else {
            return;
        };
        let target = centered_window_rect(work, logical_width, logical_height, dpi);
        let _ = set_window_pos(
            hwnd,
            null_mut(),
            target.left,
            target.top,
            target.right - target.left,
            target.bottom - target.top,
            SWP_NOZORDER,
        );
    }

    unsafe fn apply_dpi(hwnd: Hwnd, state: &mut State, dpi: u32) {
        let dpi = normalize_dpi(dpi);
        if state.dpi == dpi {
            if !state.list.is_null() {
                let _ = send_message_w(
                    state.list,
                    LB_SETITEMHEIGHT,
                    0,
                    scale_px(state.theme.result_row_height(), dpi).max(1) as Lparam,
                );
            }
            return;
        }

        if let Some((ui_font, title_font, small_font)) = create_fonts_for_dpi(dpi, state.os_build) {
            let old_fonts = [state.ui_font, state.title_font, state.small_font];
            state.ui_font = ui_font;
            state.title_font = title_font;
            state.small_font = small_font;

            for control in [
                state.edit,
                state.list,
                state.tabs[0],
                state.tabs[1],
                state.tabs[2],
                state.tabs[3],
                state.theme_button,
            ] {
                if !control.is_null() {
                    send_message_w(control, WM_SETFONT, state.ui_font as Wparam, 1);
                }
            }
            if !state.title.is_null() {
                send_message_w(state.title, WM_SETFONT, state.title_font as Wparam, 1);
            }
            if !state.subtitle.is_null() {
                send_message_w(state.subtitle, WM_SETFONT, state.small_font as Wparam, 1);
            }
            if !state.status.is_null() {
                send_message_w(state.status, WM_SETFONT, state.small_font as Wparam, 1);
            }

            for font in old_fonts {
                if !font.is_null() {
                    let _ = delete_object(font as Hgdiobj);
                }
            }
        }

        state.dpi = dpi;
        if !state.list.is_null() {
            let _ = send_message_w(
                state.list,
                LB_SETITEMHEIGHT,
                0,
                scale_px(state.theme.result_row_height(), dpi).max(1) as Lparam,
            );
        }
        resize_controls(hwnd, state);
        let _ = invalidate_rect(hwnd, null_mut(), 1);
        if !state.list.is_null() {
            let _ = invalidate_rect(state.list, null_mut(), 1);
        }
    }

    unsafe fn recover_window_to_monitor(hwnd: Hwnd, state: &mut State) {
        let Some(work) = monitor_work_area(hwnd) else {
            return;
        };
        let mut current = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if get_window_rect(hwnd, &mut current) == 0 {
            return;
        }
        let target = clamp_window_rect(current, work);
        if target != current {
            let _ = set_window_pos(
                hwnd,
                null_mut(),
                target.left,
                target.top,
                target.right - target.left,
                target.bottom - target.top,
                SWP_NOZORDER,
            );
        }
        let dpi = effective_window_dpi(hwnd);
        if dpi != state.dpi {
            apply_dpi(hwnd, state, dpi);
        } else {
            resize_controls(hwnd, state);
        }
    }

    unsafe fn resize_controls(hwnd: Hwnd, state: &mut State) {
        if state.edit.is_null() || state.list.is_null() {
            return;
        }
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if get_client_rect(hwnd, &mut rect) == 0 {
            return;
        }
        let margin = scale_px(MARGIN, state.dpi);
        let title_height = scale_px(TITLE_HEIGHT, state.dpi);
        let subtitle_height = scale_px(18, state.dpi);
        let search_height = scale_px(52, state.dpi);
        let tab_height = scale_px(TAB_HEIGHT, state.dpi);
        let status_height = scale_px(STATUS_HEIGHT, state.dpi);
        let width = (rect.right - rect.left - margin * 2).max(1);
        let title_y = scale_px(12, state.dpi);
        let subtitle_y = title_y + title_height;
        let search_y = subtitle_y + subtitle_height + scale_px(12, state.dpi);
        let tabs_y = search_y + search_height + scale_px(12, state.dpi);
        let status_y = tabs_y + tab_height + scale_px(10, state.dpi);
        let list_y = status_y + status_height + scale_px(6, state.dpi);
        let list_height = (rect.bottom - list_y - margin).max(1);
        let theme_width = scale_px(108, state.dpi);

        move_window(
            state.title,
            margin,
            title_y,
            (width - theme_width - scale_px(12, state.dpi)).max(1),
            title_height,
            1,
        );
        move_window(
            state.subtitle,
            margin,
            subtitle_y,
            (width - theme_width - scale_px(12, state.dpi)).max(1),
            subtitle_height,
            1,
        );
        move_window(
            state.theme_button,
            margin + (width - theme_width).max(0),
            title_y + scale_px(4, state.dpi),
            theme_width,
            scale_px(34, state.dpi),
            1,
        );
        move_window(state.edit, margin, search_y, width, search_height, 1);

        let tab_gap = scale_px(8, state.dpi);
        let tab_width = scale_px(94, state.dpi);
        for (index, tab) in state.tabs.iter().enumerate() {
            move_window(
                *tab,
                margin + index as i32 * (tab_width + tab_gap),
                tabs_y,
                tab_width,
                tab_height,
                1,
            );
        }
        move_window(state.status, margin, status_y, width, status_height, 1);
        move_window(state.list, margin, list_y, width, list_height, 1);
    }

    unsafe fn update_tab_labels(state: &State) {
        for (index, (mode, label)) in [
            (SearchMode::All, "Tümü"),
            (SearchMode::Files, "Dosyalar"),
            (SearchMode::Folders, "Klasörler"),
            (SearchMode::Content, "İçerik"),
        ]
        .into_iter()
        .enumerate()
        {
            let text = if state.mode == mode {
                format!("• {label}")
            } else {
                label.to_string()
            };
            let text = wide(&text);
            set_window_text_w(state.tabs[index], text.as_ptr());
        }
    }

    unsafe fn refresh_results(state: &mut State) {
        send_message_w(state.list, LB_RESETCONTENT, 0, 0);
        state.results.clear();

        let len = get_window_text_length_w(state.edit).clamp(0, MAX_QUERY_U16);
        if len == 0 {
            set_idle_status(state);
            return;
        }
        let mut buffer = vec![0_u16; len as usize + 1];
        let copied = get_window_text_w(state.edit, buffer.as_mut_ptr(), len + 1);
        if copied <= 0 {
            return;
        }
        let query = String::from_utf16_lossy(&buffer[..copied as usize]);
        let query = query.trim();
        if query.is_empty() {
            set_idle_status(state);
            return;
        }

        let explicit_path_filter = match state.mode {
            SearchMode::Content => None,
            _ => parse_search_query(query).filters.path_contains,
        };

        let started = Instant::now();
        let hits = search_for_mode(state, query);
        let elapsed = started.elapsed();
        let Ok(hits) = hits else {
            set_status(state, "Arama geçici olarak kullanılamıyor");
            return;
        };

        for hit in hits {
            let path = state
                .store
                .reconstruct_path(&hit, 256)
                .unwrap_or_else(|_| format!("{}:\\{}", hit.volume, hit.hit.name));
            if state
                .scope
                .as_deref()
                .is_some_and(|scope| !path_is_within_scope(&path, scope))
            {
                continue;
            }
            if !path_matches_explicit_filter(&path, explicit_path_filter.as_deref()) {
                continue;
            }
            let row = ResultRow {
                name: hit.hit.name.clone(),
                path,
                is_directory: hit.hit.flags & FLAG_DIRECTORY != 0,
            };
            let label = wide(&row.name);
            let index = state.results.len();
            let added = send_message_w(state.list, LB_ADDSTRING, 0, label.as_ptr() as Lparam);
            if added >= 0 {
                send_message_w(state.list, LB_SETITEMDATA, added as Wparam, index as Lparam);
                state.results.push(row);
                if state.results.len() >= MAX_RESULTS {
                    break;
                }
            }
        }

        let count = state.results.len();
        let timing = elapsed.as_secs_f64() * 1000.0;
        let status = match state.scope.as_deref() {
            Some(scope) => format!("{count} sonuç  •  {timing:.1} ms  •  {scope}"),
            None => format!("{count} sonuç  •  {timing:.1} ms"),
        };
        set_status(state, &status);
        invalidate_rect(state.list, null_mut(), 0);
    }

    fn search_for_mode(
        state: &mut State,
        query: &str,
    ) -> io::Result<Vec<search_core::VolumeSearchHit>> {
        let scoped_limit = if state.scope.is_some() {
            MAX_RESULTS.saturating_mul(4)
        } else {
            MAX_RESULTS
        };
        match state.mode {
            SearchMode::Content => {
                let terms = content_terms(query);
                state
                    .store
                    .search_content(&terms, scoped_limit.saturating_mul(2))
            }
            SearchMode::Files | SearchMode::Folders => {
                let mut parsed = parse_search_query(query);
                apply_scope_filter(&mut parsed, state.scope.as_deref());
                parsed.filters.item_type = Some(match state.mode {
                    SearchMode::Files => ItemTypeFilter::File,
                    SearchMode::Folders => ItemTypeFilter::Directory,
                    _ => unreachable!(),
                });
                state.store.search_filtered(&parsed, scoped_limit, 100_000)
            }
            SearchMode::All => {
                let mut parsed = parse_search_query(query);
                let has_explicit_filters = !parsed.filters.is_empty();
                if has_explicit_filters {
                    apply_scope_filter(&mut parsed, state.scope.as_deref());
                    state.store.search_filtered(&parsed, scoped_limit, 100_000)
                } else if relation_for_query(&parsed.text).is_some() {
                    state.store.search_related(&parsed.text, scoped_limit)
                } else if should_route_natural(query) {
                    route_natural_query(state, query, scoped_limit)
                } else if state.scope.is_some() {
                    apply_scope_filter(&mut parsed, state.scope.as_deref());
                    state.store.search_filtered(&parsed, scoped_limit, 100_000)
                } else {
                    state.store.search_ranked(&parsed.text, scoped_limit)
                }
            }
        }
    }

    unsafe fn set_status(state: &State, value: &str) {
        let value = wide(value);
        set_window_text_w(state.status, value.as_ptr());
    }

    unsafe fn set_idle_status(state: &State) {
        match state.scope.as_deref() {
            Some(scope) => set_status(state, &format!("Bu konumda ara  •  {scope}")),
            None => set_status(state, "Hazır  •  Yerel index  •  Bulut yok  •  Alt+Space"),
        }
    }

    unsafe fn draw_filter_chip(state: &State, draw: &DrawItemStruct) {
        let (mode, label) = match draw.ctl_id as usize {
            ID_ALL => (SearchMode::All, "Tümü"),
            ID_FILES => (SearchMode::Files, "Dosyalar"),
            ID_FOLDERS => (SearchMode::Folders, "Klasörler"),
            ID_CONTENT => (SearchMode::Content, "İçerik"),
            _ => return,
        };

        let active = state.mode == mode;
        fill_rect(draw.hdc, &draw.rc_item, state.background_brush);

        let inset = scale_px(2, state.dpi);
        let mut chip = Rect {
            left: draw.rc_item.left + inset,
            top: draw.rc_item.top + inset,
            right: draw.rc_item.right - inset,
            bottom: draw.rc_item.bottom - inset,
        };
        if chip.right <= chip.left {
            chip.right = chip.left + 1;
        }
        if chip.bottom <= chip.top {
            chip.bottom = chip.top + 1;
        }

        fill_rect(
            draw.hdc,
            &chip,
            if active {
                state.accent_brush
            } else {
                state.surface_brush
            },
        );
        set_bk_mode(draw.hdc, TRANSPARENT);
        set_text_color(
            draw.hdc,
            if active {
                state.palette.selected_text.colorref()
            } else {
                state.palette.text.colorref()
            },
        );

        let old_font = select_object(draw.hdc, state.small_font as Hgdiobj);
        let text = wide(label);
        let mut text_rect = chip;
        draw_text_w(
            draw.hdc,
            text.as_ptr(),
            -1,
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        if !old_font.is_null() {
            select_object(draw.hdc, old_font);
        }
    }

    unsafe fn draw_result_row(state: &State, draw: &DrawItemStruct) {
        if draw.item_id == u32::MAX {
            return;
        }
        let index = draw.item_data;
        let Some(row) = state.results.get(index) else {
            return;
        };

        let selected = draw.item_state & ODS_SELECTED != 0;
        fill_rect(draw.hdc, &draw.rc_item, state.background_brush);

        let inset_x = scale_px(4, state.dpi);
        let inset_y = scale_px(3, state.dpi);
        let mut card = Rect {
            left: draw.rc_item.left + inset_x,
            top: draw.rc_item.top + inset_y,
            right: draw.rc_item.right - inset_x,
            bottom: draw.rc_item.bottom - inset_y,
        };
        if card.right <= card.left {
            card.right = card.left + 1;
        }
        if card.bottom <= card.top {
            card.bottom = card.top + 1;
        }
        fill_rect(
            draw.hdc,
            &card,
            if selected {
                state.accent_brush
            } else {
                state.surface_brush
            },
        );

        if !selected {
            let accent_width = scale_px(4, state.dpi);
            let accent = Rect {
                left: card.left,
                top: card.top,
                right: (card.left + accent_width).min(card.right),
                bottom: card.bottom,
            };
            fill_rect(draw.hdc, &accent, state.accent_brush);
        }
        set_bk_mode(draw.hdc, TRANSPARENT);

        let old_font = select_object(draw.hdc, state.ui_font as Hgdiobj);
        let title_color = if selected {
            state.palette.selected_text
        } else {
            state.palette.text
        };
        set_text_color(draw.hdc, title_color.colorref());

        let icon = if row.is_directory { "▣" } else { "◆" };
        let title = wide(&format!("{icon}  {}", row.name));
        let badge_width = scale_px(86, state.dpi);
        let mut title_rect = Rect {
            left: card.left + scale_px(14, state.dpi),
            top: card.top + scale_px(6, state.dpi),
            right: (card.right - badge_width).max(card.left + scale_px(40, state.dpi)),
            bottom: card.top + scale_px(32, state.dpi),
        };
        draw_text_w(
            draw.hdc,
            title.as_ptr(),
            -1,
            &mut title_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );

        select_object(draw.hdc, state.small_font as Hgdiobj);
        let path_color = if selected {
            state.palette.selected_text
        } else {
            state.palette.muted
        };
        set_text_color(draw.hdc, path_color.colorref());
        let badge = wide(if row.is_directory { "KLASÖR" } else { "DOSYA" });
        let mut badge_rect = Rect {
            left: (card.right - badge_width).max(card.left),
            top: card.top + scale_px(6, state.dpi),
            right: card.right - scale_px(12, state.dpi),
            bottom: card.top + scale_px(32, state.dpi),
        };
        draw_text_w(
            draw.hdc,
            badge.as_ptr(),
            -1,
            &mut badge_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );

        let path = wide(&row.path);
        let mut path_rect = Rect {
            left: card.left + scale_px(36, state.dpi),
            top: card.top + scale_px(31, state.dpi),
            right: card.right - scale_px(12, state.dpi),
            bottom: card.bottom - scale_px(5, state.dpi),
        };
        draw_text_w(
            draw.hdc,
            path.as_ptr(),
            -1,
            &mut path_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );

        if !old_font.is_null() {
            select_object(draw.hdc, old_font);
        }
    }

    unsafe fn open_selected(hwnd: Hwnd, state: &mut State) -> bool {
        let selected = send_message_w(state.list, LB_GETCURSEL, 0, 0);
        if selected < 0 {
            return false;
        }
        let item_data = send_message_w(state.list, LB_GETITEMDATA, selected as Wparam, 0);
        let index = if item_data >= 0 {
            item_data as usize
        } else {
            selected as usize
        };
        let Some(row) = state.results.get(index) else {
            return false;
        };
        let operation = wide("open");
        let path = wide(&row.path);
        let result = shell_execute_w(
            hwnd,
            operation.as_ptr(),
            path.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        ) as isize;
        if result > 32 {
            if state.resident {
                show_window(hwnd, SW_HIDE);
                state.intent_model = None;
            }
            true
        } else {
            false
        }
    }

    unsafe fn set_query(state: &mut State, query: &str) {
        let query = wide(query);
        set_window_text_w(state.edit, query.as_ptr());
        refresh_results(state);
    }

    unsafe fn apply_search_request(state: &mut State, request: SearchRequest) {
        state.scope = request.scope.and_then(normalize_scope);
        set_query(state, request.query.as_deref().unwrap_or(""));
    }

    unsafe fn append_menu_item(menu: Hmenu, id: usize, label: &str, checked: bool) {
        let label = wide(label);
        let flags = MF_STRING | if checked { MF_CHECKED } else { 0 };
        let _ = append_menu_w(menu, flags, id, label.as_ptr());
    }

    unsafe fn append_menu_separator(menu: Hmenu) {
        let _ = append_menu_w(menu, MF_SEPARATOR, 0, null_mut());
    }

    unsafe fn show_theme_menu(hwnd: Hwnd, state: &mut State) {
        let menu = create_popup_menu();
        if menu.is_null() {
            set_status(state, "Görünüm menüsü açılamadı");
            return;
        }

        append_menu_item(
            menu,
            CMD_PRESET_SIGNATURE,
            "Preset: Search Tool Signature",
            state.theme.preset == ThemePreset::Signature,
        );
        append_menu_item(
            menu,
            CMD_PRESET_MIDNIGHT,
            "Preset: Midnight",
            state.theme.preset == ThemePreset::Midnight,
        );
        append_menu_item(
            menu,
            CMD_PRESET_GRAPHITE,
            "Preset: Graphite",
            state.theme.preset == ThemePreset::Graphite,
        );
        append_menu_item(
            menu,
            CMD_PRESET_FROST,
            "Preset: Frost",
            state.theme.preset == ThemePreset::Frost,
        );
        append_menu_item(
            menu,
            CMD_PRESET_NATIVE,
            "Preset: Windows Native",
            state.theme.preset == ThemePreset::Native,
        );
        append_menu_separator(menu);

        append_menu_item(
            menu,
            CMD_THEME_SYSTEM,
            "Tema: Sistem",
            state.theme.mode == ThemeMode::System,
        );
        append_menu_item(
            menu,
            CMD_THEME_DARK,
            "Tema: Koyu",
            state.theme.mode == ThemeMode::Dark,
        );
        append_menu_item(
            menu,
            CMD_THEME_LIGHT,
            "Tema: Açık",
            state.theme.mode == ThemeMode::Light,
        );
        append_menu_separator(menu);

        append_menu_item(
            menu,
            CMD_BACKDROP_AUTO,
            "Efekt: Otomatik",
            state.theme.backdrop == Backdrop::Auto,
        );
        append_menu_item(
            menu,
            CMD_BACKDROP_ACRYLIC,
            "Efekt: Acrylic / Win10 fallback",
            state.theme.backdrop == Backdrop::Acrylic,
        );
        append_menu_item(
            menu,
            CMD_BACKDROP_MICA,
            "Efekt: Mica (Windows 11)",
            state.theme.backdrop == Backdrop::Mica,
        );
        append_menu_item(
            menu,
            CMD_BACKDROP_NONE,
            "Efekt: Düz renk",
            state.theme.backdrop == Backdrop::None,
        );
        append_menu_separator(menu);

        for (id, opacity) in [
            (CMD_OPACITY_60, 60_u8),
            (CMD_OPACITY_75, 75),
            (CMD_OPACITY_90, 90),
            (CMD_OPACITY_100, 100),
        ] {
            append_menu_item(
                menu,
                id,
                &format!("Pencere saydamlığı: %{opacity}"),
                state.theme.opacity_percent == opacity,
            );
        }
        append_menu_separator(menu);

        append_menu_item(menu, CMD_ACCENT, "Renk: Vurgu...", false);
        append_menu_item(menu, CMD_BACKGROUND_COLOR, "Renk: Arka plan...", false);
        append_menu_item(menu, CMD_SURFACE_COLOR, "Renk: Kart / yüzey...", false);
        append_menu_item(menu, CMD_TEXT_COLOR, "Renk: Ana yazı...", false);
        append_menu_item(menu, CMD_MUTED_COLOR, "Renk: İkincil yazı...", false);
        append_menu_item(menu, CMD_RESET_PALETTE, "Renkleri preset'e döndür", false);
        append_menu_separator(menu);

        append_menu_item(
            menu,
            CMD_DENSITY_COMPACT,
            "Sonuç yoğunluğu: Compact",
            state.theme.density == Density::Compact,
        );
        append_menu_item(
            menu,
            CMD_DENSITY_COMFORTABLE,
            "Sonuç yoğunluğu: Comfortable",
            state.theme.density == Density::Comfortable,
        );
        append_menu_item(
            menu,
            CMD_DENSITY_SPACIOUS,
            "Sonuç yoğunluğu: Spacious",
            state.theme.density == Density::Spacious,
        );
        append_menu_separator(menu);

        append_menu_item(
            menu,
            CMD_SIZE_COMPACT,
            "Panel: Compact 760×540",
            state.theme.width == 760 && state.theme.height == 540,
        );
        append_menu_item(
            menu,
            CMD_SIZE_STANDARD,
            "Panel: Standard 900×640",
            state.theme.width == 900 && state.theme.height == 640,
        );
        append_menu_item(
            menu,
            CMD_SIZE_WIDE,
            "Panel: Wide 1120×720",
            state.theme.width == 1120 && state.theme.height == 720,
        );
        append_menu_separator(menu);

        append_menu_item(menu, CMD_BACKGROUND_IMAGE, "Arka plan resmi seç...", false);
        append_menu_item(
            menu,
            CMD_BACKGROUND_IMAGE_CLEAR,
            "Arka plan resmini kaldır",
            state.theme.background_image.is_none(),
        );
        append_menu_item(
            menu,
            CMD_BACKGROUND_FIT_FILL,
            "Resim yerleşimi: Fill",
            state.theme.background_fit == BackgroundFit::Fill,
        );
        append_menu_item(
            menu,
            CMD_BACKGROUND_FIT_FIT,
            "Resim yerleşimi: Fit",
            state.theme.background_fit == BackgroundFit::Fit,
        );
        append_menu_item(
            menu,
            CMD_BACKGROUND_FIT_STRETCH,
            "Resim yerleşimi: Stretch",
            state.theme.background_fit == BackgroundFit::Stretch,
        );
        for (id, opacity) in [
            (CMD_BACKGROUND_IMAGE_OPACITY_20, 20_u8),
            (CMD_BACKGROUND_IMAGE_OPACITY_35, 35),
            (CMD_BACKGROUND_IMAGE_OPACITY_60, 60),
            (CMD_BACKGROUND_IMAGE_OPACITY_100, 100),
        ] {
            append_menu_item(
                menu,
                id,
                &format!("Resim opaklığı: %{opacity}"),
                state.theme.background_image_opacity == opacity,
            );
        }
        append_menu_separator(menu);

        append_menu_item(
            menu,
            CMD_DEFAULT_APPS,
            "Windows varsayılan arama ayarları...",
            false,
        );
        append_menu_item(menu, CMD_ADVANCED_THEME, "Tema dosyasını aç...", false);

        let mut point = Point { x: 0, y: 0 };
        if get_cursor_pos(&mut point) == 0 {
            let _ = destroy_menu(menu);
            return;
        }
        let command = track_popup_menu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            null_mut(),
        ) as usize;
        let _ = destroy_menu(menu);

        let mut resize_window = false;
        let changed = match command {
            CMD_PRESET_SIGNATURE => {
                state.theme.apply_preset(ThemePreset::Signature);
                true
            }
            CMD_PRESET_MIDNIGHT => {
                state.theme.apply_preset(ThemePreset::Midnight);
                true
            }
            CMD_PRESET_GRAPHITE => {
                state.theme.apply_preset(ThemePreset::Graphite);
                true
            }
            CMD_PRESET_FROST => {
                state.theme.apply_preset(ThemePreset::Frost);
                true
            }
            CMD_PRESET_NATIVE => {
                state.theme.apply_preset(ThemePreset::Native);
                true
            }
            CMD_THEME_SYSTEM => {
                state.theme.mode = ThemeMode::System;
                true
            }
            CMD_THEME_DARK => {
                state.theme.mode = ThemeMode::Dark;
                true
            }
            CMD_THEME_LIGHT => {
                state.theme.mode = ThemeMode::Light;
                true
            }
            CMD_BACKDROP_AUTO => {
                state.theme.backdrop = Backdrop::Auto;
                true
            }
            CMD_BACKDROP_ACRYLIC => {
                state.theme.backdrop = Backdrop::Acrylic;
                true
            }
            CMD_BACKDROP_MICA => {
                state.theme.backdrop = Backdrop::Mica;
                true
            }
            CMD_BACKDROP_NONE => {
                state.theme.backdrop = Backdrop::None;
                true
            }
            CMD_OPACITY_60 => {
                state.theme.opacity_percent = 60;
                true
            }
            CMD_OPACITY_75 => {
                state.theme.opacity_percent = 75;
                true
            }
            CMD_OPACITY_90 => {
                state.theme.opacity_percent = 90;
                true
            }
            CMD_OPACITY_100 => {
                state.theme.opacity_percent = 100;
                true
            }
            CMD_ACCENT => choose_color(hwnd, state.theme.accent)
                .map(|value| state.theme.accent = value)
                .is_some(),
            CMD_BACKGROUND_COLOR => choose_color(
                hwnd,
                state.theme.background.unwrap_or(state.palette.background),
            )
            .map(|value| state.theme.background = Some(value))
            .is_some(),
            CMD_SURFACE_COLOR => {
                choose_color(hwnd, state.theme.surface.unwrap_or(state.palette.surface))
                    .map(|value| state.theme.surface = Some(value))
                    .is_some()
            }
            CMD_TEXT_COLOR => choose_color(hwnd, state.theme.text.unwrap_or(state.palette.text))
                .map(|value| state.theme.text = Some(value))
                .is_some(),
            CMD_MUTED_COLOR => choose_color(hwnd, state.theme.muted.unwrap_or(state.palette.muted))
                .map(|value| state.theme.muted = Some(value))
                .is_some(),
            CMD_RESET_PALETTE => {
                let preset = state.theme.preset;
                state.theme.apply_preset(preset);
                true
            }
            CMD_DENSITY_COMPACT => {
                state.theme.density = Density::Compact;
                true
            }
            CMD_DENSITY_COMFORTABLE => {
                state.theme.density = Density::Comfortable;
                true
            }
            CMD_DENSITY_SPACIOUS => {
                state.theme.density = Density::Spacious;
                true
            }
            CMD_SIZE_COMPACT => {
                state.theme.width = 760;
                state.theme.height = 540;
                resize_window = true;
                true
            }
            CMD_SIZE_STANDARD => {
                state.theme.width = 900;
                state.theme.height = 640;
                resize_window = true;
                true
            }
            CMD_SIZE_WIDE => {
                state.theme.width = 1120;
                state.theme.height = 720;
                resize_window = true;
                true
            }
            CMD_BACKGROUND_IMAGE => choose_background_image(hwnd, state),
            CMD_BACKGROUND_IMAGE_CLEAR => {
                state.theme.background_image = None;
                true
            }
            CMD_BACKGROUND_FIT_FILL => {
                state.theme.background_fit = BackgroundFit::Fill;
                true
            }
            CMD_BACKGROUND_FIT_FIT => {
                state.theme.background_fit = BackgroundFit::Fit;
                true
            }
            CMD_BACKGROUND_FIT_STRETCH => {
                state.theme.background_fit = BackgroundFit::Stretch;
                true
            }
            CMD_BACKGROUND_IMAGE_OPACITY_20 => {
                state.theme.background_image_opacity = 20;
                true
            }
            CMD_BACKGROUND_IMAGE_OPACITY_35 => {
                state.theme.background_image_opacity = 35;
                true
            }
            CMD_BACKGROUND_IMAGE_OPACITY_60 => {
                state.theme.background_image_opacity = 60;
                true
            }
            CMD_BACKGROUND_IMAGE_OPACITY_100 => {
                state.theme.background_image_opacity = 100;
                true
            }
            CMD_DEFAULT_APPS => {
                open_default_apps(hwnd, state);
                false
            }
            CMD_ADVANCED_THEME => {
                open_theme_config(hwnd, state);
                false
            }
            _ => false,
        };

        if changed {
            if let Err(error) = state.theme.save() {
                set_status(state, &format!("Görünüm kaydedilemedi: {error}"));
                return;
            }
            apply_runtime_theme(hwnd, state);
            if resize_window {
                center_search_window(hwnd, state.theme.width, state.theme.height, state.dpi);
            }
            set_status(state, "Görünüm anında uygulandı ve kaydedildi");
        }
    }

    unsafe fn choose_color(hwnd: Hwnd, initial: Rgb) -> Option<Rgb> {
        let mut custom = [0_u32; 16];
        let mut chooser = ChooseColorW {
            struct_size: std::mem::size_of::<ChooseColorW>() as u32,
            owner: hwnd,
            instance: null_mut(),
            rgb_result: initial.colorref(),
            custom_colors: custom.as_mut_ptr(),
            flags: CC_RGBINIT | CC_FULLOPEN,
            custom_data: 0,
            hook: null_mut(),
            template_name: null_mut(),
        };
        if choose_color_w(&mut chooser) == 0 {
            return None;
        }
        let value = chooser.rgb_result;
        Some(Rgb::new(
            (value & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            ((value >> 16) & 0xff) as u8,
        ))
    }

    unsafe fn choose_background_image(hwnd: Hwnd, state: &mut State) -> bool {
        let filter: Vec<u16> = "Resimler\0*.png;*.jpg;*.jpeg;*.bmp\0Tüm dosyalar\0*.*\0\0"
            .encode_utf16()
            .collect();
        let title = wide("Search Tool arka plan resmi seç");
        let mut file = vec![0_u16; 32_768];
        let mut dialog = OpenFileNameW {
            struct_size: std::mem::size_of::<OpenFileNameW>() as u32,
            owner: hwnd,
            instance: null_mut(),
            filter: filter.as_ptr(),
            custom_filter: null_mut(),
            max_custom_filter: 0,
            filter_index: 1,
            file: file.as_mut_ptr(),
            max_file: file.len() as u32,
            file_title: null_mut(),
            max_file_title: 0,
            initial_dir: null_mut(),
            title: title.as_ptr(),
            flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_EXPLORER,
            file_offset: 0,
            file_extension: 0,
            default_extension: null_mut(),
            custom_data: 0,
            hook: null_mut(),
            template_name: null_mut(),
            reserved: null_mut(),
            reserved_dword: 0,
            flags_ex: 0,
        };
        if get_open_file_name_w(&mut dialog) == 0 {
            return false;
        }
        let end = file
            .iter()
            .position(|&value| value == 0)
            .unwrap_or(file.len());
        if end == 0 {
            return false;
        }
        state.theme.background_image = Some(PathBuf::from(String::from_utf16_lossy(&file[..end])));
        true
    }

    unsafe fn apply_runtime_theme(hwnd: Hwnd, state: &mut State) {
        state.dark = match state.theme.mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => system_prefers_dark(),
        };
        state.palette = state.theme.palette(state.dark);

        for brush in [
            state.background_brush as Hgdiobj,
            state.surface_brush as Hgdiobj,
            state.accent_brush as Hgdiobj,
        ] {
            if !brush.is_null() {
                let _ = delete_object(brush);
            }
        }
        state.background_brush = create_solid_brush(state.palette.background.colorref());
        state.surface_brush = create_solid_brush(state.palette.surface.colorref());
        state.accent_brush = create_solid_brush(state.palette.accent.colorref());
        reload_background_image(state);

        if !state.list.is_null() {
            let _ = send_message_w(
                state.list,
                LB_SETITEMHEIGHT,
                0,
                scale_px(state.theme.result_row_height(), state.dpi).max(1) as Lparam,
            );
        }

        apply_control_theme(state);
        apply_window_composition(hwnd, state as *mut State);
        resize_controls(hwnd, state);
        for control in [
            hwnd,
            state.edit,
            state.list,
            state.title,
            state.subtitle,
            state.status,
            state.tabs[0],
            state.tabs[1],
            state.tabs[2],
            state.tabs[3],
            state.theme_button,
        ] {
            if !control.is_null() {
                let _ = invalidate_rect(control, null_mut(), 1);
            }
        }
        update_window(hwnd);
    }

    unsafe fn open_default_apps(hwnd: Hwnd, state: &State) {
        let operation = wide("open");
        let uri = wide("ms-settings:defaultapps?registeredAppMachine=Search%20Tool");
        let result = shell_execute_w(
            hwnd,
            operation.as_ptr(),
            uri.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        ) as isize;
        if result > 32 {
            set_status(state, "Windows varsayılan uygulamalar sayfası açıldı");
        } else {
            set_status(state, "Windows varsayılan uygulamalar sayfası açılamadı");
        }
    }

    unsafe fn open_theme_config(hwnd: Hwnd, state: &State) {
        theme::ensure_default_config();
        let operation = wide("open");
        let path = wide(&theme::config_path().to_string_lossy());
        let result = shell_execute_w(
            hwnd,
            operation.as_ptr(),
            path.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        ) as isize;
        if result > 32 {
            set_status(state, "Gelişmiş tema dosyası açıldı");
        } else {
            set_status(state, "Tema ayar dosyası açılamadı");
        }
    }

    fn should_route_natural(query: &str) -> bool {
        let normalized = query.to_lowercase();
        query.split_whitespace().count() >= 3
            || [
                "içinde",
                "icinde",
                "geçen",
                "gecen",
                "contains",
                "containing",
                "alakalı",
                "alakali",
                "related",
                "benzer",
                "fuzzy",
            ]
            .iter()
            .any(|marker| normalized.contains(marker))
    }

    fn search_ranked_with_scope(
        state: &mut State,
        query: &str,
        limit: usize,
    ) -> io::Result<Vec<search_core::VolumeSearchHit>> {
        if state.scope.is_none() {
            return state.store.search_ranked(query, limit);
        }
        let mut parsed = parse_search_query(query);
        apply_scope_filter(&mut parsed, state.scope.as_deref());
        state.store.search_filtered(&parsed, limit, 100_000)
    }

    fn route_natural_query(
        state: &mut State,
        query: &str,
        limit: usize,
    ) -> io::Result<Vec<search_core::VolumeSearchHit>> {
        if state.intent_model.is_none() {
            match TinyIntentModel::load(&state.model_path) {
                Ok(model) => state.intent_model = Some(model),
                Err(_) => {
                    let subject = query_subject(query);
                    return search_ranked_with_scope(state, &subject, limit);
                }
            }
        }
        let subject = query_subject(query);
        let Some(model) = state.intent_model.as_ref() else {
            return search_ranked_with_scope(state, &subject, limit);
        };
        let prediction = model.classify(query);
        match prediction.intent {
            QueryIntent::ContentSearch => {
                let terms = content_terms(query);
                state.store.search_content(&terms, limit)
            }
            QueryIntent::RelatedSearch => state.store.search_related(&subject, limit),
            QueryIntent::FuzzySearch => state.store.search_fuzzy(&subject, 2, limit),
            QueryIntent::ExactSearch | QueryIntent::Unknown => {
                search_ranked_with_scope(state, &subject, limit)
            }
            QueryIntent::CleanupAnalysis | QueryIntent::WebLookup | QueryIntent::Help => {
                search_ranked_with_scope(state, &subject, limit)
            }
        }
    }

    unsafe fn send_request_to_existing(hwnd: Hwnd, request: &SearchRequest) {
        let mut payload = Vec::<u16>::new();
        payload.extend(request.query.as_deref().unwrap_or("").encode_utf16());
        payload.push(0);
        payload.extend(request.scope.as_deref().unwrap_or("").encode_utf16());
        payload.push(0);
        let copy = CopyDataStruct {
            dw_data: 2,
            cb_data: (payload.len() * 2) as u32,
            lp_data: payload.as_ptr().cast(),
        };
        send_message_w(
            hwnd,
            WM_COPYDATA,
            0,
            (&copy as *const CopyDataStruct) as Lparam,
        );
    }

    fn decode_ipc_request(words: &[u16]) -> Option<SearchRequest> {
        let first_end = words.iter().position(|&value| value == 0)?;
        let rest = words.get(first_end + 1..)?;
        let second_end = rest
            .iter()
            .position(|&value| value == 0)
            .unwrap_or(rest.len());
        let query = String::from_utf16_lossy(&words[..first_end]);
        let scope = String::from_utf16_lossy(&rest[..second_end]);
        Some(SearchRequest {
            query: (!query.trim().is_empty()).then(|| query.trim().to_string()),
            scope: normalize_scope(scope),
        })
    }

    fn is_search_uri(value: &str) -> bool {
        value.split_once(':').is_some_and(|(scheme, _)| {
            scheme.eq_ignore_ascii_case("search") || scheme.eq_ignore_ascii_case("searchtool")
        })
    }

    fn parse_search_uri(uri: &str) -> Option<SearchRequest> {
        let (scheme, rest) = uri.split_once(':')?;
        if !scheme.eq_ignore_ascii_case("search") && !scheme.eq_ignore_ascii_case("searchtool") {
            return None;
        }
        let rest = rest.trim_start_matches('?');
        let mut request = SearchRequest::default();
        for pair in rest.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if key.eq_ignore_ascii_case("query") || key.eq_ignore_ascii_case("q") {
                let decoded = percent_decode(value);
                if !decoded.trim().is_empty() {
                    request.query = Some(decoded.trim().to_string());
                }
                continue;
            }
            if key.eq_ignore_ascii_case("scope") || key.eq_ignore_ascii_case("location") {
                request.scope = normalize_scope(percent_decode(value));
                continue;
            }
            if key.eq_ignore_ascii_case("crumb") {
                let decoded = percent_decode(value);
                if let Some((kind, location)) = decoded.split_once(':') {
                    if kind.eq_ignore_ascii_case("location") {
                        request.scope = normalize_scope(location);
                    }
                }
            }
        }
        if !rest.contains('=') {
            let decoded = percent_decode(rest);
            if !decoded.trim().is_empty() {
                request.query = Some(decoded.trim().to_string());
            }
        }
        (!request.is_empty()).then_some(request)
    }

    fn normalize_scope(value: impl AsRef<str>) -> Option<String> {
        let value = value.as_ref().trim().trim_matches('"');
        if value.is_empty() {
            return None;
        }
        let mut normalized = value.replace('/', "\\");
        while normalized.ends_with('\\')
            && !(normalized.len() == 3 && normalized.as_bytes().get(1) == Some(&b':'))
        {
            normalized.pop();
        }
        (!normalized.is_empty()).then_some(normalized)
    }

    fn normalized_scope_key(value: &str) -> String {
        value
            .replace('/', "\\")
            .chars()
            .flat_map(char::to_lowercase)
            .collect()
    }

    fn path_is_within_scope(path: &str, scope: &str) -> bool {
        let path = normalized_scope_key(path);
        let scope = normalized_scope_key(scope);
        if path == scope {
            return true;
        }
        let mut prefix = scope;
        if !prefix.ends_with('\\') {
            prefix.push('\\');
        }
        path.starts_with(&prefix)
    }

    fn scope_filter_needle(scope: &str) -> String {
        let mut needle = normalized_scope_key(scope);
        if !needle.ends_with('\\') {
            needle.push('\\');
        }
        needle
    }

    fn path_matches_explicit_filter(path: &str, needle: Option<&str>) -> bool {
        needle.is_none_or(|needle| search_core::store::normalize_name(path).contains(needle))
    }

    fn apply_scope_filter(parsed: &mut search_core::ParsedSearchQuery, scope: Option<&str>) {
        if let Some(scope) = scope {
            parsed.filters.path_contains = Some(scope_filter_needle(scope));
        }
    }

    fn percent_decode(value: &str) -> String {
        let bytes = value.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'%' if index + 2 < bytes.len() => {
                    let hi = hex(bytes[index + 1]);
                    let lo = hex(bytes[index + 2]);
                    if let (Some(hi), Some(lo)) = (hi, lo) {
                        out.push((hi << 4) | lo);
                        index += 3;
                        continue;
                    }
                    out.push(bytes[index]);
                }
                b'+' => out.push(b' '),
                value => out.push(value),
            }
            index += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn hex(value: u8) -> Option<u8> {
        match value {
            b'0'..=b'9' => Some(value - b'0'),
            b'a'..=b'f' => Some(value - b'a' + 10),
            b'A'..=b'F' => Some(value - b'A' + 10),
            _ => None,
        }
    }

    fn system_prefers_dark() -> bool {
        unsafe {
            let hkey = std::ptr::with_exposed_provenance_mut::<c_void>(0x8000_0001usize);
            let sub_key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
            let value_name = wide("AppsUseLightTheme");
            let mut value = 1_u32;
            let mut size = std::mem::size_of::<u32>() as u32;
            let result = reg_get_value_w(
                hkey,
                sub_key.as_ptr(),
                value_name.as_ptr(),
                RRF_RT_REG_DWORD,
                null_mut(),
                (&mut value as *mut u32).cast(),
                &mut size,
            );
            result == 0 && value == 0
        }
    }

    fn default_model_path() -> PathBuf {
        if let Some(path) = env::var_os("SEARCH_TOOL_MODEL") {
            return PathBuf::from(path);
        }
        if let Ok(exe) = env::current_exe() {
            if let Some(parent) = exe.parent() {
                for ancestor in parent.ancestors().take(3) {
                    let candidate = ancestor.join("models").join("tiny-intent-v1.stm");
                    if candidate.is_file() {
                        return candidate;
                    }
                }
            }
        }
        PathBuf::from("models").join("tiny-intent-v1.stm")
    }

    fn default_index_dir() -> PathBuf {
        env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
            .join("SearchTool")
            .join("index")
    }

    fn menu_id(id: usize) -> *mut c_void {
        std::ptr::with_exposed_provenance_mut::<c_void>(id)
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn show_error(error: &str) {
        let text = wide(error);
        let caption = wide("Search Tool");
        unsafe {
            message_box_w(null_mut(), text.as_ptr(), caption.as_ptr(), 0x10);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_search_uri_accepts_documented_search_query() {
            assert_eq!(
                parse_search_uri("search:query=hello%20world"),
                Some(SearchRequest {
                    query: Some("hello world".to_string()),
                    scope: None,
                })
            );
        }

        #[test]
        fn parse_search_uri_accepts_private_protocol_and_plus_spaces() {
            assert_eq!(
                parse_search_uri("searchtool:q=report+2026"),
                Some(SearchRequest {
                    query: Some("report 2026".to_string()),
                    scope: None,
                })
            );
        }

        #[test]
        fn search_uri_scheme_is_case_insensitive() {
            assert!(is_search_uri("SEARCH:query=hello"));
            assert!(is_search_uri("SearchTool:q=hello"));
            assert_eq!(
                parse_search_uri("SeArCh:query=hello"),
                Some(SearchRequest {
                    query: Some("hello".to_string()),
                    scope: None,
                })
            );
            assert_eq!(parse_search_uri("file:query=hello"), None);
        }

        #[test]
        fn ipc_empty_request_resets_to_global_search() {
            assert_eq!(decode_ipc_request(&[0, 0]), Some(SearchRequest::default()));
        }

        #[test]
        fn ipc_request_roundtrip_decodes_query_and_scope() {
            let request = SearchRequest {
                query: Some("report 2026".to_string()),
                scope: Some(r"C:\\Users\\umut\\Documents".to_string()),
            };
            let mut payload = Vec::<u16>::new();
            payload.extend(request.query.as_deref().unwrap_or("").encode_utf16());
            payload.push(0);
            payload.extend(request.scope.as_deref().unwrap_or("").encode_utf16());
            payload.push(0);
            assert_eq!(decode_ipc_request(&payload), Some(request));
        }

        #[test]
        fn parse_search_uri_accepts_explorer_location_crumb() {
            assert_eq!(
                parse_search_uri(
                    "search:query=report&crumb=location:C%3A%5CUsers%5Cumut%5CDocuments"
                ),
                Some(SearchRequest {
                    query: Some("report".to_string()),
                    scope: Some(r"C:\Users\umut\Documents".to_string()),
                })
            );
        }

        #[test]
        fn private_protocol_can_open_a_scope_without_query() {
            assert_eq!(
                parse_search_uri("searchtool:scope=C%3A%5CProjects"),
                Some(SearchRequest {
                    query: None,
                    scope: Some(r"C:\Projects".to_string()),
                })
            );
        }

        #[test]
        fn scope_candidate_filter_uses_directory_boundary() {
            assert_eq!(scope_filter_needle(r"C:\Projects"), r"c:\projects\");
            assert_eq!(scope_filter_needle(r"C:\"), r"c:\");
            assert!(!r"c:\projects-old\readme.md".contains(&scope_filter_needle(r"C:\Projects")));
            assert!(
                r"c:\projects\searchtool\readme.md".contains(&scope_filter_needle(r"C:\Projects"))
            );
        }

        #[test]
        fn scope_matching_does_not_leak_to_similar_prefixes() {
            assert!(path_is_within_scope(
                r"C:\Projects\SearchTool\README.md",
                r"C:\Projects"
            ));
            assert!(path_is_within_scope(r"C:\Projects", r"C:\Projects"));
            assert!(!path_is_within_scope(
                r"C:\Projects-old\README.md",
                r"C:\Projects"
            ));
        }

        #[test]
        fn explicit_path_filter_remains_an_additional_scope_constraint() {
            let parsed = parse_search_query(r#"report path:"C:\Projects\docs""#);
            let needle = parsed.filters.path_contains.as_deref();
            assert!(path_matches_explicit_filter(
                r"C:\Projects\docs\report.txt",
                needle
            ));
            assert!(!path_matches_explicit_filter(
                r"C:\Projects\src\report.txt",
                needle
            ));
        }

        #[test]
        fn shell_bridge_process_filter_is_narrow() {
            for name in [
                r"C:\Windows\SystemApps\Microsoft.Windows.Search\SearchApp.exe",
                r"C:\Windows\SystemApps\MicrosoftWindows.Client.CBS\SearchHost.exe",
                "SearchUI.exe",
                "StartMenuExperienceHost.exe",
                "ShellExperienceHost.exe",
            ] {
                assert!(is_shell_search_process_name(name), "{name}");
            }
            for name in [
                "explorer.exe",
                "cmd.exe",
                "powershell.exe",
                "SearchTool.exe",
            ] {
                assert!(!is_shell_search_process_name(name), "{name}");
            }

            assert!(is_explorer_process_name(r"C:\Windows\explorer.exe"));
            assert!(is_explorer_process_name("EXPLORER.EXE"));
            assert!(!is_explorer_process_name("SearchApp.exe"));
        }

        #[test]
        fn shell_bridge_routes_only_search_input_keys() {
            for vk in [
                VK_SPACE,
                b'A' as u32,
                b'Z' as u32,
                b'0' as u32,
                b'9' as u32,
                VK_BACK,
                VK_LEFT,
                VK_RIGHT,
                VK_DELETE,
                VK_RETURN as u32,
                VK_ESCAPE as u32,
            ] {
                assert!(is_bridge_routable_key(vk), "vk={vk:#x}");
            }
            for vk in [VK_LWIN, VK_RWIN, 0x70, 0x71, 0x5D] {
                assert!(!is_bridge_routable_key(vk), "vk={vk:#x}");
            }
        }

        #[test]
        fn utf16_backspace_boundary_keeps_surrogate_pairs_intact() {
            let text = "A😀B".encode_utf16().collect::<Vec<_>>();
            assert_eq!(previous_utf16_boundary(&text, text.len()), text.len() - 1);
            assert_eq!(previous_utf16_boundary(&text, text.len() - 1), 1);
            assert_eq!(previous_utf16_boundary(&text, 1), 0);
            assert_eq!(previous_utf16_boundary(&text, 0), 0);
        }

        #[test]
        fn control_shortcut_filter_is_explicit() {
            for vk in *b"ACVXZ" {
                assert!(is_supported_control_shortcut(vk as u32));
            }
            for vk in *b"BPY" {
                assert!(!is_supported_control_shortcut(vk as u32));
            }
        }

        #[test]
        fn explorer_search_classes_are_narrow() {
            for class in [
                "SearchEditBoxWrapperClass",
                "Search Box",
                "UniversalSearchBand",
            ] {
                assert!(is_explorer_search_class(class), "{class}");
            }
            for class in ["DirectUIHWND", "ToolbarWindow32", "CabinetWClass", "Edit"] {
                assert!(!is_explorer_search_class(class), "{class}");
            }
        }

        #[test]
        fn explorer_address_toolbar_scope_extraction_is_locale_agnostic() {
            assert_eq!(
                extract_explorer_scope_from_toolbar_text(r"Adres: C:\Users\umut\Projects"),
                Some(r"C:\Users\umut\Projects".to_string())
            );
            assert_eq!(
                extract_explorer_scope_from_toolbar_text(r"Address: D:\Code\Search Tool\"),
                Some(r"D:\Code\Search Tool".to_string())
            );
            assert_eq!(
                extract_explorer_scope_from_toolbar_text(r"Address: \\server\share\folder"),
                Some(r"\\server\share\folder".to_string())
            );
            assert_eq!(
                extract_explorer_scope_from_toolbar_text("Gezinti düğmeleri"),
                None
            );
        }

        #[test]
        fn dpi_scaling_uses_96_dpi_logical_units() {
            assert_eq!(normalize_dpi(0), BASE_DPI);
            assert_eq!(scale_px(18, 0), 18);
            assert_eq!(scale_px(18, 96), 18);
            assert_eq!(scale_px(18, 144), 27);
            assert_eq!(scale_px(58, 192), 116);
            assert_eq!(scale_px(1, 120), 1);
        }

        #[test]
        fn windows_build_capabilities_are_explicitly_gated() {
            for build in [
                10_240, 10_586, 14_393, 15_063, 16_299, 17_134, 17_763, 18_362, 18_363, 19_041,
                19_042, 19_043, 19_044, 19_045,
            ] {
                assert_eq!(platform_label(build), "WINDOWS 10");
                assert!(!supports_modern_frame(build));
                assert!(!supports_system_backdrop(build));
            }

            for build in [22_000, 22_621, 22_631, 26_100] {
                assert_eq!(platform_label(build), "WINDOWS 11");
                assert!(supports_modern_frame(build));
            }
            assert!(!supports_system_backdrop(22_000));
            assert!(supports_system_backdrop(22_621));
            assert!(supports_system_backdrop(22_631));
            assert!(supports_system_backdrop(26_100));
            assert_eq!(platform_label(0), "WINDOWS COMPAT");
        }

        #[test]
        fn background_fit_geometry_is_deterministic() {
            let bounds = Rect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 100,
            };
            assert_eq!(
                image_destination_rect(200, 100, bounds, BackgroundFit::Fit),
                Rect {
                    left: 0,
                    top: 25,
                    right: 100,
                    bottom: 75,
                }
            );
            assert_eq!(
                image_destination_rect(200, 100, bounds, BackgroundFit::Fill),
                Rect {
                    left: -50,
                    top: 0,
                    right: 150,
                    bottom: 100,
                }
            );
            assert_eq!(
                image_destination_rect(200, 100, bounds, BackgroundFit::Stretch),
                bounds
            );
        }

        #[test]
        fn centered_window_rect_handles_negative_monitor_origins() {
            let work = Rect {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1080,
            };
            assert_eq!(
                centered_window_rect(work, 820, 590, 144),
                Rect {
                    left: -1575,
                    top: 39,
                    right: -345,
                    bottom: 924,
                }
            );
        }

        #[test]
        fn clamp_window_rect_recovers_a_removed_monitor() {
            let stale = Rect {
                left: 2100,
                top: -200,
                right: 2920,
                bottom: 390,
            };
            let remaining_work = Rect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040,
            };
            assert_eq!(
                clamp_window_rect(stale, remaining_work),
                Rect {
                    left: 1100,
                    top: 0,
                    right: 1920,
                    bottom: 590,
                }
            );
        }

        #[test]
        fn clamp_window_rect_shrinks_oversized_windows_to_work_area() {
            let oversized = Rect {
                left: -500,
                top: -400,
                right: 2500,
                bottom: 1800,
            };
            let work = Rect {
                left: 100,
                top: 50,
                right: 1100,
                bottom: 850,
            };
            assert_eq!(clamp_window_rect(oversized, work), work);
        }
    }
}

#[cfg(windows)]
fn main() {
    if let Err(error) = windows_app::run() {
        windows_app::show_error(&error.to_string());
        std::process::exit(1);
    }
}
