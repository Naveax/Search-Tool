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
        collections::HashMap,
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

    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
    const WS_EX_APPWINDOW: u32 = 0x0004_0000;
    const WS_EX_LAYERED: u32 = 0x0008_0000;
    const ES_AUTOHSCROLL: u32 = 0x0080;
    const LBS_NOTIFY: u32 = 0x0001;
    const LBS_OWNERDRAWFIXED: u32 = 0x0010;
    const LBS_HASSTRINGS: u32 = 0x0040;
    const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
    const SS_LEFT: u32 = 0x0000;
    const SS_NOPREFIX: u32 = 0x0080;
    const SS_ENDELLIPSIS: u32 = 0x4000;
    const SS_PATHELLIPSIS: u32 = 0x8000;
    const BS_OWNERDRAW: u32 = 0x000B;

    const SW_HIDE: i32 = 0;
    const SW_SHOW: i32 = 5;
    const SW_SHOWNOACTIVATE: i32 = 4;
    const SW_RESTORE: i32 = 9;
    const SW_SHOWNORMAL: i32 = 1;

    const WM_NCCREATE: u32 = 0x0081;
    const WM_NCDESTROY: u32 = 0x0082;
    const WM_CREATE: u32 = 0x0001;
    const WM_DESTROY: u32 = 0x0002;
    const WM_SIZE: u32 = 0x0005;
    const WM_ACTIVATE: u32 = 0x0006;
    const WM_KILLFOCUS: u32 = 0x0008;
    const WM_SETTINGCHANGE: u32 = 0x001A;
    const WM_SYSCOLORCHANGE: u32 = 0x0015;
    // Standard MSAA WinEvent: notify assistive tools when LISTBOX's
    // derived accessible name (from its preceding STATIC label) changes.
    const EVENT_OBJECT_NAMECHANGE: u32 = 0x800C;
    const OBJID_CLIENT: i32 = -4;
    const WM_DISPLAYCHANGE: u32 = 0x007E;
    const WM_DPICHANGED: u32 = 0x02E0;
    const WM_COMMAND: u32 = 0x0111;
    const WM_CLOSE: u32 = 0x0010;
    const WM_QUIT: u32 = 0x0012;
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
    const WM_MOUSEMOVE: u32 = 0x0200;
    const WM_MOUSELEAVE: u32 = 0x02A3;
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_IME_STARTCOMPOSITION: u32 = 0x010D;
    const WM_IME_ENDCOMPOSITION: u32 = 0x010E;
    const WM_KEYUP: u32 = 0x0101;
    const WM_SYSKEYDOWN: u32 = 0x0104;
    const WM_SYSKEYUP: u32 = 0x0105;
    const WM_SETFONT: u32 = 0x0030;
    const WM_SHELL_BRIDGE_BEGIN: u32 = 0x8000 + 0x51;
    const WM_SHELL_BRIDGE_CHAR: u32 = 0x8000 + 0x52;
    const WM_SHELL_BRIDGE_KEY: u32 = 0x8000 + 0x53;
    const WM_THEME_BUTTON_HOT: u32 = 0x8000 + 0x54;

    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const GWLP_USERDATA: i32 = -21;
    const EN_SETFOCUS: usize = 0x0100;
    const EN_KILLFOCUS: usize = 0x0200;
    const EN_CHANGE: usize = 0x0300;
    const BN_CLICKED: usize = 0;
    const LBN_SELCHANGE: usize = 1;
    const LBN_DBLCLK: usize = 2;

    const LB_ADDSTRING: u32 = 0x0180;
    const LB_INSERTSTRING: u32 = 0x0181;
    const LB_DELETESTRING: u32 = 0x0182;
    const LB_GETTEXT: u32 = 0x0189;
    const LB_GETTEXTLEN: u32 = 0x018A;
    const PM_REMOVE: u32 = 0x0001;
    const LB_RESETCONTENT: u32 = 0x0184;
    const LB_SETCURSEL: u32 = 0x0186;
    const LB_GETCURSEL: u32 = 0x0188;
    const LB_GETCOUNT: u32 = 0x018B;
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
    const ODS_CHECKED: u32 = 0x0008;
    const ODS_FOCUS: u32 = 0x0010;
    const ODS_HOTLIGHT: u32 = 0x0040;
    const TME_LEAVE: u32 = 0x0000_0002;
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
    const MAX_NATIVE_LABEL_U16: usize = 65_536;
    const MAX_SHELL_ICON_TYPES: usize = 96;
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
    const SHGFI_ICON: u32 = 0x0000_0100;
    const SHGFI_SMALLICON: u32 = 0x0000_0001;
    const SHGFI_USEFILEATTRIBUTES: u32 = 0x0000_0010;
    const DI_NORMAL: u32 = 0x0003;

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
    // USER32 holds raw item_data pointers while the popup is active; boxing keeps
    // every descriptor at a stable address even if the backing Vec reallocates.
    #[allow(clippy::vec_box)]
    static THEME_MENU_VISUALS: Mutex<Vec<Box<MenuItemVisual>>> = Mutex::new(Vec::new());

    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
    const BASE_DPI: u32 = 96;
    const ERROR_ALREADY_EXISTS: u32 = 183;

    const LWA_ALPHA: u32 = 0x0000_0002;
    const SPI_SETWORKAREA: u32 = 0x002F;
    const SPI_GETWORKAREA: u32 = 0x0030;
    const SPI_GETHIGHCONTRAST: u32 = 0x0042;
    const HCF_HIGHCONTRASTON: u32 = 0x0001;
    const COLOR_WINDOW: i32 = 5;
    const COLOR_WINDOWTEXT: i32 = 8;
    const COLOR_HIGHLIGHT: i32 = 13;
    const COLOR_HIGHLIGHTTEXT: i32 = 14;
    const GCLP_HBRBACKGROUND: i32 = -10;
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
    const MF_POPUP: u32 = 0x0010;
    const MFT_OWNERDRAW: u32 = 0x0100;
    const MIIM_FTYPE: u32 = 0x0100;
    const MIIM_STRING: u32 = 0x0040;
    const MIIM_DATA: u32 = 0x0020;
    const MIM_BACKGROUND: u32 = 0x0000_0002;
    const ODT_MENU: u32 = 1;
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
    const ID_DETAIL_HEADER: usize = 15;
    const ID_DETAIL_NAME: usize = 16;
    const ID_DETAIL_KIND: usize = 17;
    const ID_DETAIL_PATH: usize = 18;
    const ID_DETAIL_OPEN: usize = 19;
    const ID_SEARCH_ACCESSIBLE_LABEL: usize = 20;
    const ID_RESULTS_ACCESSIBLE_LABEL: usize = 21;
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
    struct HighContrastW {
        cb_size: u32,
        flags: u32,
        default_scheme: *mut u16,
    }

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
    struct TrackMouseEvent {
        cb_size: u32,
        flags: u32,
        hwnd_track: Hwnd,
        hover_time: u32,
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
    struct ShFileInfoW {
        icon: Hicon,
        icon_index: i32,
        attributes: u32,
        display_name: [u16; 260],
        type_name: [u16; 80],
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
    struct MenuInfo {
        cb_size: u32,
        mask: u32,
        style: u32,
        max_height: u32,
        background: Hbrush,
        context_help_id: u32,
        menu_data: usize,
    }

    #[repr(C)]
    struct MenuItemInfoW {
        cb_size: u32,
        mask: u32,
        item_type: u32,
        state: u32,
        id: u32,
        submenu: Hmenu,
        checked_bitmap: *mut c_void,
        unchecked_bitmap: *mut c_void,
        item_data: usize,
        type_data: *mut u16,
        text_len: u32,
        item_bitmap: *mut c_void,
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
        #[link_name = "DestroyIcon"]
        fn destroy_icon(icon: Hicon) -> i32;
        #[link_name = "DrawIconEx"]
        fn draw_icon_ex(
            dc: Hdc,
            left: i32,
            top: i32,
            icon: Hicon,
            width: i32,
            height: i32,
            step: u32,
            brush: Hbrush,
            flags: u32,
        ) -> i32;
        #[link_name = "GetMessageW"]
        fn get_message_w(msg: *mut Msg, hwnd: Hwnd, min: u32, max: u32) -> i32;
        #[link_name = "PeekMessageW"]
        fn peek_message_w(msg: *mut Msg, hwnd: Hwnd, min: u32, max: u32, remove: u32) -> i32;
        #[link_name = "IsDialogMessageW"]
        fn is_dialog_message_w(hwnd: Hwnd, msg: *mut Msg) -> i32;
        #[link_name = "GetNextDlgTabItem"]
        fn get_next_dlg_tab_item(hwnd: Hwnd, control: Hwnd, previous: i32) -> Hwnd;
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
        #[link_name = "SetClassLongPtrW"]
        fn set_class_long_ptr_w(hwnd: Hwnd, index: i32, value: isize) -> isize;
        #[link_name = "GetSysColor"]
        fn get_sys_color(index: i32) -> u32;
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
        #[link_name = "NotifyWinEvent"]
        fn notify_win_event(event: u32, hwnd: Hwnd, object_id: i32, child_id: i32);
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
        #[link_name = "TrackMouseEvent"]
        fn track_mouse_event(event: *mut TrackMouseEvent) -> i32;
        #[link_name = "DrawFocusRect"]
        fn draw_focus_rect(hdc: Hdc, rect: *const Rect) -> i32;
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
        #[link_name = "GetMenuItemCount"]
        fn get_menu_item_count(menu: Hmenu) -> i32;
        #[link_name = "SetMenuInfo"]
        fn set_menu_info(menu: Hmenu, info: *const MenuInfo) -> i32;
        #[link_name = "SetMenuItemInfoW"]
        fn set_menu_item_info_w(
            menu: Hmenu,
            item: u32,
            by_position: i32,
            info: *const MenuItemInfoW,
        ) -> i32;
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

    #[link(name = "comctl32")]
    extern "system" {
        #[link_name = "SetWindowSubclass"]
        fn set_window_subclass(
            hwnd: Hwnd,
            proc: Option<
                unsafe extern "system" fn(Hwnd, u32, Wparam, Lparam, usize, usize) -> Lresult,
            >,
            id: usize,
            ref_data: usize,
        ) -> i32;
        #[link_name = "RemoveWindowSubclass"]
        fn remove_window_subclass(
            hwnd: Hwnd,
            proc: Option<
                unsafe extern "system" fn(Hwnd, u32, Wparam, Lparam, usize, usize) -> Lresult,
            >,
            id: usize,
        ) -> i32;
        #[link_name = "DefSubclassProc"]
        fn def_subclass_proc(hwnd: Hwnd, msg: u32, w_param: Wparam, l_param: Lparam) -> Lresult;
    }

    #[link(name = "gdi32")]
    extern "system" {
        #[link_name = "CreateSolidBrush"]
        fn create_solid_brush(color: u32) -> Hbrush;
        #[link_name = "GetStockObject"]
        fn get_stock_object(index: i32) -> Hgdiobj;
        #[link_name = "RoundRect"]
        fn round_rect(
            hdc: Hdc,
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
            ellipse_width: i32,
            ellipse_height: i32,
        ) -> i32;
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
        #[link_name = "SHGetFileInfoW"]
        fn sh_get_file_info_w(
            path: *const u16,
            file_attributes: u32,
            info: *mut ShFileInfoW,
            info_size: u32,
            flags: u32,
        ) -> usize;
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

    struct MenuItemVisual {
        label: Vec<u16>,
        has_submenu: bool,
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

    fn should_show_at_launch(smoke: bool, resident: bool, has_request: bool) -> bool {
        !smoke && (!resident || has_request)
    }

    struct State {
        store: MultiLiveSearchStore,
        edit: Hwnd,
        list: Hwnd,
        search_label: Hwnd,
        results_label: Hwnd,
        title: Hwnd,
        subtitle: Hwnd,
        status: Hwnd,
        tabs: [Hwnd; 4],
        theme_button: Hwnd,
        detail_header: Hwnd,
        detail_name: Hwnd,
        detail_kind: Hwnd,
        detail_path: Hwnd,
        detail_open: Hwnd,
        edit_focused: bool,
        ime_composing: bool,
        programmatic_edit_update: bool,
        theme_button_hot: bool,
        resident: bool,
        hotkey_registered: bool,
        intent_model: Option<TinyIntentModel>,
        model_path: PathBuf,
        initial_request: Option<SearchRequest>,
        scope: Option<String>,
        mode: SearchMode,
        results: Vec<ResultRow>,
        shell_icons: HashMap<String, Hicon>,
        theme: UiTheme,
        palette: Palette,
        dark: bool,
        high_contrast: bool,
        background_brush: Hbrush,
        surface_brush: Hbrush,
        accent_brush: Hbrush,
        muted_brush: Hbrush,
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
                    self.muted_brush as Hgdiobj,
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
        if state.high_contrast || state.background_image.is_null() {
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

    unsafe fn monitor_layout(hwnd: Hwnd) -> Option<(Rect, Rect)> {
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
        (get_monitor_info_w(monitor, &mut info) != 0).then_some((info.work, info.monitor))
    }

    unsafe fn monitor_work_area(hwnd: Hwnd) -> Option<Rect> {
        monitor_layout(hwnd).map(|(work, _)| work)
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
        center_search_window(
            hwnd,
            state.theme.width,
            state.theme.height,
            state.dpi,
            state.resident,
        );
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
        let mut ui_theme = UiTheme::load();
        let mut dark = match ui_theme.mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => system_prefers_dark(),
        };
        let os_build = unsafe { windows_build_number() };

        let mut resident = false;
        // Native-first policy: Windows Search and Explorer keep their own UI.
        // The legacy keyboard bridge is available only as an explicit opt-in.
        let mut shell_bridge = false;
        let mut smoke = false;
        let mut ui_preview = false;
        let mut ui_selftest = false;
        let mut ui_selftest_winevent_offscreen = false;
        let mut ui_selftest_inspect_ms = 0_u64;
        let mut ui_selftest_report: Option<PathBuf> = None;
        let mut index_source = None;
        let mut initial_request = SearchRequest::default();
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--resident" => resident = true,
                "--shell-bridge" => shell_bridge = true,
                "--no-shell-bridge" => shell_bridge = false,
                "--smoke" => smoke = true,
                "--ui-preview" => ui_preview = true,
                "--ui-selftest" => ui_selftest = true,
                "--ui-selftest-winevent-offscreen" => ui_selftest_winevent_offscreen = true,
                "--ui-selftest-inspect-ms" => {
                    ui_selftest_inspect_ms = args
                        .next()
                        .and_then(|value| value.parse::<u64>().ok())
                        .unwrap_or(0)
                        .min(30_000);
                }
                "--ui-selftest-report" => ui_selftest_report = args.next().map(PathBuf::from),
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
        if ui_selftest_winevent_offscreen && (!ui_selftest || ui_selftest_inspect_ms == 0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "offscreen WinEvent probe requires --ui-selftest and bounded inspection",
            ));
        }
        if ui_selftest {
            // This opt-in developer test is silent and cannot take over the desktop.
            resident = true;
            shell_bridge = false;
            smoke = false;
            ui_preview = false;
            ui_theme.apply_preset(ThemePreset::Native);
            ui_theme.width = 780;
            ui_theme.height = 720;
            dark = system_prefers_dark();
            if index_source.is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--ui-selftest requires an explicit isolated index path",
                ));
            }
        }
        // The resident flyout defaults to familiar Windows surfaces, while
        // deliberately selected non-signature presets retain their styles.
        if resident && ui_theme.preset == ThemePreset::Signature {
            ui_theme.apply_preset(ThemePreset::Native);
            // Measured from the real Windows 11 Search flyout at 96 DPI.
            // Keep explicit user-selected dimensions unchanged.
            if os_build >= 22_000 && ui_theme.width == 900 && ui_theme.height == 640 {
                ui_theme.width = 780;
                ui_theme.height = 720;
            }
            dark = system_prefers_dark();
        }
        // Read-only Windows accessibility state; never modify user settings.
        let high_contrast = unsafe { system_high_contrast_enabled() };
        let palette = resolve_palette(&ui_theme, dark, high_contrast);
        let initial_request = (!initial_request.is_empty()).then_some(initial_request);
        // WM_CREATE consumes State::initial_request before CreateWindowExW
        // returns, so preserve launch visibility outside that state.
        let show_at_launch =
            should_show_at_launch(smoke || ui_selftest, resident, initial_request.is_some());
        let index_source = index_source.unwrap_or_else(default_index_dir);

        let mutex_name = wide(r"Local\SearchToolGui");
        let mutex = unsafe { create_mutex_w(null_mut(), 0, mutex_name.as_ptr()) };
        if mutex.is_null() {
            return Err(io::Error::last_os_error());
        }
        let mutex = MutexGuard(mutex);
        if unsafe { get_last_error() } == ERROR_ALREADY_EXISTS {
            if ui_selftest {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "UI self-test will not reuse or foreground a running Search Tool",
                ));
            }
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
        let muted_brush = unsafe { create_solid_brush(palette.muted.colorref()) };
        if background_brush.is_null()
            || surface_brush.is_null()
            || accent_brush.is_null()
            || muted_brush.is_null()
        {
            return Err(io::Error::last_os_error());
        }
        let gdiplus_token = unsafe { start_gdiplus() };
        let background_image = unsafe { load_theme_background(&ui_theme, gdiplus_token) };

        let mut state = Box::new(State {
            store,
            edit: null_mut(),
            list: null_mut(),
            search_label: null_mut(),
            results_label: null_mut(),
            title: null_mut(),
            subtitle: null_mut(),
            status: null_mut(),
            tabs: [null_mut(); 4],
            theme_button: null_mut(),
            detail_header: null_mut(),
            detail_name: null_mut(),
            detail_kind: null_mut(),
            detail_path: null_mut(),
            detail_open: null_mut(),
            edit_focused: false,
            ime_composing: false,
            programmatic_edit_update: false,
            theme_button_hot: false,
            resident,
            hotkey_registered: false,
            intent_model: None,
            model_path: default_model_path(),
            initial_request,
            scope: None,
            mode: SearchMode::All,
            results: Vec::new(),
            shell_icons: HashMap::new(),
            theme: ui_theme,
            palette,
            dark,
            high_contrast,
            background_brush,
            surface_brush,
            accent_brush,
            muted_brush,
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
        let ex_style = (if !state.high_contrast && state.theme.alpha() < 255 {
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
                WS_POPUP | WS_CLIPCHILDREN | if resident { 0 } else { WS_THICKFRAME },
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
                (*raw_state).resident,
            );

            if !show_at_launch {
                show_window(hwnd, SW_HIDE);
            } else {
                show_window(hwnd, SW_SHOW);
                if resident {
                    // Explicit user search requests should open in the foreground.
                    set_foreground_window(hwnd);
                    set_focus((*raw_state).edit);
                }
                update_window(hwnd);
            }
        }
        if ui_selftest {
            // The controls exist on this thread, but the window stays hidden.
            // Always tear down owned HICONs/Win32 controls even on assertion failure.
            let outcome = unsafe { run_hidden_ui_regression(hwnd, raw_state) };
            if let Some(path) = ui_selftest_report.as_ref() {
                let result = match &outcome {
                    Ok(()) => "PASS".to_string(),
                    Err(error) => format!("FAIL: {error}"),
                };
                let _ = std::fs::write(path, result);
            }
            if ui_selftest_inspect_ms > 0 && outcome.is_ok() {
                if ui_selftest_winevent_offscreen {
                    // Explicit isolated WinEvent fixture, never the user's
                    // foreground window: visible *style* for IsWindowVisible,
                    // placed far outside ordinary display coordinates, and
                    // shown without activation or keyboard focus.
                    unsafe {
                        move_window(hwnd, -30_000, -30_000, 780, 720, 0);
                        show_window(hwnd, SW_SHOWNOACTIVATE);
                        // Some headless window managers correct a position
                        // on first ShowWindow. Keep the synthetic popup
                        // outside the desktop even after first display.
                        move_window(hwnd, -30_000, -30_000, 780, 720, 0);
                    }
                }
                // The ordinary MSAA inspection path stays hidden. The opt-in
                // WinEvent path has WS_VISIBLE offscreen, so real listeners
                // receive notifications after category changes.
                // Both paths are bounded and use only synthetic test data.
                let until =
                    Instant::now() + std::time::Duration::from_millis(ui_selftest_inspect_ms);
                while Instant::now() < until {
                    let mut pending = Msg {
                        hwnd: null_mut(),
                        message: 0,
                        w_param: 0,
                        l_param: 0,
                        time: 0,
                        pt_x: 0,
                        pt_y: 0,
                        private: 0,
                    };
                    if unsafe { peek_message_w(&mut pending, null_mut(), 0, 0, PM_REMOVE) } != 0 {
                        if pending.message == WM_QUIT {
                            break;
                        }
                        unsafe {
                            translate_message(&pending);
                            dispatch_message_w(&pending);
                        }
                    } else {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                }
            }
            if unsafe { destroy_window(hwnd) } == 0 {
                return Err(io::Error::last_os_error());
            }
            return outcome;
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

            // While an IME is composing inside EDIT, Enter/Escape/arrows/Tab
            // belong to the IME candidate window, not the search flyout.
            let popup_keys_allowed = unsafe {
                let state_ptr = get_window_long_ptr_w(hwnd, GWLP_USERDATA) as *const State;
                state_ptr.is_null()
                    || popup_shortcuts_allowed(
                        get_focus() == (*state_ptr).edit,
                        (*state_ptr).ime_composing,
                    )
            };
            if msg.message == WM_KEYDOWN && popup_keys_allowed {
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
                        if !state_ptr.is_null() {
                            let state = &mut *state_ptr;
                            // Native Search and the Shell bridge must agree:
                            // Enter from the query selects Best match only
                            // when the list has no selected row already.
                            let _ =
                                prepare_search_enter_selection(state, get_focus() == state.edit);
                            // Route Enter to search only for Edit and ListBox focus.
                            // Category, appearance and Open buttons handle themselves.
                            if should_route_result_enter(get_focus(), state.edit, state.list)
                                && open_selected(hwnd, state)
                            {
                                continue;
                            }
                        }
                    },
                    VK_DOWN => unsafe {
                        let state_ptr = get_window_long_ptr_w(hwnd, GWLP_USERDATA) as *mut State;
                        if !state_ptr.is_null()
                            && get_focus() == (*state_ptr).edit
                            && prepare_query_down_selection(&*state_ptr)
                        {
                            // Preserve the user's selected hit after Shift+Tab
                            // or requery; only create a selection if none exists.
                            set_focus((*state_ptr).list);
                            continue;
                        }
                    },
                    key if key == VK_UP as usize => unsafe {
                        let state_ptr = get_window_long_ptr_w(hwnd, GWLP_USERDATA) as *mut State;
                        if !state_ptr.is_null()
                            && get_focus() == (*state_ptr).list
                            && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                        {
                            // Up from the best match returns to the query field.
                            set_focus((*state_ptr).edit);
                            continue;
                        }
                    },
                    _ => {}
                }
            }

            // WS_TABSTOP alone is insufficient for a custom Win32 popup.
            // IsDialogMessageW supplies native forward/reverse Tab traversal
            // without intercepting other keys, including text and IME input.
            if popup_keys_allowed
                && should_handle_dialog_tab(msg.message, msg.w_param)
                && unsafe { is_dialog_message_w(hwnd, &mut msg) } != 0
            {
                continue;
            }
            unsafe {
                translate_message(&msg);
                dispatch_message_w(&msg);
            }
        }
        Ok(())
    }

    // Observe standard EDIT IME notifications without handling composition
    // ourselves. Always forward messages to the native edit procedure.
    unsafe extern "system" fn edit_ime_subclass_proc(
        hwnd: Hwnd,
        msg: u32,
        w_param: Wparam,
        l_param: Lparam,
        subclass_id: usize,
        ref_data: usize,
    ) -> Lresult {
        let parent = ref_data as Hwnd;
        let state_ptr = if parent.is_null() {
            null_mut()
        } else {
            get_window_long_ptr_w(parent, GWLP_USERDATA) as *mut State
        };
        if msg == WM_IME_STARTCOMPOSITION && !state_ptr.is_null() {
            (*state_ptr).ime_composing = true;
        }
        let result = def_subclass_proc(hwnd, msg, w_param, l_param);
        // The native EDIT may emit EN_CHANGE while finalizing text. Keep the
        // composition guard active until the default procedure has returned.
        if (msg == WM_IME_ENDCOMPOSITION || msg == WM_KILLFOCUS)
            && !state_ptr.is_null()
            && (*state_ptr).ime_composing
        {
            (*state_ptr).ime_composing = false;
            refresh_results(&mut *state_ptr);
        }
        if msg == WM_NCDESTROY {
            let _ = remove_window_subclass(hwnd, Some(edit_ime_subclass_proc), subclass_id);
        }
        result
    }

    unsafe extern "system" fn theme_button_subclass_proc(
        hwnd: Hwnd,
        msg: u32,
        w_param: Wparam,
        l_param: Lparam,
        subclass_id: usize,
        ref_data: usize,
    ) -> Lresult {
        let parent = ref_data as Hwnd;
        match msg {
            WM_MOUSEMOVE => {
                let mut event = TrackMouseEvent {
                    cb_size: std::mem::size_of::<TrackMouseEvent>() as u32,
                    flags: TME_LEAVE,
                    hwnd_track: hwnd,
                    hover_time: 0,
                };
                let _ = track_mouse_event(&mut event);
                if !parent.is_null() {
                    let _ = send_message_w(parent, WM_THEME_BUTTON_HOT, 1, 0);
                }
            }
            WM_MOUSELEAVE => {
                if !parent.is_null() {
                    let _ = send_message_w(parent, WM_THEME_BUTTON_HOT, 0, 0);
                }
            }
            WM_NCDESTROY => {
                let _ = remove_window_subclass(hwnd, Some(theme_button_subclass_proc), subclass_id);
            }
            _ => {}
        }
        def_subclass_proc(hwnd, msg, w_param, l_param)
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
            // A taskbar flyout closes when focus moves elsewhere, unlike a
            // persistent centered application window.
            WM_ACTIVATE if !state_ptr.is_null() => {
                if (*state_ptr).resident && (w_param & 0xffff) == 0 {
                    SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
                    show_window(hwnd, SW_HIDE);
                }
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
            WM_SYSCOLORCHANGE if !state_ptr.is_null() => {
                // Windows notifies us after accessible system colors change.
                // Re-read the system contrast colors without modifying settings.
                apply_runtime_theme(hwnd, &mut *state_ptr);
                0
            }
            WM_SETTINGCHANGE if !state_ptr.is_null() => {
                if w_param as u32 == SPI_SETWORKAREA {
                    recover_window_to_monitor(hwnd, &mut *state_ptr);
                }
                // A contrast-mode transition may be announced by either
                // WM_SETTINGCHANGE or WM_SYSCOLORCHANGE; avoid duplicate work.
                if (*state_ptr).high_contrast != system_high_contrast_enabled() {
                    apply_runtime_theme(hwnd, &mut *state_ptr);
                }
                0
            }
            WM_THEME_BUTTON_HOT if !state_ptr.is_null() => {
                let state = &mut *state_ptr;
                let hot = w_param != 0;
                if state.theme_button_hot != hot {
                    state.theme_button_hot = hot;
                    let _ = invalidate_rect(state.theme_button, null_mut(), 0);
                }
                0
            }
            WM_COMMAND if !state_ptr.is_null() => {
                let notification = (w_param >> 16) & 0xffff;
                let source = l_param as Hwnd;
                let state = &mut *state_ptr;
                if source == state.edit && notification == EN_SETFOCUS {
                    state.edit_focused = true;
                    invalidate_search_frame(hwnd, state);
                    return 0;
                }
                if source == state.edit && notification == EN_KILLFOCUS {
                    state.edit_focused = false;
                    invalidate_search_frame(hwnd, state);
                    return 0;
                }
                if source == state.edit && notification == EN_CHANGE {
                    // Partial/preedit IME text is not a committed search query.
                    // The EDIT subclass refreshes once composition is finished.
                    if !state.ime_composing && !state.programmatic_edit_update {
                        refresh_results(state);
                    }
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
                        // Re-selecting an already active category must not
                        // repeat an expensive search or disturb its status,
                        // selection and detail view. The edit/query change
                        // handler remains responsible for real re-queries.
                        if state.mode == mode {
                            return 0;
                        }
                        state.mode = mode;
                        update_tab_labels(state);
                        refresh_results(state);
                        return 0;
                    }
                    if source == state.detail_open {
                        let _ = open_selected(hwnd, state);
                        return 0;
                    }
                    if source == state.theme_button {
                        show_theme_menu(hwnd, state);
                        return 0;
                    }
                }
                if source == state.list && notification == LBN_SELCHANGE {
                    // Mouse or keyboard notifications can arrive for a native
                    // row whose item-data/label was silently changed. Do not
                    // leave a bogus selected row highlighted for accessibility
                    // clients after its detail card has been cleared.
                    if state.resident
                        && state.theme.preset == ThemePreset::Native
                        && supports_modern_frame(state.os_build)
                        && send_message_w(state.list, LB_GETCURSEL, 0, 0) >= 0
                        && selected_detail_row(state).is_none()
                    {
                        reject_native_keyboard_selection(state);
                    } else {
                        update_detail_controls(state);
                    }
                    return 0;
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
                    // Unlike regular WM_KEYDOWN, bridge messages skip the
                    // outer IME guard. An IME candidate-selection Enter must
                    // reach native EDIT, never open a search result.
                    if popup_shortcuts_allowed(true, state.ime_composing) {
                        let _ = prepare_search_enter_selection(state, true);
                        let _ = open_selected(hwnd, state);
                    } else {
                        let _ = send_message_w(state.edit, WM_KEYDOWN, vk as usize, key_lparam);
                    }
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
                    center_search_window(
                        hwnd,
                        state.theme.width,
                        state.theme.height,
                        state.dpi,
                        state.resident,
                    );
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
                            state.resident,
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
                if measure.ctl_type == ODT_MENU
                    && measure.item_data != 0
                    && measure_theme_menu_item(&*state_ptr, measure)
                {
                    return 1;
                }
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
                if draw.ctl_type == ODT_MENU && draw.item_data != 0 {
                    draw_theme_menu_item(&*state_ptr, draw);
                    return 1;
                }
                if draw.ctl_id as usize == ID_LIST {
                    draw_result_row(&*state_ptr, draw);
                    return 1;
                }
                if [ID_ALL, ID_FILES, ID_FOLDERS, ID_CONTENT].contains(&(draw.ctl_id as usize)) {
                    draw_filter_chip(&*state_ptr, draw);
                    return 1;
                }
                if draw.ctl_id as usize == ID_THEME {
                    draw_theme_button(&*state_ptr, draw);
                    return 1;
                }
                if draw.ctl_id as usize == ID_DETAIL_OPEN {
                    draw_detail_open_button(&*state_ptr, draw);
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
                    let hdc = w_param as Hdc;
                    fill_rect(hdc, &rect, state.background_brush);
                    draw_background_image(state, hdc, rect);
                    let native = state.resident
                        && state.theme.preset == ThemePreset::Native
                        && supports_modern_frame(state.os_build);
                    if let Some(frame) = search_frame_rect(hwnd, state) {
                        if native {
                            let border_brush = if state.edit_focused {
                                state.accent_brush
                            } else {
                                state.muted_brush
                            };
                            fill_rounded_surface(hdc, frame, border_brush, scale_px(9, state.dpi));
                            let inner = Rect {
                                left: frame.left + 1,
                                top: frame.top + 1,
                                right: frame.right - 1,
                                bottom: frame.bottom - 1,
                            };
                            fill_rounded_surface(
                                hdc,
                                inner,
                                state.surface_brush,
                                scale_px(8, state.dpi),
                            );
                        } else {
                            fill_rect(
                                hdc,
                                &frame,
                                if state.edit_focused {
                                    state.accent_brush
                                } else {
                                    state.muted_brush
                                },
                            );
                        }
                    }
                    if native && !state.results.is_empty() {
                        if let Some((_, detail)) = native_result_columns(rect, state.dpi) {
                            fill_rounded_surface(
                                hdc,
                                detail,
                                state.surface_brush,
                                scale_px(12, state.dpi),
                            );
                        }
                    }
                    if native && state.results.is_empty() {
                        let has_query =
                            !state.edit.is_null() && get_window_text_length_w(state.edit) > 0;
                        let heading = wide(if has_query {
                            "Eşleşme bulunamadı"
                        } else {
                            "Aramaya başlayın"
                        });
                        let description = wide(if has_query {
                            "Farklı bir kelime veya daha kısa bir arama deneyin."
                        } else {
                            "Dosyalarınızı ve klasörlerinizi hızlıca bulun."
                        });
                        let top =
                            scale_px(275, state.dpi).min(rect.bottom - scale_px(85, state.dpi));
                        let margin = scale_px(36, state.dpi);
                        set_bk_mode(hdc, TRANSPARENT);
                        let old_font = select_object(hdc, state.title_font as Hgdiobj);
                        set_text_color(hdc, state.palette.text.colorref());
                        let mut headline = Rect {
                            left: margin,
                            top,
                            right: rect.right - margin,
                            bottom: top + scale_px(44, state.dpi),
                        };
                        draw_text_w(
                            hdc,
                            heading.as_ptr(),
                            -1,
                            &mut headline,
                            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                        );
                        select_object(hdc, state.small_font as Hgdiobj);
                        set_text_color(hdc, state.palette.muted.colorref());
                        let mut detail = Rect {
                            left: margin,
                            top: top + scale_px(47, state.dpi),
                            right: rect.right - margin,
                            bottom: top + scale_px(84, state.dpi),
                        };
                        draw_text_w(
                            hdc,
                            description.as_ptr(),
                            -1,
                            &mut detail,
                            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                        );
                        if !old_font.is_null() {
                            select_object(hdc, old_font);
                        }
                    }
                    return 1;
                }
                0
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN if !state_ptr.is_null() => {
                let state = &*state_ptr;
                let hdc = w_param as Hdc;
                let detail_control = [
                    state.detail_header,
                    state.detail_name,
                    state.detail_kind,
                    state.detail_path,
                ]
                .contains(&(l_param as Hwnd));
                set_text_color(
                    hdc,
                    if detail_control && l_param as Hwnd != state.detail_name {
                        state.palette.muted.colorref()
                    } else {
                        state.palette.text.colorref()
                    },
                );
                set_bk_mode(hdc, TRANSPARENT);
                if detail_control {
                    set_bk_color(hdc, state.palette.surface.colorref());
                    state.surface_brush as Lresult
                } else {
                    set_bk_color(hdc, state.palette.background.colorref());
                    state.background_brush as Lresult
                }
            }
            WM_CTLCOLOREDIT if !state_ptr.is_null() => {
                let state = &*state_ptr;
                let hdc = w_param as Hdc;
                set_text_color(hdc, state.palette.text.colorref());
                set_bk_color(hdc, state.palette.surface.colorref());
                state.surface_brush as Lresult
            }
            WM_CTLCOLORLISTBOX if !state_ptr.is_null() => {
                let state = &*state_ptr;
                let hdc = w_param as Hdc;
                set_text_color(hdc, state.palette.text.colorref());
                if state.resident
                    && state.theme.preset == ThemePreset::Native
                    && supports_modern_frame(state.os_build)
                {
                    set_bk_color(hdc, state.palette.background.colorref());
                    state.background_brush as Lresult
                } else {
                    set_bk_color(hdc, state.palette.surface.colorref());
                    state.surface_brush as Lresult
                }
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
                    && (state_ptr.is_null() || !(*state_ptr).resident)
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
                    for icon in state.shell_icons.values() {
                        if !icon.is_null() {
                            let _ = destroy_icon(*icon);
                        }
                    }
                    state.shell_icons.clear();
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
            wide("Ara").as_ptr(),
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
            "Dosyalar, klasörler ve içerik  •  {}",
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
        // Immediately preceding hidden STATIC siblings give standard Win32
        // EDIT / LISTBOX controls stable UIA/MSAA names without custom COM
        // providers or changing the visible Search flyout layout.
        state.search_label = create_window_ex_w(
            0,
            static_class.as_ptr(),
            wide("Arama sorgusu").as_ptr(),
            WS_CHILD | SS_LEFT | SS_NOPREFIX,
            0,
            0,
            1,
            1,
            hwnd,
            menu_id(ID_SEARCH_ACCESSIBLE_LABEL),
            instance,
            null_mut(),
        );
        state.edit = create_window_ex_w(
            0,
            edit_class.as_ptr(),
            empty.as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL,
            0,
            0,
            100,
            SEARCH_HEIGHT,
            hwnd,
            menu_id(ID_EDIT),
            instance,
            null_mut(),
        );
        state.status = create_window_ex_w(
            0,
            static_class.as_ptr(),
            wide(
                if state.resident
                    && state.theme.preset == ThemePreset::Native
                    && supports_modern_frame(state.os_build)
                {
                    "Dosyalar ve klasörler"
                } else {
                    "Hazır  •  Yerel index  •  Bulut yok"
                },
            )
            .as_ptr(),
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

        // The visual reading order is query -> category chips -> results.
        // Create LISTBOX after the tabs so native Tab/Shift+Tab follows that
        // order. Keep its non-focusable STATIC name immediately before it.
        state.results_label = create_window_ex_w(
            0,
            static_class.as_ptr(),
            wide(&accessible_results_name(0)).as_ptr(),
            WS_CHILD | SS_LEFT | SS_NOPREFIX,
            0,
            0,
            1,
            1,
            hwnd,
            menu_id(ID_RESULTS_ACCESSIBLE_LABEL),
            instance,
            null_mut(),
        );
        state.list = create_window_ex_w(
            0,
            list_class.as_ptr(),
            empty.as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
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
        state.theme_button = create_window_ex_w(
            0,
            button_class.as_ptr(),
            wide("Görünüm").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
            0,
            0,
            84,
            TITLE_HEIGHT,
            hwnd,
            menu_id(ID_THEME),
            instance,
            null_mut(),
        );

        // Win32 child controls expose details to keyboard and screen readers.
        for (id, text, target) in [
            (ID_DETAIL_HEADER, "En iyi eşleşme", &mut state.detail_header),
            (ID_DETAIL_NAME, "", &mut state.detail_name),
            (ID_DETAIL_KIND, "", &mut state.detail_kind),
            (ID_DETAIL_PATH, "", &mut state.detail_path),
        ] {
            *target = create_window_ex_w(
                0,
                static_class.as_ptr(),
                wide(text).as_ptr(),
                WS_CHILD
                    | SS_LEFT
                    | SS_NOPREFIX
                    | if id == ID_DETAIL_PATH {
                        SS_PATHELLIPSIS
                    } else if id == ID_DETAIL_NAME {
                        SS_ENDELLIPSIS
                    } else {
                        0
                    },
                0,
                0,
                100,
                24,
                hwnd,
                menu_id(id),
                instance,
                null_mut(),
            );
        }
        state.detail_open = create_window_ex_w(
            0,
            button_class.as_ptr(),
            wide("Aç").as_ptr(),
            WS_CHILD | WS_TABSTOP | BS_OWNERDRAW,
            0,
            0,
            100,
            36,
            hwnd,
            menu_id(ID_DETAIL_OPEN),
            instance,
            null_mut(),
        );

        if state.title.is_null()
            || state.search_label.is_null()
            || state.results_label.is_null()
            || state.detail_header.is_null()
            || state.detail_name.is_null()
            || state.detail_kind.is_null()
            || state.detail_path.is_null()
            || state.detail_open.is_null()
            || state.subtitle.is_null()
            || state.edit.is_null()
            || state.list.is_null()
            || state.status.is_null()
            || state.theme_button.is_null()
            || state.tabs.iter().any(|hwnd| hwnd.is_null())
        {
            return -1;
        }

        if set_window_subclass(
            state.theme_button,
            Some(theme_button_subclass_proc),
            ID_THEME,
            hwnd as usize,
        ) == 0
            || set_window_subclass(
                state.edit,
                Some(edit_ime_subclass_proc),
                ID_EDIT,
                hwnd as usize,
            ) == 0
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
            state.detail_header,
            state.detail_name,
            state.detail_kind,
            state.detail_path,
            state.detail_open,
        ] {
            send_message_w(control, WM_SETFONT, state.ui_font as Wparam, 1);
        }
        send_message_w(state.title, WM_SETFONT, state.title_font as Wparam, 1);
        send_message_w(state.subtitle, WM_SETFONT, state.small_font as Wparam, 1);
        send_message_w(state.status, WM_SETFONT, state.small_font as Wparam, 1);

        let cue = wide(
            if state.resident
                && state.theme.preset == ThemePreset::Native
                && supports_modern_frame(state.os_build)
            {
                "Aramak için buraya yazın"
            } else {
                "Her şeyi ara — dosya, klasör, uygulama veya içerik"
            },
        );
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
            let _ = set_window_theme(
                hwnd,
                if state.high_contrast {
                    null_mut()
                } else {
                    explorer.as_ptr()
                },
                null_mut(),
            );
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

            let border = if state.high_contrast {
                state.palette.accent.colorref()
            } else if state.theme.preset == ThemePreset::Native {
                0xFFFF_FFFF
            } else {
                state.palette.accent.colorref()
            };
            let _ = dwm_set_window_attribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                (&border as *const u32).cast(),
                std::mem::size_of::<u32>() as u32,
            );
        }

        if supports_system_backdrop(state.os_build) {
            let backdrop = if state.high_contrast {
                DWMSBT_NONE
            } else {
                match state.theme.backdrop {
                    Backdrop::Auto => DWMSBT_AUTO,
                    Backdrop::Mica => DWMSBT_MAINWINDOW,
                    Backdrop::Acrylic => DWMSBT_TRANSIENTWINDOW,
                    Backdrop::None => DWMSBT_NONE,
                }
            };
            let _ = dwm_set_window_attribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                (&backdrop as *const i32).cast(),
                std::mem::size_of::<i32>() as u32,
            );
        }

        let alpha = if state.high_contrast {
            255
        } else {
            state.theme.alpha()
        };
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

    // Optional independent flyout, respecting work-area taskbar placement.
    // This does not replace or reposition Windows-owned SearchHost.exe.
    // Work and monitor rectangles are physical pixels, including negative origins.
    fn taskbar_search_rect(
        work: Rect,
        monitor: Rect,
        logical_width: i32,
        logical_height: i32,
        dpi: u32,
    ) -> Rect {
        let available_width = (work.right - work.left).max(1);
        let available_height = (work.bottom - work.top).max(1);
        let gap = scale_px(12, dpi);
        let width = scale_px(logical_width.clamp(720, 960), dpi)
            .min(available_width.saturating_sub(gap * 2).max(1));
        let height = scale_px(logical_height.clamp(620, 760), dpi)
            .min(available_height.saturating_sub(gap * 2).max(1));

        let top_inset = (work.top - monitor.top).max(0);
        let bottom_inset = (monitor.bottom - work.bottom).max(0);
        let left_inset = (work.left - monitor.left).max(0);
        let right_inset = (monitor.right - work.right).max(0);
        let largest_inset = top_inset.max(bottom_inset).max(left_inset).max(right_inset);
        // No detectable work-area inset: preserve taskbar-at-bottom fallback.
        let (x, y) = if largest_inset > 0 && top_inset == largest_inset {
            (work.left + (available_width - width) / 2, work.top + gap)
        } else if largest_inset > 0 && left_inset == largest_inset {
            (work.left + gap, work.top + (available_height - height) / 2)
        } else if largest_inset > 0 && right_inset == largest_inset {
            (
                work.right - width - gap,
                work.top + (available_height - height) / 2,
            )
        } else {
            (
                work.left + (available_width - width) / 2,
                work.bottom - height - gap,
            )
        };

        let x = x.clamp(work.left, work.right - width);
        let y = y.clamp(work.top, work.bottom - height);
        Rect {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    unsafe fn center_search_window(
        hwnd: Hwnd,
        logical_width: i32,
        logical_height: i32,
        dpi: u32,
        resident: bool,
    ) {
        let layout = monitor_layout(hwnd);
        let work = layout.map(|(work, _)| work).or_else(|| {
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
        let target = if resident {
            taskbar_search_rect(
                work,
                layout.map(|(_, monitor)| monitor).unwrap_or(work),
                logical_width,
                logical_height,
                dpi,
            )
        } else {
            centered_window_rect(work, logical_width, logical_height, dpi)
        };
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

    unsafe fn search_frame_rect(hwnd: Hwnd, state: &State) -> Option<Rect> {
        if state.edit.is_null() {
            return None;
        }
        let mut edit_rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if get_window_rect(state.edit, &mut edit_rect) == 0 {
            return None;
        }
        let mut top_left = Point {
            x: edit_rect.left,
            y: edit_rect.top,
        };
        let mut bottom_right = Point {
            x: edit_rect.right,
            y: edit_rect.bottom,
        };
        if screen_to_client(hwnd, &mut top_left) == 0
            || screen_to_client(hwnd, &mut bottom_right) == 0
        {
            return None;
        }
        let border = scale_px(2, state.dpi).max(1);
        Some(Rect {
            left: top_left.x - border,
            top: top_left.y - border,
            right: bottom_right.x + border,
            bottom: bottom_right.y + border,
        })
    }

    unsafe fn invalidate_search_frame(hwnd: Hwnd, state: &State) {
        if let Some(frame) = search_frame_rect(hwnd, state) {
            let _ = invalidate_rect(hwnd, &frame, 1);
        }
    }

    fn native_result_columns(client: Rect, dpi: u32) -> Option<(Rect, Rect)> {
        let margin = scale_px(24, dpi);
        let gap = scale_px(16, dpi);
        let total = client.right - client.left - margin * 2;
        // Keep a single readable list on compact screens rather than
        // crushing two panes into unusable widths.
        if total < scale_px(710, dpi) {
            return None;
        }
        let left_width = (total - gap) * 52 / 100;
        let top = client.top + scale_px(158, dpi);
        let bottom = client.bottom - margin;
        if bottom - top < scale_px(190, dpi) {
            return None;
        }
        let left = Rect {
            left: client.left + margin,
            top,
            right: client.left + margin + left_width,
            bottom,
        };
        let detail = Rect {
            left: left.right + gap,
            top,
            right: client.right - margin,
            bottom,
        };
        Some((left, detail))
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
        let native_flyout = state.resident
            && state.theme.preset == ThemePreset::Native
            && supports_modern_frame(state.os_build);
        if native_flyout {
            // Windows 11 Search reference: 780x720 flyout, ~32px top search
            // inset, ~36px input and a navigation row directly below it.
            // The results here remain Search Tool results; not native SearchHost.
            show_window(state.title, SW_HIDE);
            show_window(state.subtitle, SW_HIDE);
            let margin = scale_px(24, state.dpi);
            let content_width = (rect.right - rect.left - margin * 2).max(1);
            let search_top = scale_px(32, state.dpi);
            let search_height = scale_px(36, state.dpi);
            let border = scale_px(1, state.dpi).max(1);
            move_window(
                state.edit,
                margin + border,
                search_top + border,
                (content_width - border * 2).max(1),
                (search_height - border * 2).max(1),
                1,
            );
            let tab_y = search_top + search_height + scale_px(12, state.dpi);
            let tab_height = scale_px(34, state.dpi);
            let tab_gap = scale_px(8, state.dpi);
            let appearance_width = scale_px(32, state.dpi).min(content_width);
            let category_space = (content_width - appearance_width - tab_gap).max(1);
            let desired = [64, 100, 116, 88].map(|w| scale_px(w, state.dpi));
            let desired_total = desired.iter().sum::<i32>() + tab_gap * 3;
            let compact_width = ((category_space - tab_gap * 3) / 4).max(1);
            let mut next_x = margin;
            for (index, tab) in state.tabs.iter().enumerate() {
                let tab_width = if desired_total <= category_space {
                    desired[index]
                } else {
                    compact_width
                };
                move_window(*tab, next_x, tab_y, tab_width, tab_height, 1);
                next_x += tab_width + tab_gap;
            }
            move_window(
                state.theme_button,
                (rect.right - margin - appearance_width).max(margin),
                tab_y,
                appearance_width,
                tab_height,
                1,
            );
            let status_y = tab_y + tab_height + scale_px(12, state.dpi);
            let status_height = scale_px(STATUS_HEIGHT, state.dpi);
            move_window(
                state.status,
                margin,
                status_y,
                content_width,
                status_height,
                1,
            );
            let list_y = status_y + status_height + scale_px(8, state.dpi);
            if let Some((left, detail)) = native_result_columns(rect, state.dpi) {
                move_window(
                    state.list,
                    left.left,
                    left.top,
                    left.right - left.left,
                    left.bottom - left.top,
                    1,
                );
                let inset = scale_px(20, state.dpi);
                let x = detail.left + inset;
                let width = (detail.right - x - inset).max(1);
                move_window(
                    state.detail_header,
                    x,
                    detail.top + scale_px(24, state.dpi),
                    width,
                    scale_px(26, state.dpi),
                    1,
                );
                move_window(
                    state.detail_name,
                    x,
                    detail.top + scale_px(78, state.dpi),
                    width,
                    scale_px(34, state.dpi),
                    1,
                );
                move_window(
                    state.detail_kind,
                    x,
                    detail.top + scale_px(121, state.dpi),
                    width,
                    scale_px(28, state.dpi),
                    1,
                );
                move_window(
                    state.detail_path,
                    x,
                    detail.top + scale_px(172, state.dpi),
                    width,
                    scale_px(70, state.dpi),
                    1,
                );
                move_window(
                    state.detail_open,
                    x,
                    detail.bottom - inset - scale_px(40, state.dpi),
                    width,
                    scale_px(40, state.dpi),
                    1,
                );
            } else {
                move_window(
                    state.list,
                    margin,
                    list_y,
                    content_width,
                    (rect.bottom - list_y - margin).max(1),
                    1,
                );
            }
            update_detail_controls(state);
            if state.results.is_empty() {
                show_window(state.list, SW_HIDE);
            }
            return;
        }
        show_window(state.title, SW_SHOW);
        show_window(state.subtitle, SW_SHOW);
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
        let theme_width = scale_px(
            if state.theme.preset == ThemePreset::Native {
                38
            } else {
                108
            },
            state.dpi,
        );

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
        let search_border = scale_px(2, state.dpi).max(1);
        move_window(
            state.edit,
            margin + search_border,
            search_y + search_border,
            (width - search_border * 2).max(1),
            (search_height - search_border * 2).max(1),
            1,
        );

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
        update_detail_controls(state);
    }

    // The native results LISTBOX does not use LBS_SORT: every row is
    // inserted in the same order as State.results. A broken or unreadable
    // item-data mapping must not silently select a different filesystem
    // object. Fail closed for both the detail card and ShellExecute.
    fn verified_selected_result_index(
        selected: isize,
        item_data: isize,
        result_count: usize,
    ) -> Option<usize> {
        let selected = usize::try_from(selected).ok()?;
        let item_data = usize::try_from(item_data).ok()?;
        (selected == item_data && selected < result_count).then_some(selected)
    }

    unsafe fn native_result_count_matches(state: &State) -> bool {
        let native_count = send_message_w(state.list, LB_GETCOUNT, 0, 0);
        usize::try_from(native_count).ok() == Some(state.results.len())
    }

    fn result_accessible_label(row: &ResultRow) -> String {
        format!(
            "{} · {} · {}",
            row.name,
            if row.is_directory { "Klasör" } else { "Dosya" },
            row.path,
        )
    }

    // Malformed index labels must be skipped individually, not allowed to
    // invalidate an otherwise usable query by failing ListBox insertion.
    fn verified_result_accessible_label(row: &ResultRow) -> Option<String> {
        if row.name.contains('\0') || row.path.contains('\0') {
            return None;
        }
        let label = result_accessible_label(row);
        (label.encode_utf16().count() <= MAX_NATIVE_LABEL_U16).then_some(label)
    }

    // Verify what Win32 actually exposes as the selected row text, not just
    // its item-data index. A same-count row replacement must not cause Open
    // to act on a different cached path than the visible/accessibility label.
    unsafe fn native_result_label_matches(list: Hwnd, index: usize, expected: &str) -> bool {
        let Ok(len) = usize::try_from(send_message_w(list, LB_GETTEXTLEN, index, 0)) else {
            return false;
        };
        if len > MAX_NATIVE_LABEL_U16 {
            return false;
        }
        let mut actual = vec![0_u16; len + 1];
        let read = send_message_w(list, LB_GETTEXT, index, actual.as_mut_ptr() as Lparam);
        read == len as isize && actual[..len].iter().copied().eq(expected.encode_utf16())
    }

    unsafe fn selected_detail_row(state: &State) -> Option<&ResultRow> {
        // Reject stale or externally corrupted native rows even when the
        // selected row's item-data happens to match a cached result. A hidden
        // ListBox must not leave actionable details from cached results.
        if get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE == 0
            || !native_result_count_matches(state)
        {
            return None;
        }
        let selected = send_message_w(state.list, LB_GETCURSEL, 0, 0);
        if selected < 0 {
            return None;
        }
        let item_data = send_message_w(state.list, LB_GETITEMDATA, selected as Wparam, 0);
        let index = verified_selected_result_index(selected, item_data, state.results.len())?;
        let row = state.results.get(index)?;
        let label = verified_result_accessible_label(row)?;
        native_result_label_matches(state.list, index, &label).then_some(row)
    }

    // A hidden detail card must never retain a previously selected file path in
    // its Win32 STATIC text: accessibility clients can inspect hidden controls.
    fn detail_content(row: Option<&ResultRow>) -> (&str, &str, &str) {
        match row {
            Some(row) => (
                &row.name,
                if row.is_directory { "Klasör" } else { "Dosya" },
                &row.path,
            ),
            None => ("", "", ""),
        }
    }

    unsafe fn update_detail_controls(state: &State) {
        if state.detail_open.is_null() || state.list.is_null() {
            return;
        }
        let parent = get_parent(state.list);
        let mut client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let native = state.resident
            && state.theme.preset == ThemePreset::Native
            && supports_modern_frame(state.os_build)
            && !parent.is_null()
            && get_client_rect(parent, &mut client) != 0
            && native_result_columns(client, state.dpi).is_some();
        let row = if native {
            selected_detail_row(state)
        } else {
            None
        };
        let (name, kind, path) = detail_content(row);
        set_window_text_w(state.detail_name, wide(name).as_ptr());
        set_window_text_w(state.detail_kind, wide(kind).as_ptr());
        set_window_text_w(state.detail_path, wide(path).as_ptr());
        for control in [
            state.detail_header,
            state.detail_name,
            state.detail_kind,
            state.detail_path,
            state.detail_open,
        ] {
            show_window(control, if row.is_some() { SW_SHOW } else { SW_HIDE });
        }
        if native && !parent.is_null() {
            invalidate_rect(parent, null_mut(), 1);
        }
    }

    fn accessible_filter_name(label: &str, selected: bool) -> String {
        if selected {
            format!("{label} (seçili)")
        } else {
            label.to_string()
        }
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
            // Owner-drawn chips paint their own label and underline, so the
            // HWND title describes selection for MSAA/screen readers.
            // The native BUTTON caption is not automatically announced as
            // changed to external WinEvent listeners when it is renamed.
            let hwnd = state.tabs[index];
            if hwnd.is_null() {
                continue;
            }
            let name = accessible_filter_name(label, state.mode == mode);
            let old_name = read_control_text_for_test(hwnd);
            if old_name == name {
                continue;
            }
            set_window_text_w(hwnd, wide(&name).as_ptr());
            let parent = get_parent(hwnd);
            if !parent.is_null()
                && should_notify_result_name_change(
                    &old_name,
                    &name,
                    is_window_visible(parent) != 0,
                )
            {
                notify_win_event(EVENT_OBJECT_NAMECHANGE, hwnd, OBJID_CLIENT, 0);
            }
        }
    }

    // Query Shell icons by type only; indexed/synthetic paths are never opened.
    fn shell_icon_key(name: &str, directory: bool) -> String {
        if directory {
            return "folder".to_string();
        }
        Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .filter(|ext| !ext.is_empty())
            .map(|ext| {
                format!(
                    ".{}",
                    ext.chars().take(24).collect::<String>().to_lowercase()
                )
            })
            .unwrap_or_else(|| "file".to_string())
    }

    unsafe fn cache_result_shell_icon(state: &mut State, row: &ResultRow) {
        let key = shell_icon_key(&row.name, row.is_directory);
        if state.shell_icons.contains_key(&key) || state.shell_icons.len() >= MAX_SHELL_ICON_TYPES {
            return;
        }
        let mut info: ShFileInfoW = std::mem::zeroed();
        let probe = wide(if row.is_directory { "folder" } else { &key });
        let attributes = if row.is_directory {
            FILE_ATTRIBUTE_DIRECTORY
        } else {
            FILE_ATTRIBUTE_NORMAL
        };
        let status = sh_get_file_info_w(
            probe.as_ptr(),
            attributes,
            &mut info,
            std::mem::size_of::<ShFileInfoW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON | SHGFI_USEFILEATTRIBUTES,
        );
        state
            .shell_icons
            .insert(key, if status != 0 { info.icon } else { null_mut() });
    }

    fn should_restore_query_focus(parent_visible: bool, list_focused: bool) -> bool {
        parent_visible && list_focused
    }

    // Keep the native result list visible while synchronous query rebuilding
    // is in progress. Only hide it when there are genuinely no results.
    // Never give focus to a hidden self-test window or an inactive popup.
    unsafe fn update_native_result_visibility(state: &State, has_results: bool) {
        let parent = get_parent(state.list);
        if has_results {
            show_window(state.list, SW_SHOW);
        } else {
            if !parent.is_null()
                && should_restore_query_focus(
                    is_window_visible(parent) != 0,
                    get_focus() == state.list,
                )
            {
                set_focus(state.edit);
            }
            show_window(state.list, SW_HIDE);
        }
        if !parent.is_null() {
            invalidate_rect(parent, null_mut(), 1);
        }
    }

    // Keep the same selected filesystem object after a query/category refresh,
    // but never carry a file selection onto a directory (or the reverse) if
    // an index update reused that path. Display names are not unique.
    fn refreshed_selection_index(
        rows: &[ResultRow],
        previous: Option<(&str, bool)>,
    ) -> Option<usize> {
        if rows.is_empty() {
            return None;
        }
        Some(
            previous
                .and_then(|(path, was_directory)| {
                    rows.iter()
                        .position(|row| row.path == path && row.is_directory == was_directory)
                })
                .unwrap_or(0),
        )
    }

    // Windows resolves dot components before opening. Refuse paths with
    // unresolved dot segments so a lexical scope prefix cannot conceal a
    // traversal outside the search folder.
    fn has_dot_path_segment(path: &str) -> bool {
        path.split(['\\', '/'])
            .any(|segment| segment == "." || segment == "..")
    }

    // Search indexes can contain orphaned or malformed entries. A failed
    // parent-chain reconstruction must never fabricate a root-level filename
    // that could open a different real file via ShellExecute.
    fn verified_result_path(reconstructed: io::Result<String>) -> Option<String> {
        let path = reconstructed.ok()?;
        let bytes = path.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || !matches!(bytes[2], b'\\' | b'/')
            || path.contains('\0')
            || has_dot_path_segment(&path)
        {
            return None;
        }
        Some(path)
    }

    // Native unsorted ListBox rows and State.results must have a
    // one-to-one, verified mapping from their initial insertion. Do not
    // accept a row whose Win32 item-data write/read failed or was reordered.
    unsafe fn insert_verified_result_label(list: Hwnd, label: &str, expected_index: usize) -> bool {
        let text = wide(label);
        let added = send_message_w(list, LB_ADDSTRING, 0, text.as_ptr() as Lparam);
        if added < 0 {
            return false;
        }
        let consistent = usize::try_from(added).ok() == Some(expected_index)
            && send_message_w(
                list,
                LB_SETITEMDATA,
                added as Wparam,
                expected_index as Lparam,
            ) >= 0
            && send_message_w(list, LB_GETITEMDATA, added as Wparam, 0) == expected_index as isize
            && native_result_label_matches(list, expected_index, label);
        if !consistent {
            // Best-effort rollback here; the caller clears the entire list
            // and result cache on failure, including if deletion fails.
            send_message_w(list, LB_DELETESTRING, added as Wparam, 0);
        }
        consistent
    }

    unsafe fn refresh_results(state: &mut State) {
        let native = state.resident
            && state.theme.preset == ThemePreset::Native
            && supports_modern_frame(state.os_build);
        let previous_selection = if native {
            selected_detail_row(state).map(|row| (row.path.clone(), row.is_directory))
        } else {
            None
        };
        send_message_w(state.list, LB_RESETCONTENT, 0, 0);
        state.results.clear();
        // Hide stale details before any early return on empty/invalid input.
        update_detail_controls(state);
        // No visibility toggle yet: the query is handled synchronously and
        // repaint happens after the results are known, avoiding list flicker.
        let len = get_window_text_length_w(state.edit).clamp(0, MAX_QUERY_U16);
        if len == 0 {
            set_idle_status(state);
            if native {
                update_native_result_visibility(state, false);
            }
            return;
        }
        let mut buffer = vec![0_u16; len as usize + 1];
        let copied = get_window_text_w(state.edit, buffer.as_mut_ptr(), len + 1);
        if copied <= 0 {
            if native {
                update_native_result_visibility(state, false);
            }
            return;
        }
        let query = String::from_utf16_lossy(&buffer[..copied as usize]);
        let query = query.trim();
        if query.is_empty() {
            set_idle_status(state);
            if native {
                update_native_result_visibility(state, false);
            }
            return;
        }

        let explicit_path_filter = match state.mode {
            SearchMode::Content => None,
            _ => parse_gui_search_query(query).filters.path_contains,
        };

        let started = Instant::now();
        let hits = search_for_mode(state, query);
        let elapsed = started.elapsed();
        let Ok(hits) = hits else {
            set_status(state, "Arama geçici olarak kullanılamıyor");
            if native {
                update_native_result_visibility(state, false);
            }
            return;
        };

        let mut insertion_failed = false;
        for hit in hits {
            // Fail closed: a result without a verified absolute path is not
            // actionable and must not appear in the Open-ready list.
            let Some(path) = verified_result_path(state.store.reconstruct_path(&hit, 256)) else {
                continue;
            };
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
            // LBS_HASSTRINGS supplies this text to MSAA/UIA while the
            // visual owner-drawn renderer continues to use ResultRow.
            // Include type + full result path to disambiguate same-name files.
            let Some(label) = verified_result_accessible_label(&row) else {
                continue;
            };
            if !insert_verified_result_label(state.list, &label, state.results.len()) {
                insertion_failed = true;
                break;
            }
            cache_result_shell_icon(state, &row);
            state.results.push(row);
            if state.results.len() >= MAX_RESULTS {
                break;
            }
        }
        if insertion_failed {
            // If any native row cannot be associated with the correct
            // result, do not expose a partially populated actionable list.
            send_message_w(state.list, LB_RESETCONTENT, 0, 0);
            state.results.clear();
            update_detail_controls(state);
            set_status(state, "Sonuç listesi güvenli biçimde oluşturulamadı");
            if native {
                update_native_result_visibility(state, false);
            }
            return;
        }

        let count = state.results.len();
        let timing = elapsed.as_secs_f64() * 1000.0;
        let status = if native && count == 0 {
            "Sonuç bulunamadı".to_string()
        } else {
            match state.scope.as_deref() {
                Some(scope) => format!("{count} sonuç  •  {timing:.1} ms  •  {scope}"),
                None => format!("{count} sonuç  •  {timing:.1} ms"),
            }
        };
        set_status(state, &status);
        if native {
            if let Some(index) = refreshed_selection_index(
                &state.results,
                previous_selection
                    .as_ref()
                    .map(|(path, directory)| (path.as_str(), *directory)),
            ) {
                send_message_w(state.list, LB_SETCURSEL, index, 0);
            }
        }
        if native {
            // selected_detail_row rejects hidden lists. Restore ListBox
            // visibility before rebuilding the selected result detail card,
            // rather than relying on incidental WM_SHOWWINDOW notifications.
            update_native_result_visibility(state, count > 0);
        }
        update_detail_controls(state);
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
                let mut parsed = parse_gui_search_query(query);
                apply_scope_filter(&mut parsed, state.scope.as_deref());
                parsed.filters.item_type = Some(match state.mode {
                    SearchMode::Files => ItemTypeFilter::File,
                    SearchMode::Folders => ItemTypeFilter::Directory,
                    _ => unreachable!(),
                });
                state.store.search_filtered(&parsed, scoped_limit, 100_000)
            }
            SearchMode::All => {
                let mut parsed = parse_gui_search_query(query);
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

    fn accessible_results_name(count: usize) -> String {
        format!("Arama sonuçları ({count} sonuç)")
    }

    fn should_notify_result_name_change(
        old_name: &str,
        new_name: &str,
        popup_visible: bool,
    ) -> bool {
        popup_visible && old_name != new_name
    }

    unsafe fn set_status(state: &State, value: &str) {
        let value = wide(value);
        set_window_text_w(state.status, value.as_ptr());
        if !state.results_label.is_null() {
            // Standard native LISTBOX gets its MSAA name from the preceding
            // non-focusable STATIC. Changing its text alone may not signal an
            // accessibility name change to a listening client.
            let name = accessible_results_name(state.results.len());
            let old_name = read_control_text_for_test(state.results_label);
            if old_name != name {
                set_window_text_w(state.results_label, wide(&name).as_ptr());
                let parent = get_parent(state.list);
                if !state.list.is_null()
                    && !parent.is_null()
                    && should_notify_result_name_change(
                        &old_name,
                        &name,
                        is_window_visible(parent) != 0,
                    )
                {
                    notify_win_event(EVENT_OBJECT_NAMECHANGE, state.list, OBJID_CLIENT, 0);
                }
            }
        }
    }

    unsafe fn set_idle_status(state: &State) {
        match state.scope.as_deref() {
            Some(scope) => set_status(state, &format!("Bu konumda ara  •  {scope}")),
            None => set_status(
                state,
                if state.resident
                    && state.theme.preset == ThemePreset::Native
                    && supports_modern_frame(state.os_build)
                {
                    "Aramak için yazın"
                } else {
                    "Hazır  •  Yerel index  •  Bulut yok  •  Alt+Space"
                },
            ),
        }
    }

    unsafe fn measure_theme_menu_item(state: &State, measure: &mut MeasureItemStruct) -> bool {
        if measure.ctl_type != ODT_MENU || measure.item_data == 0 {
            return false;
        }
        let visual = &*(measure.item_data as *const MenuItemVisual);
        let chars = visual.label.len().saturating_sub(1).min(56) as i32;
        let chrome = if visual.has_submenu { 78 } else { 58 };
        let logical_width = (chars.saturating_mul(7) + chrome).clamp(220, 470);
        measure.item_width = scale_px(logical_width, state.dpi).max(1) as u32;
        measure.item_height = scale_px(22, state.dpi).max(20) as u32;
        true
    }

    unsafe fn draw_theme_menu_item(state: &State, draw: &DrawItemStruct) {
        if draw.ctl_type != ODT_MENU || draw.item_data == 0 {
            return;
        }
        let visual = &*(draw.item_data as *const MenuItemVisual);
        let selected = draw.item_state & ODS_SELECTED != 0;
        let checked = draw.item_state & ODS_CHECKED != 0;
        fill_rect(
            draw.hdc,
            &draw.rc_item,
            if selected {
                state.accent_brush
            } else {
                state.surface_brush
            },
        );
        set_bk_mode(draw.hdc, TRANSPARENT);
        set_text_color(
            draw.hdc,
            if selected {
                state.palette.selected_text.colorref()
            } else {
                state.palette.text.colorref()
            },
        );
        let old_font = select_object(draw.hdc, state.small_font as Hgdiobj);
        let check_width = scale_px(32, state.dpi);
        if checked {
            let check = wide("✓");
            let mut check_rect = Rect {
                left: draw.rc_item.left + scale_px(4, state.dpi),
                top: draw.rc_item.top,
                right: draw.rc_item.left + check_width,
                bottom: draw.rc_item.bottom,
            };
            draw_text_w(
                draw.hdc,
                check.as_ptr(),
                -1,
                &mut check_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
        let mut text_rect = Rect {
            left: draw.rc_item.left + check_width + scale_px(4, state.dpi),
            top: draw.rc_item.top,
            right: draw.rc_item.right
                - scale_px(if visual.has_submenu { 34 } else { 12 }, state.dpi),
            bottom: draw.rc_item.bottom,
        };
        draw_text_w(
            draw.hdc,
            visual.label.as_ptr(),
            -1,
            &mut text_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        if visual.has_submenu {
            let arrow = wide("›");
            let mut arrow_rect = Rect {
                left: draw.rc_item.right - scale_px(30, state.dpi),
                top: draw.rc_item.top,
                right: draw.rc_item.right - scale_px(6, state.dpi),
                bottom: draw.rc_item.bottom,
            };
            draw_text_w(
                draw.hdc,
                arrow.as_ptr(),
                -1,
                &mut arrow_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
        if !old_font.is_null() {
            select_object(draw.hdc, old_font);
        }
    }

    unsafe fn draw_theme_button(state: &State, draw: &DrawItemStruct) {
        let pressed = draw.item_state & ODS_SELECTED != 0;
        let focused = draw.item_state & ODS_FOCUS != 0;
        let hot = state.theme_button_hot || draw.item_state & ODS_HOTLIGHT != 0;
        fill_rect(draw.hdc, &draw.rc_item, state.background_brush);

        let inset = scale_px(1, state.dpi).max(1);
        let mut frame = Rect {
            left: draw.rc_item.left + inset,
            top: draw.rc_item.top + inset,
            right: draw.rc_item.right - inset,
            bottom: draw.rc_item.bottom - inset,
        };
        if frame.right <= frame.left {
            frame.right = frame.left + 1;
        }
        if frame.bottom <= frame.top {
            frame.bottom = frame.top + 1;
        }

        // Use the configured accent as the border so this control follows every
        // preset/custom accent instead of falling back to the stock Win32 button.
        fill_rect(
            draw.hdc,
            &frame,
            if state.high_contrast {
                state.accent_brush
            } else if state.theme.preset == ThemePreset::Native {
                state.surface_brush
            } else {
                state.accent_brush
            },
        );
        if !pressed {
            let border = scale_px(if hot || focused { 2 } else { 1 }, state.dpi).max(1);
            let inner = Rect {
                left: frame.left + border,
                top: frame.top + border,
                right: frame.right - border,
                bottom: frame.bottom - border,
            };
            if inner.right > inner.left && inner.bottom > inner.top {
                fill_rect(draw.hdc, &inner, state.surface_brush);
                frame = inner;
            }
        }

        set_bk_mode(draw.hdc, TRANSPARENT);
        set_text_color(
            draw.hdc,
            if pressed {
                state.palette.selected_text.colorref()
            } else if state.high_contrast {
                state.palette.text.colorref()
            } else {
                state.palette.accent.colorref()
            },
        );
        let old_font = select_object(draw.hdc, state.small_font as Hgdiobj);
        let text = wide(if state.theme.preset == ThemePreset::Native {
            "⋯"
        } else {
            "Görünüm"
        });
        let mut text_rect = frame;
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
        if focused {
            let focus_inset = scale_px(4, state.dpi).max(2);
            let focus = Rect {
                left: draw.rc_item.left + focus_inset,
                top: draw.rc_item.top + focus_inset,
                right: draw.rc_item.right - focus_inset,
                bottom: draw.rc_item.bottom - focus_inset,
            };
            if focus.right > focus.left && focus.bottom > focus.top {
                let _ = draw_focus_rect(draw.hdc, &focus);
            }
        }
    }

    // Windows 11-style rounded GDI surface without extra brushes or pens.
    unsafe fn fill_rounded_surface(hdc: Hdc, rect: Rect, brush: Hbrush, radius: i32) {
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return;
        }
        const NULL_PEN: i32 = 8;
        let old_brush = select_object(hdc, brush as Hgdiobj);
        let old_pen = select_object(hdc, get_stock_object(NULL_PEN));
        let diameter = radius.max(1).saturating_mul(2);
        round_rect(
            hdc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            diameter,
            diameter,
        );
        if !old_pen.is_null() {
            select_object(hdc, old_pen);
        }
        if !old_brush.is_null() {
            select_object(hdc, old_brush);
        }
    }

    unsafe fn draw_detail_open_button(state: &State, draw: &DrawItemStruct) {
        let pressed = draw.item_state & ODS_SELECTED != 0;
        let focused = draw.item_state & ODS_FOCUS != 0;
        fill_rect(draw.hdc, &draw.rc_item, state.surface_brush);
        let inset = scale_px(1, state.dpi).max(1);
        let rect = Rect {
            left: draw.rc_item.left + inset,
            top: draw.rc_item.top + inset,
            right: draw.rc_item.right - inset,
            bottom: draw.rc_item.bottom - inset,
        };
        fill_rounded_surface(draw.hdc, rect, state.accent_brush, scale_px(8, state.dpi));
        set_text_color(draw.hdc, state.palette.selected_text.colorref());
        set_bk_mode(draw.hdc, TRANSPARENT);
        let old_font = select_object(draw.hdc, state.ui_font as Hgdiobj);
        let mut text_rect = rect;
        let label = wide("Aç");
        draw_text_w(
            draw.hdc,
            label.as_ptr(),
            -1,
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        if !old_font.is_null() {
            select_object(draw.hdc, old_font);
        }
        if focused && !pressed {
            let focus = Rect {
                left: rect.left + inset * 3,
                top: rect.top + inset * 3,
                right: rect.right - inset * 3,
                bottom: rect.bottom - inset * 3,
            };
            draw_focus_rect(draw.hdc, &focus);
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

        let native = state.resident
            && state.theme.preset == ThemePreset::Native
            && supports_modern_frame(state.os_build);
        if native {
            fill_rounded_surface(
                draw.hdc,
                chip,
                if active && state.high_contrast {
                    state.accent_brush
                } else if active {
                    state.surface_brush
                } else {
                    state.background_brush
                },
                scale_px(9, state.dpi),
            );
            if active {
                let underline = Rect {
                    left: chip.left + scale_px(12, state.dpi),
                    top: chip.bottom - scale_px(2, state.dpi).max(1),
                    right: chip.right - scale_px(12, state.dpi),
                    bottom: chip.bottom,
                };
                if underline.right > underline.left {
                    fill_rect(draw.hdc, &underline, state.accent_brush);
                }
            }
        } else {
            fill_rect(
                draw.hdc,
                &chip,
                if active {
                    state.accent_brush
                } else {
                    state.surface_brush
                },
            );
        }
        set_bk_mode(draw.hdc, TRANSPARENT);
        set_text_color(
            draw.hdc,
            if active && (!native || state.high_contrast) {
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
            if selected && (state.theme.preset != ThemePreset::Native || state.high_contrast) {
                state.accent_brush
            } else if selected {
                state.surface_brush
            } else if state.theme.preset == ThemePreset::Native {
                state.background_brush
            } else {
                state.surface_brush
            },
        );

        if !selected && state.theme.preset != ThemePreset::Native && !state.high_contrast {
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
        let title_color =
            if selected && (state.theme.preset != ThemePreset::Native || state.high_contrast) {
                state.palette.selected_text
            } else {
                state.palette.text
            };
        set_text_color(draw.hdc, title_color.colorref());

        let icon_key = shell_icon_key(&row.name, row.is_directory);
        if !state.high_contrast {
            if let Some(&icon) = state.shell_icons.get(&icon_key) {
                if !icon.is_null() {
                    let icon_size = scale_px(16, state.dpi);
                    let _ = draw_icon_ex(
                        draw.hdc,
                        card.left + scale_px(13, state.dpi),
                        card.top + scale_px(10, state.dpi),
                        icon,
                        icon_size,
                        icon_size,
                        0,
                        null_mut(),
                        DI_NORMAL,
                    );
                }
            }
        }
        let title = wide(&row.name);
        let badge_width = scale_px(86, state.dpi);
        let mut title_rect = Rect {
            left: card.left + scale_px(if state.high_contrast { 14 } else { 36 }, state.dpi),
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
        let path_color =
            if selected && (state.theme.preset != ThemePreset::Native || state.high_contrast) {
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
            left: card.left + scale_px(if state.high_contrast { 14 } else { 36 }, state.dpi),
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
        // Owner-drawn ListBox rows paint focus explicitly for keyboard users.
        if draw.item_state & ODS_FOCUS != 0 {
            let inset = scale_px(2, state.dpi).max(1);
            let focus = Rect {
                left: card.left + inset,
                top: card.top + inset,
                right: card.right - inset,
                bottom: card.bottom - inset,
            };
            if focus.right > focus.left && focus.bottom > focus.top {
                let _ = draw_focus_rect(draw.hdc, &focus);
            }
        }
    }

    fn popup_shortcuts_allowed(focused_edit: bool, ime_composing: bool) -> bool {
        !focused_edit || !ime_composing
    }

    fn should_handle_dialog_tab(message: u32, key: Wparam) -> bool {
        // IsDialogMessageW would also redirect Return, Escape and arrow keys;
        // those retain Search Tool's existing query and result semantics.
        message == WM_KEYDOWN && key == VK_TAB as Wparam
    }

    fn should_route_result_enter(focused: Hwnd, edit: Hwnd, list: Hwnd) -> bool {
        focused == edit || focused == list
    }

    fn query_down_target(current: isize, result_count: usize) -> Option<usize> {
        if result_count == 0 {
            return None;
        }
        if current >= 0 && (current as usize) < result_count {
            Some(current as usize)
        } else {
            Some(0)
        }
    }

    // A rejected keyboard selection must not remain selected. Otherwise a
    // later Enter sees LB_GETCURSEL >= 0 and cannot retry Best match even
    // after the corrupt row mapping has recovered.
    unsafe fn reject_native_keyboard_selection(state: &State) -> bool {
        send_message_w(state.list, LB_SETCURSEL, Wparam::MAX, 0);
        update_detail_controls(state);
        false
    }

    // Update native LISTBOX selection, but never move focus here. The outer
    // GetMessage loop owns keyboard focus; synthetic hidden tests can safely
    // exercise the same selection logic without touching the user's desktop.
    unsafe fn prepare_query_down_selection(state: &State) -> bool {
        // Avoid focusing a control that a responsive layout has hidden,
        // even if its previous in-memory results have not been cleared.
        if get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE == 0
            || !native_result_count_matches(state)
        {
            return reject_native_keyboard_selection(state);
        }
        let selected = send_message_w(state.list, LB_GETCURSEL, 0, 0);
        let Some(index) = query_down_target(selected, state.results.len()) else {
            return reject_native_keyboard_selection(state);
        };
        if selected != index as isize {
            if send_message_w(state.list, LB_SETCURSEL, index, 0) < 0 {
                return reject_native_keyboard_selection(state);
            }
            update_detail_controls(state);
        }
        // Do not move keyboard focus into a row with corrupt item-data or
        // native text. Also clear the failed selection for a future retry.
        if selected_detail_row(state).is_none() {
            return reject_native_keyboard_selection(state);
        }
        true
    }

    fn should_select_best_match(focused_edit: bool, selected: isize, count: usize) -> bool {
        focused_edit && selected < 0 && count > 0
    }

    // Both normal EDIT Enter and the out-of-process Shell bridge Enter use
    // this selection path. Never ShellExecute here: hidden Win32 tests can
    // assert the identical preparation logic without opening any real file.
    unsafe fn prepare_search_enter_selection(state: &State, from_query: bool) -> bool {
        if get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE == 0
            || !native_result_count_matches(state)
        {
            return reject_native_keyboard_selection(state);
        }
        let selected = send_message_w(state.list, LB_GETCURSEL, 0, 0);
        if selected >= 0 && selected_detail_row(state).is_none() {
            // A previously selected row can become corrupt without a native
            // selection-change notification. Remove it before declining Enter
            // so the stale detail path and selection cannot linger.
            return reject_native_keyboard_selection(state);
        }
        if !should_select_best_match(from_query, selected, state.results.len()) {
            return false;
        }
        if send_message_w(state.list, LB_SETCURSEL, 0, 0) < 0 {
            return reject_native_keyboard_selection(state);
        }
        update_detail_controls(state);
        // Enter must never announce a Best match from corrupt native data.
        // Roll back the just-selected row so Enter can retry after recovery.
        if selected_detail_row(state).is_none() {
            return reject_native_keyboard_selection(state);
        }
        true
    }

    // An indexed path can become stale between search and a user pressing
    // Open. Recheck both existence and expected file/directory kind directly
    // before ShellExecute. Never launch a different object type at that path.
    fn selected_path_still_openable(path: &str, expected_directory: bool) -> bool {
        std::fs::metadata(path).is_ok_and(|metadata| {
            if expected_directory {
                metadata.is_dir()
            } else {
                metadata.is_file()
            }
        })
    }

    // A lexical path inside scope can traverse a junction or symlink into a
    // different directory. Check the resolved filesystem target just before
    // Open; fail closed if either scope or target can no longer be resolved.
    // Unscoped searches retain their existing behavior.
    fn selected_path_within_scope(path: &str, scope: Option<&str>) -> bool {
        let Some(scope) = scope else {
            return true;
        };
        if !path_is_within_scope(path, scope) {
            return false;
        }
        let (Ok(target), Ok(root)) = (std::fs::canonicalize(path), std::fs::canonicalize(scope))
        else {
            return false;
        };
        path_is_within_scope(&target.to_string_lossy(), &root.to_string_lossy())
    }

    unsafe fn open_selected(hwnd: Hwnd, state: &mut State) -> bool {
        let Some(row) = selected_detail_row(state) else {
            // The same fail-closed mapping check drives both detail and Open.
            // A silent native row change may leave both its selected index
            // and old detail card intact. Revoke the selection as well as
            // clearing details so double-click cannot retain an invalid row.
            return reject_native_keyboard_selection(state);
        };
        if !selected_path_still_openable(&row.path, row.is_directory) {
            // Do not ShellExecute a deleted item or a path whose type no
            // longer matches the index. Clear the now-unactionable selection
            // and detail card rather than leaving a misleading Open button.
            set_status(state, "Seçili sonuç artık mevcut değil veya türü değişti");
            return reject_native_keyboard_selection(state);
        }
        if !selected_path_within_scope(&row.path, state.scope.as_deref()) {
            // A stale index entry or reparse point must not open outside an
            // Explorer-scoped search. Clear the unusable selected result.
            set_status(
                state,
                "Seçilen sonuç arama konumu dışında veya konum doğrulanamıyor",
            );
            return reject_native_keyboard_selection(state);
        }
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

    // Same-process, synchronous Win32 regression; no SendInput, focus stealing,
    // user-desktop capture, production index or simulated IME acceptance.
    unsafe fn list_accessible_text_for_test(list: Hwnd, index: usize) -> String {
        let len = send_message_w(list, LB_GETTEXTLEN, index, 0);
        if len < 0 {
            return String::new();
        }
        let mut buffer = vec![0_u16; len as usize + 1];
        let read = send_message_w(list, LB_GETTEXT, index, buffer.as_mut_ptr() as Lparam);
        if read < 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..read as usize])
    }

    unsafe fn read_control_text_for_test(control: Hwnd) -> String {
        let len = get_window_text_length_w(control).max(0);
        let mut chars = vec![0_u16; len as usize + 1];
        let copied = get_window_text_w(control, chars.as_mut_ptr(), len + 1).max(0);
        String::from_utf16_lossy(&chars[..copied as usize])
    }

    unsafe fn require_ui_selftest(condition: bool, reason: &str) -> io::Result<()> {
        if condition {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "hidden UI regression failed: {reason}"
            )))
        }
    }

    // Hidden Win32 EDIT controls do not reliably generate automatic EN_CHANGE
    // for every SetWindowTextW mutation. Deliver the documented WM_COMMAND
    // notification deterministically to test the exact application handler.
    unsafe fn drive_hidden_edit_change(
        hwnd: Hwnd,
        state_ptr: *mut State,
        query: &str,
    ) -> io::Result<()> {
        require_ui_selftest(
            set_window_text_w((*state_ptr).edit, wide(query).as_ptr()) != 0,
            "SetWindowTextW failed",
        )?;
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).edit) == query,
            "Edit text mismatch after SetWindowTextW",
        )?;
        let _ = send_message_w(
            hwnd,
            WM_COMMAND,
            ID_EDIT | (EN_CHANGE << 16),
            (*state_ptr).edit as Lparam,
        );
        Ok(())
    }

    unsafe fn run_hidden_ui_regression(hwnd: Hwnd, state_ptr: *mut State) -> io::Result<()> {
        require_ui_selftest(is_window_visible(hwnd) == 0, "parent must stay hidden")?;
        require_ui_selftest(
            (*state_ptr).resident
                && (*state_ptr).theme.preset == ThemePreset::Native
                && supports_modern_frame((*state_ptr).os_build),
            "expected Windows 11 native resident mode",
        )?;
        let mut bounds = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        require_ui_selftest(
            get_client_rect(hwnd, &mut bounds) != 0
                && native_result_columns(bounds, (*state_ptr).dpi).is_some(),
            "native two-column layout unavailable",
        )?;

        require_ui_selftest(
            read_control_text_for_test((*state_ptr).search_label) == "Arama sorgusu"
                && read_control_text_for_test((*state_ptr).results_label)
                    == accessible_results_name(0)
                && get_window_long_ptr_w((*state_ptr).search_label, GWL_STYLE) as u32
                    & (WS_VISIBLE | WS_TABSTOP)
                    == 0
                && get_window_long_ptr_w((*state_ptr).results_label, GWL_STYLE) as u32
                    & (WS_VISIBLE | WS_TABSTOP)
                    == 0,
            "accessible search/result labels must exist and remain non-focusable",
        )?;
        // Even with no results, the query -> category chips traversal must
        // remain stable and skip the invisible ListBox in either direction.
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_next_dlg_tab_item(hwnd, (*state_ptr).edit, 0) == (*state_ptr).tabs[0]
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[0], 1) == (*state_ptr).edit
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0)
                    == (*state_ptr).theme_button,
            "empty-query Tab navigation must pass through categories and skip results",
        )?;

        // Exercise real native row insertion with a deliberately incorrect
        // expected slot. A mismatch must roll back, leaving no actionable
        // ListBox rows before the synthetic query populates the fixture.
        require_ui_selftest(
            !insert_verified_result_label((*state_ptr).list, "synthetic rejected row", 1)
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 0,
            "failed ListBox insertion left an unmatched native result",
        )?;
        require_ui_selftest(
            !insert_verified_result_label((*state_ptr).list, "synthetic\0truncated row", 0)
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 0,
            "NUL-truncated native label was not safely rolled back",
        )?;
        require_ui_selftest(
            insert_verified_result_label((*state_ptr).list, "synthetic valid row", 0)
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 1
                && send_message_w((*state_ptr).list, LB_GETITEMDATA, 0, 0) == 0,
            "verified native ListBox insertion failed",
        )?;
        send_message_w((*state_ptr).list, LB_RESETCONTENT, 0, 0);

        // Exercise the native EDIT and the production WM_COMMAND handler.
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).results_label) == accessible_results_name(3),
            "initial three search results did not update accessibility count",
        )?;
        // Exercise the actual EDIT -> filtered index -> native ListBox path
        // for Windows-valid forward slashes in user-entered path:/in: tokens.
        // Synthetic paths are C:\\Users\\Demo; no real files are opened.
        for query in [
            r#"SearchTool path:"C:\Users\Demo""#,
            r#"SearchTool path:"C:/Users/Demo""#,
            r#"SearchTool in:C:/Users/Demo"#,
        ] {
            drive_hidden_edit_change(hwnd, state_ptr, query)?;
            require_ui_selftest(
                (*state_ptr).results.len() == 3
                    && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                    && read_control_text_for_test((*state_ptr).results_label)
                        == accessible_results_name(3),
                &format!(
                    "forward-slash path filter lost synthetic Windows index results: query={query:?}, count={}, status={:?}",
                    (*state_ptr).results.len(),
                    read_control_text_for_test((*state_ptr).status),
                ),
            )?;
        }
        drive_hidden_edit_change(hwnd, state_ptr, r#"SearchTool path:C:/Users/Nonexistent"#)?;
        require_ui_selftest(
            (*state_ptr).results.is_empty()
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 0,
            "forward-slash path filter included results outside its directory",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            (*state_ptr).results.len() == 3
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3,
            "forward-slash path filter fixture did not restore ordinary search",
        )?;
        let next = get_next_dlg_tab_item(hwnd, (*state_ptr).edit, 0);
        let edit_style = get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32;
        let list_style = get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32;
        require_ui_selftest(
            edit_style & WS_TABSTOP != 0
                && list_style & (WS_VISIBLE | WS_TABSTOP) == WS_VISIBLE | WS_TABSTOP
                && next == (*state_ptr).tabs[0]
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[0], 1)
                    == (*state_ptr).edit
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0)
                    == (*state_ptr).list
                && get_next_dlg_tab_item(hwnd, (*state_ptr).list, 1)
                    == (*state_ptr).tabs[3],
            &format!("native visual Tab order: next={next:?}, first_tab={:?}, list_style={list_style:#x}, edit_style={edit_style:#x}",
                (*state_ptr).tabs[0]),
        )?;
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).tabs[0]) == "Tümü (seçili)"
                && read_control_text_for_test((*state_ptr).tabs[1]) == "Dosyalar"
                && read_control_text_for_test((*state_ptr).tabs[2]) == "Klasörler"
                && read_control_text_for_test((*state_ptr).tabs[3]) == "İçerik"
                && read_control_text_for_test((*state_ptr).theme_button) == "Görünüm"
                && read_control_text_for_test((*state_ptr).detail_open) == "Aç",
            "owner-drawn controls must expose readable accessible labels",
        )?;
        // Exercise the real WM_COMMAND category handler without clicking or
        // shifting desktop focus; verify that selection state follows mode.
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_FILES | (BN_CLICKED << 16),
            (*state_ptr).tabs[1] as Lparam,
        );
        require_ui_selftest(
            (*state_ptr).mode == SearchMode::Files
                && read_control_text_for_test((*state_ptr).tabs[0]) == "Tümü"
                && read_control_text_for_test((*state_ptr).tabs[1]) == "Dosyalar (seçili)",
            "category switch did not update MSAA selected-state label",
        )?;
        // A repeated click on the active category is a true no-op. The
        // marker must survive instead of being replaced by a fresh query
        // duration/status from refresh_results. No real file is opened.
        let files_count = (*state_ptr).results.len();
        let selected_files_row = send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0);
        let files_detail = read_control_text_for_test((*state_ptr).detail_path);
        let noop_marker = "Kategori yeniden seçimi aramayı tekrar çalıştırmamalı";
        set_status(&*state_ptr, noop_marker);
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_FILES | (BN_CLICKED << 16),
            (*state_ptr).tabs[1] as Lparam,
        );
        require_ui_selftest(
            (*state_ptr).mode == SearchMode::Files
                && (*state_ptr).results.len() == files_count
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == selected_files_row
                && read_control_text_for_test((*state_ptr).detail_path) == files_detail
                && read_control_text_for_test((*state_ptr).status) == noop_marker
                && read_control_text_for_test((*state_ptr).tabs[1]) == "Dosyalar (seçili)",
            "reselecting active Files category needlessly refreshed results",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_ALL | (BN_CLICKED << 16),
            (*state_ptr).tabs[0] as Lparam,
        );
        require_ui_selftest(
            (*state_ptr).mode == SearchMode::All
                && read_control_text_for_test((*state_ptr).tabs[0]) == "Tümü (seçili)"
                && (*state_ptr).results.len() == 3,
            "returning to all results did not restore accessible state",
        )?;
        // Match the native focusable control order after result repopulation.
        // This verifies Windows' dialog manager candidate selection rather than
        // synthesizing physical Tab/Shift+Tab keystrokes.
        require_ui_selftest(
            get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[0], 0) == (*state_ptr).tabs[1]
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[1], 0) == (*state_ptr).tabs[2]
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[2], 0) == (*state_ptr).tabs[3]
                && get_next_dlg_tab_item(hwnd, (*state_ptr).list, 0) == (*state_ptr).theme_button
                && get_next_dlg_tab_item(hwnd, (*state_ptr).theme_button, 1) == (*state_ptr).list
                && get_next_dlg_tab_item(hwnd, (*state_ptr).theme_button, 0)
                    == (*state_ptr).detail_open,
            "category, ListBox, theme and Open button Tab order mismatch",
        )?;
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).edit) == "SearchTool",
            "Edit did not retain the entered query",
        )?;
        require_ui_selftest(
            (*state_ptr).results.len() == 3
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3,
            "expected three indexed synthetic results after EN_CHANGE",
        )?;
        // Simulate a system color-change notification without changing any
        // global Windows accessibility setting. Preserve query/selection.
        let _ = send_message_w(hwnd, WM_SYSCOLORCHANGE, 0, 0);
        require_ui_selftest(
            (*state_ptr).high_contrast == system_high_contrast_enabled()
                && (*state_ptr).palette
                    == resolve_palette(
                        &(*state_ptr).theme,
                        (*state_ptr).dark,
                        (*state_ptr).high_contrast,
                    )
                && (*state_ptr).results.len() == 3,
            "system color change did not refresh accessibility palette",
        )?;
        let first_row = &(&(*state_ptr).results)[0];
        let label = list_accessible_text_for_test((*state_ptr).list, 0);
        require_ui_selftest(
            label.contains(&first_row.name)
                && label.contains(&first_row.path)
                && label.contains(if first_row.is_directory {
                    "Klasör"
                } else {
                    "Dosya"
                }),
            "owner-drawn ListBox accessibility item missing name/type/path",
        )?;
        let initial = first_row.name.clone();
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).detail_name) == initial,
            "first result did not populate native detail",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "Open button hidden while a result is selected",
        )?;

        // Programmatic LB_SETCURSEL does not emit a selection notification.
        // Deliver the same WM_COMMAND notification as a real ListBox selection.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETCURSEL, 1, 0) >= 0,
            "could not select the second synthetic result",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_LIST | (LBN_SELCHANGE << 16),
            (*state_ptr).list as Lparam,
        );
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).detail_name) == (&(*state_ptr).results)[1].name
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[1].path,
            "selection change did not update name and full path",
        )?;

        // Rebuild the same query through the real EDIT/WM_COMMAND path. The
        // selected path must survive instead of snapping back to best match.
        let selected_path = (&(*state_ptr).results)[1].path.clone();
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 1
                && read_control_text_for_test((*state_ptr).detail_path) == selected_path
                && (&(*state_ptr).results)[1].path == selected_path,
            "requery lost the selected result although its full path survived",
        )?;

        // The keyboard Down handler calls the same helper before SetFocus.
        // This test never sends physical keys and never steals focus.
        require_ui_selftest(
            prepare_query_down_selection(&*state_ptr)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 1
                && read_control_text_for_test((*state_ptr).detail_path) == selected_path,
            "Down from query reset an existing result selection",
        )?;
        // A freshly unselected list must not expose stale result details.
        let _ = send_message_w((*state_ptr).list, LB_SETCURSEL, usize::MAX, 0);
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && prepare_query_down_selection(&*state_ptr)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path,
            "Down from an unselected query failed to select best match",
        )?;
        // Enter must preserve a later explicit selection, not reset it to
        // the best match. No ShellExecute runs in this synthetic regression.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETCURSEL, 1, 0) >= 0,
            "could not select a later result before Enter",
        )?;
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            !prepare_search_enter_selection(&*state_ptr, true)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 1
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[1].path,
            "Enter replaced the user's second selected result",
        )?;
        let _ = send_message_w((*state_ptr).list, LB_SETCURSEL, usize::MAX, 0);
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            !prepare_search_enter_selection(&*state_ptr, false)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty(),
            "Enter outside the query must not select a hidden Best match",
        )?;
        require_ui_selftest(
            prepare_search_enter_selection(&*state_ptr, true)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path,
            "Shell-bridge Enter did not select the first result before opening",
        )?;
        // An already selected, subsequently corrupted row must also be
        // cleared by Enter, without a separate selection notification.
        let old_selected_path = read_control_text_for_test((*state_ptr).detail_path);
        require_ui_selftest(
            old_selected_path == (&(*state_ptr).results)[0].path
                && send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 1) >= 0,
            "could not set up stale selected Enter fixture",
        )?;
        require_ui_selftest(
            !prepare_search_enter_selection(&*state_ptr, true)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0,
            "Enter retained a stale previously selected native result",
        )?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 0) >= 0
                && prepare_search_enter_selection(&*state_ptr, true)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0,
            "Enter failed to recover after stale selected item-data repair",
        )?;
        // An Enter-created Best match with invalid item-data must be
        // deselected, not left stuck as an unusable selected row. Restoring
        // item-data must allow another Enter without refreshing the index.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETCURSEL, Wparam::MAX, 0) < 0
                && send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 1) >= 0,
            "could not prepare corrupt Best match item-data",
        )?;
        require_ui_selftest(
            !prepare_search_enter_selection(&*state_ptr, true)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty(),
            "Enter left an invalid Best match selected after rejection",
        )?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 0) >= 0
                && prepare_search_enter_selection(&*state_ptr, true)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0,
            "Enter could not reselect Best match after mapping recovery",
        )?;
        // Corrupt only the synthetic ListBox row-to-result mapping. An
        // unreadable, out-of-range or wrong-but-valid mapping must not
        // expose the wrong detail or call ShellExecute. Restore each time.
        for wrong_mapping in [-1_isize, 999_isize, 1_isize] {
            require_ui_selftest(
                send_message_w(
                    (*state_ptr).list,
                    LB_SETITEMDATA,
                    0,
                    wrong_mapping as Lparam,
                ) >= 0
                    && send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0,
                "could not corrupt synthetic ListBox item data",
            )?;
            update_detail_controls(&*state_ptr);
            require_ui_selftest(
                send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                    && selected_detail_row(&*state_ptr).is_none()
                    && !prepare_query_down_selection(&*state_ptr)
                    && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                    && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                    && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32
                        & WS_VISIBLE
                        == 0
                    && !open_selected(hwnd, &mut *state_ptr),
                "invalid item-data mapping exposed or opened a different result",
            )?;
        }
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 0) >= 0
                && send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0,
            "could not restore synthetic ListBox mapping",
        )?;
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            selected_detail_row(&*state_ptr).is_some()
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    != 0,
            "valid item-data mapping did not restore safe details",
        )?;
        // A native selection-change notification must also roll back a
        // corrupted row, not merely hide its detail card while retaining the
        // bogus ListBox selection visible to keyboard/accessibility clients.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 1) >= 0,
            "could not prepare corrupted native selection-change fixture",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_LIST | (LBN_SELCHANGE << 16),
            (*state_ptr).list as Lparam,
        );
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0,
            "corrupted native selection notification left a selected result",
        )?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 0) >= 0
                && send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0,
            "could not restore native selection after corrupted notification",
        )?;
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            selected_detail_row(&*state_ptr).is_some()
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path,
            "valid native selection did not recover after rejected notification",
        )?;

        // Replacing a row's displayed label at the same slot and restoring
        // matching item-data must still fail closed: the native text no longer
        // describes the file the Rust cache would otherwise open. Do not
        // send a selection notification: Open must clear any stale detail.
        let previous_detail_path = read_control_text_for_test((*state_ptr).detail_path);
        require_ui_selftest(
            previous_detail_path == (&(*state_ptr).results)[0].path,
            "native row corruption fixture has no prior visible detail",
        )?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_DELETESTRING, 0, 0) == 2
                && send_message_w(
                    (*state_ptr).list,
                    LB_INSERTSTRING,
                    0,
                    wide("synthetic substituted native result").as_ptr() as Lparam,
                ) == 0
                && send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 0) >= 0
                && send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0,
            "could not substitute synthetic native row label",
        )?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && send_message_w((*state_ptr).list, LB_GETITEMDATA, 0, 0) == 0
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                && selected_detail_row(&*state_ptr).is_none()
                && read_control_text_for_test((*state_ptr).detail_path) == previous_detail_path,
            "substituted native row fixture did not retain an invalid selection",
        )?;
        // Dispatch the same notification as a ListBox double click. The
        // handler must clear the old selection, rather than only hiding Open.
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_LIST | (LBN_DBLCLK << 16),
            (*state_ptr).list as Lparam,
        );
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && !prepare_query_down_selection(&*state_ptr),
            "invalid double-click left a native selection or stale Open details",
        )?;
        refresh_results(&mut *state_ptr);
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && selected_detail_row(&*state_ptr).is_some()
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path,
            "native result label substitution did not recover after refresh",
        )?;

        // A spurious native row must invalidate even an otherwise correct
        // selected item-data mapping, then recover after removal. It must
        // clear a previously valid selected detail even without LBN_SELCHANGE.
        let prior_path_before_count_mismatch = read_control_text_for_test((*state_ptr).detail_path);
        require_ui_selftest(
            prior_path_before_count_mismatch == (&(*state_ptr).results)[0].path,
            "count mismatch fixture has no previously selected detail",
        )?;
        require_ui_selftest(
            send_message_w(
                (*state_ptr).list,
                LB_ADDSTRING,
                0,
                wide("synthetic extra row").as_ptr() as Lparam,
            ) == 3,
            "could not append unmatched native ListBox row",
        )?;
        // Do not explicitly update the stale detail before Down: prove the
        // real keyboard handler removes it after the count mismatch.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                && selected_detail_row(&*state_ptr).is_none()
                && read_control_text_for_test((*state_ptr).detail_path)
                    == prior_path_before_count_mismatch
                && !prepare_query_down_selection(&*state_ptr)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && !open_selected(hwnd, &mut *state_ptr),
            "Down retained a stale selected result after count mismatch",
        )?;
        require_ui_selftest(
            !prepare_search_enter_selection(&*state_ptr, true)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0,
            "Enter selected a Best match from an inconsistent native list",
        )?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_DELETESTRING, 3, 0) == 3,
            "could not remove unmatched native ListBox row",
        )?;
        send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0);
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            selected_detail_row(&*state_ptr).is_some() && prepare_query_down_selection(&*state_ptr),
            "native ListBox count recovery failed",
        )?;

        // A missing native row must also fail closed, including when the
        // selected row still has a valid index and item-data. Requery must
        // rebuild a usable list without carrying the corruption forward.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_DELETESTRING, 2, 0) == 2,
            "could not remove synthetic native result row",
        )?;
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            selected_detail_row(&*state_ptr).is_none()
                && !prepare_query_down_selection(&*state_ptr)
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && !open_selected(hwnd, &mut *state_ptr),
            "missing native row exposed, focused or opened a cached result",
        )?;
        refresh_results(&mut *state_ptr);
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && (*state_ptr).results.len() == 3
                && send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0
                && selected_detail_row(&*state_ptr).is_some(),
            "query refresh did not recover after a missing native row",
        )?;

        // The test only prepares a selection, never ShellExecute or SendInput.
        // The same shortcut must not target a hidden LISTBOX with cached
        // results. The parent and the entire self-test remain hidden.
        show_window((*state_ptr).list, SW_HIDE);
        let hidden_status_marker = "Hidden ListBox must not open cached results";
        set_status(&*state_ptr, hidden_status_marker);
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            !prepare_query_down_selection(&*state_ptr)
                && selected_detail_row(&*state_ptr).is_none()
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && !open_selected(hwnd, &mut *state_ptr)
                && read_control_text_for_test((*state_ptr).status) == hidden_status_marker,
            "hidden ListBox exposed or opened cached results",
        )?;
        send_message_w((*state_ptr).list, LB_SETCURSEL, Wparam::MAX, 0);
        require_ui_selftest(
            !prepare_search_enter_selection(&*state_ptr, true)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0,
            "Enter selected Best match in a hidden ListBox",
        )?;
        show_window((*state_ptr).list, SW_SHOW);
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0
                && selected_detail_row(&*state_ptr).is_some(),
            "visible ListBox did not recover normal selection",
        )?;
        update_detail_controls(&*state_ptr);

        for query in ["", "  ", "SearchToolNoMatchZZZ"] {
            drive_hidden_edit_change(hwnd, state_ptr, query)?;
            require_ui_selftest(
                !prepare_search_enter_selection(&*state_ptr, true),
                "Enter selected a result when the query has no matches",
            )?;
            require_ui_selftest(
                read_control_text_for_test((*state_ptr).results_label)
                    == accessible_results_name(0),
                "empty/no-match result count did not update accessible label",
            )?;
            let rows = send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0);
            let name = read_control_text_for_test((*state_ptr).detail_name);
            let kind = read_control_text_for_test((*state_ptr).detail_kind);
            let path = read_control_text_for_test((*state_ptr).detail_path);
            let visible =
                get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE != 0;
            require_ui_selftest(
                !prepare_query_down_selection(&*state_ptr),
                "Down from query must not select an empty result list",
            )?;
            require_ui_selftest(
                (*state_ptr).results.is_empty() && rows == 0
                    && name.is_empty() && kind.is_empty() && path.is_empty() && !visible
                    && list_accessible_text_for_test((*state_ptr).list, 0).is_empty(),
                &format!(
                    "query={query:?}, edit={:?}, results={}, list_count={rows}, name={name:?}, kind={kind:?}, path={path:?}, button_visible={visible}",
                    read_control_text_for_test((*state_ptr).edit),
                    (*state_ptr).results.len(),
                ),
            )?;
            require_ui_selftest(
                get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0
                    && get_next_dlg_tab_item(hwnd, (*state_ptr).edit, 0) == (*state_ptr).tabs[0]
                    && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0)
                        == (*state_ptr).theme_button,
                "cleared/no-match result list must skip ListBox without skipping categories",
            )?;
        }

        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).results_label) == accessible_results_name(3),
            "repopulated results did not restore accessibility count",
        )?;
        require_ui_selftest(
            (*state_ptr).results.len() == 3
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                && selected_detail_row(&*state_ptr).is_some()
                && read_control_text_for_test((*state_ptr).detail_name)
                    == (&(*state_ptr).results)[0].name
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    != 0,
            "query repopulation did not restore selected path, details and Open",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_next_dlg_tab_item(hwnd, (*state_ptr).edit, 0) == (*state_ptr).tabs[0]
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0) == (*state_ptr).list,
            "repopulated results did not restore ListBox Tab order",
        )?;
        // Synthetic IME messages through the native EDIT subclass exercise
        // composition-boundary bookkeeping without installing an IME, typing,
        // showing a window or claiming real candidate-selection acceptance.
        require_ui_selftest(
            !(*state_ptr).ime_composing,
            "IME composition state leaked before boundary regression",
        )?;
        let _ = send_message_w((*state_ptr).edit, WM_IME_STARTCOMPOSITION, 0, 0);
        require_ui_selftest(
            (*state_ptr).ime_composing,
            "EDIT subclass missed WM_IME_STARTCOMPOSITION",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchToolNoMatchZZZ")?;
        require_ui_selftest(
            (*state_ptr).results.len() == 3
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3,
            "EN_CHANGE must defer partial IME search until composition ends",
        )?;
        let _ = send_message_w((*state_ptr).edit, WM_IME_ENDCOMPOSITION, 0, 0);
        require_ui_selftest(
            !(*state_ptr).ime_composing
                && (*state_ptr).results.is_empty()
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 0
                && read_control_text_for_test((*state_ptr).detail_name).is_empty(),
            "IME end did not clear guard and refresh committed search results",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            (*state_ptr).results.len() == 3 && !(*state_ptr).ime_composing,
            "IME search refresh did not restore normal input behavior",
        )?;
        // Some IME sessions terminate on focus loss without a separate
        // END notification. Do not leave global Search hotkeys suppressed.
        let _ = send_message_w((*state_ptr).edit, WM_IME_STARTCOMPOSITION, 0, 0);
        drive_hidden_edit_change(hwnd, state_ptr, "SearchToolNoMatchZZZ")?;
        require_ui_selftest(
            (*state_ptr).ime_composing && (*state_ptr).results.len() == 3,
            "IME focus-loss precondition failed",
        )?;
        let _ = send_message_w((*state_ptr).edit, WM_KILLFOCUS, 0, 0);
        require_ui_selftest(
            !(*state_ptr).ime_composing && (*state_ptr).results.is_empty(),
            "EDIT focus loss must release IME guard and refresh query",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            !(*state_ptr).ime_composing && (*state_ptr).results.len() == 3,
            "normal search failed after IME focus-loss cleanup",
        )?;

        // Programmatic SetWindowTextW can deliver EN_CHANGE before set_query's
        // explicit refresh. Verify that the notification is ignored only
        // during the update and the final committed query still refreshes.
        let suppressed_marker = "Programmatic EN_CHANGE must not requery";
        set_status(&*state_ptr, suppressed_marker);
        (*state_ptr).programmatic_edit_update = true;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchToolNoMatchZZZ")?;
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).status) == suppressed_marker
                && (*state_ptr).results.len() == 3
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3,
            "programmatic EN_CHANGE unexpectedly refreshed search results",
        )?;
        (*state_ptr).programmatic_edit_update = false;
        set_query(&mut *state_ptr, "SearchTool");
        require_ui_selftest(
            !(*state_ptr).programmatic_edit_update
                && read_control_text_for_test((*state_ptr).edit) == "SearchTool"
                && (*state_ptr).results.len() == 3
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3,
            "programmatic query did not restore normal result refresh",
        )?;

        // Exercise the real Open handler on an intentionally absent synthetic
        // file. The preflight must reject it before calling ShellExecute,
        // without opening anything, stealing focus or touching live indexes.
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        let nonexistent = env::temp_dir().join(format!(
            "search-tool-nonexistent-open-{}-{nonce}.txt",
            std::process::id()
        ));
        let nonexistent = nonexistent.to_string_lossy().into_owned();
        require_ui_selftest(
            !Path::new(&nonexistent).exists()
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0,
            "missing-file Open preflight fixture invalid",
        )?;
        let original_path = (&(*state_ptr).results)[0].path.clone();
        (&mut (*state_ptr).results)[0].path = nonexistent;
        // Keep the synthetic visible ListBox label aligned with the missing
        // cached path, so this specifically reaches the filesystem preflight
        // instead of being rejected by the native-label integrity guard.
        let missing_label = result_accessible_label(&(&(*state_ptr).results)[0]);
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_DELETESTRING, 0, 0) == 2
                && send_message_w(
                    (*state_ptr).list,
                    LB_INSERTSTRING,
                    0,
                    wide(&missing_label).as_ptr() as Lparam,
                ) == 0
                && send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 0) >= 0
                && send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0,
            "missing-file preflight could not synchronize native fixture label",
        )?;
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            selected_detail_row(&*state_ptr).is_some(),
            "missing-file preflight fixture did not pass label verification",
        )?;
        let launched = open_selected(hwnd, &mut *state_ptr);
        (&mut (*state_ptr).results)[0].path = original_path.clone();
        require_ui_selftest(
            !launched
                && read_control_text_for_test((*state_ptr).status)
                    == "Seçili sonuç artık mevcut değil veya türü değişti"
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                && read_control_text_for_test((*state_ptr).detail_name).is_empty()
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && is_window_visible(hwnd) == 0,
            "Open must reject vanished result and clear its stale detail/Open action",
        )?;
        // An existing file which is outside the selected Explorer scope
        // must reach the resolved scope preflight, refuse ShellExecute and
        // clear the stale detail card. Only isolated temporary files are used.
        let scope_fixture_root = env::temp_dir().join(format!(
            "search-tool-outside-scope-open-{}-{nonce}",
            std::process::id()
        ));
        let valid_file_dir = scope_fixture_root.join("Outside");
        let allowed_scope = scope_fixture_root.join("Restricted");
        std::fs::create_dir_all(&valid_file_dir)?;
        std::fs::create_dir_all(&allowed_scope)?;
        let outside_file = valid_file_dir.join("existing.txt");
        std::fs::write(&outside_file, b"isolated-scope-open-preflight")?;
        let previous_scope = (*state_ptr).scope.clone();
        (*state_ptr).scope = Some(allowed_scope.to_string_lossy().into_owned());
        (&mut (*state_ptr).results)[0].path = outside_file.to_string_lossy().into_owned();
        let out_of_scope_label = result_accessible_label(&(&(*state_ptr).results)[0]);
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_DELETESTRING, 0, 0) == 2
                && send_message_w(
                    (*state_ptr).list,
                    LB_INSERTSTRING,
                    0,
                    wide(&out_of_scope_label).as_ptr() as Lparam,
                ) == 0
                && send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 0) >= 0
                && send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0,
            "outside-scope preflight could not synchronize native result label",
        )?;
        update_detail_controls(&*state_ptr);
        require_ui_selftest(
            selected_detail_row(&*state_ptr).is_some()
                && selected_path_still_openable(&(&(*state_ptr).results)[0].path, false)
                && !selected_path_within_scope(
                    &(&(*state_ptr).results)[0].path,
                    (*state_ptr).scope.as_deref(),
                )
                && read_control_text_for_test((*state_ptr).detail_path)
                    == outside_file.to_string_lossy(),
            "outside-scope Open fixture failed to reach resolved scope preflight",
        )?;
        let out_of_scope_launched = open_selected(hwnd, &mut *state_ptr);
        require_ui_selftest(
            !out_of_scope_launched
                && read_control_text_for_test((*state_ptr).status).contains("arama konumu")
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && is_window_visible(hwnd) == 0,
            "outside-scope Open retained stale selection or launched a real file",
        )?;
        (*state_ptr).scope = previous_scope;
        (&mut (*state_ptr).results)[0].path = original_path;
        std::fs::remove_dir_all(scope_fixture_root)?;

        // Restore the native label as well as the cache before the external
        // cross-process MSAA/WinEvent fixture inspects the completed window.
        refresh_results(&mut *state_ptr);
        require_ui_selftest(
            selected_detail_row(&*state_ptr).is_some()
                && list_accessible_text_for_test((*state_ptr).list, 0)
                    == result_accessible_label(&(&(*state_ptr).results)[0]),
            "missing-file fixture did not restore the native result label",
        )?;

        require_ui_selftest(
            is_window_visible(hwnd) == 0,
            "self-test unexpectedly displayed its window",
        )?;
        Ok(())
    }

    unsafe fn set_query(state: &mut State, query: &str) {
        // SetWindowTextW may synchronously deliver EN_CHANGE. Suppress only
        // that redundant notification, then refresh once with the updated
        // scope and query. Normal typing and committed IME changes still use
        // the ordinary EN_CHANGE handler.
        state.programmatic_edit_update = true;
        let query = wide(query);
        set_window_text_w(state.edit, query.as_ptr());
        state.programmatic_edit_update = false;
        refresh_results(state);
    }

    unsafe fn apply_search_request(state: &mut State, request: SearchRequest) {
        state.scope = request.scope.and_then(normalize_scope);
        set_query(state, request.query.as_deref().unwrap_or(""));
    }

    fn clear_theme_menu_visuals() {
        if let Ok(mut visuals) = THEME_MENU_VISUALS.lock() {
            visuals.clear();
        }
    }

    unsafe fn apply_menu_surface(menu: Hmenu, state: &State) {
        if menu.is_null() {
            return;
        }
        let menu_info = MenuInfo {
            cb_size: std::mem::size_of::<MenuInfo>() as u32,
            mask: MIM_BACKGROUND,
            style: 0,
            max_height: 0,
            background: state.surface_brush,
            context_help_id: 0,
            menu_data: 0,
        };
        let _ = set_menu_info(menu, &menu_info);
    }

    unsafe fn create_themed_popup(state: &State) -> Hmenu {
        let menu = create_popup_menu();
        apply_menu_surface(menu, state);
        menu
    }

    unsafe fn attach_menu_visual(menu: Hmenu, label: Vec<u16>, has_submenu: bool) {
        let position = get_menu_item_count(menu) - 1;
        if position < 0 {
            return;
        }
        let Ok(mut visuals) = THEME_MENU_VISUALS.lock() else {
            return;
        };
        let mut visual = Box::new(MenuItemVisual { label, has_submenu });
        let item_data = (&*visual as *const MenuItemVisual) as usize;
        let info = MenuItemInfoW {
            cb_size: std::mem::size_of::<MenuItemInfoW>() as u32,
            mask: MIIM_FTYPE | MIIM_STRING | MIIM_DATA,
            item_type: MFT_OWNERDRAW,
            state: 0,
            id: 0,
            submenu: null_mut(),
            checked_bitmap: null_mut(),
            unchecked_bitmap: null_mut(),
            item_data,
            type_data: visual.label.as_mut_ptr(),
            text_len: visual.label.len().saturating_sub(1) as u32,
            item_bitmap: null_mut(),
        };
        visuals.push(visual);
        let _ = set_menu_item_info_w(menu, position as u32, 1, &info);
    }

    unsafe fn append_menu_item(menu: Hmenu, id: usize, label: &str, checked: bool) {
        let label = wide(label);
        let flags = MF_STRING | if checked { MF_CHECKED } else { 0 };
        if append_menu_w(menu, flags, id, label.as_ptr()) != 0 {
            attach_menu_visual(menu, label, false);
        }
    }

    unsafe fn append_menu_submenu(menu: Hmenu, submenu: Hmenu, label: &str) {
        let label = wide(label);
        if append_menu_w(menu, MF_STRING | MF_POPUP, submenu as usize, label.as_ptr()) != 0 {
            attach_menu_visual(menu, label, true);
        } else if !submenu.is_null() {
            let _ = destroy_menu(submenu);
        }
    }

    unsafe fn append_menu_separator(menu: Hmenu) {
        let _ = append_menu_w(menu, MF_SEPARATOR, 0, null_mut());
    }

    unsafe fn show_theme_menu(hwnd: Hwnd, state: &mut State) {
        clear_theme_menu_visuals();
        let menu = create_themed_popup(state);
        let preset_menu = create_themed_popup(state);
        let theme_menu = create_themed_popup(state);
        let backdrop_menu = create_themed_popup(state);
        let opacity_menu = create_themed_popup(state);
        let color_menu = create_themed_popup(state);
        let density_menu = create_themed_popup(state);
        let size_menu = create_themed_popup(state);
        let background_menu = create_themed_popup(state);

        let menu_handles = [
            menu,
            preset_menu,
            theme_menu,
            backdrop_menu,
            opacity_menu,
            color_menu,
            density_menu,
            size_menu,
            background_menu,
        ];
        if menu_handles.iter().any(|&handle| handle.is_null()) {
            for handle in menu_handles {
                if !handle.is_null() {
                    let _ = destroy_menu(handle);
                }
            }
            set_status(state, "Görünüm menüsü açılamadı");
            return;
        }

        append_menu_item(
            preset_menu,
            CMD_PRESET_SIGNATURE,
            "Search Tool Signature",
            state.theme.preset == ThemePreset::Signature,
        );
        append_menu_item(
            preset_menu,
            CMD_PRESET_MIDNIGHT,
            "Midnight",
            state.theme.preset == ThemePreset::Midnight,
        );
        append_menu_item(
            preset_menu,
            CMD_PRESET_GRAPHITE,
            "Graphite",
            state.theme.preset == ThemePreset::Graphite,
        );
        append_menu_item(
            preset_menu,
            CMD_PRESET_FROST,
            "Frost",
            state.theme.preset == ThemePreset::Frost,
        );
        append_menu_item(
            preset_menu,
            CMD_PRESET_NATIVE,
            "Windows Native",
            state.theme.preset == ThemePreset::Native,
        );

        append_menu_item(
            theme_menu,
            CMD_THEME_SYSTEM,
            "Sistem",
            state.theme.mode == ThemeMode::System,
        );
        append_menu_item(
            theme_menu,
            CMD_THEME_DARK,
            "Koyu",
            state.theme.mode == ThemeMode::Dark,
        );
        append_menu_item(
            theme_menu,
            CMD_THEME_LIGHT,
            "Açık",
            state.theme.mode == ThemeMode::Light,
        );

        append_menu_item(
            backdrop_menu,
            CMD_BACKDROP_AUTO,
            "Otomatik",
            state.theme.backdrop == Backdrop::Auto,
        );
        append_menu_item(
            backdrop_menu,
            CMD_BACKDROP_ACRYLIC,
            "Acrylic / Win10 fallback",
            state.theme.backdrop == Backdrop::Acrylic,
        );
        append_menu_item(
            backdrop_menu,
            CMD_BACKDROP_MICA,
            "Mica (Windows 11)",
            state.theme.backdrop == Backdrop::Mica,
        );
        append_menu_item(
            backdrop_menu,
            CMD_BACKDROP_NONE,
            "Düz renk",
            state.theme.backdrop == Backdrop::None,
        );

        for (id, opacity) in [
            (CMD_OPACITY_60, 60_u8),
            (CMD_OPACITY_75, 75),
            (CMD_OPACITY_90, 90),
            (CMD_OPACITY_100, 100),
        ] {
            append_menu_item(
                opacity_menu,
                id,
                &format!("%{opacity}"),
                state.theme.opacity_percent == opacity,
            );
        }

        append_menu_item(color_menu, CMD_ACCENT, "Vurgu...", false);
        append_menu_item(color_menu, CMD_BACKGROUND_COLOR, "Arka plan...", false);
        append_menu_item(color_menu, CMD_SURFACE_COLOR, "Kart / yüzey...", false);
        append_menu_item(color_menu, CMD_TEXT_COLOR, "Ana yazı...", false);
        append_menu_item(color_menu, CMD_MUTED_COLOR, "İkincil yazı...", false);
        append_menu_item(
            color_menu,
            CMD_RESET_PALETTE,
            "Renkleri preset'e döndür",
            false,
        );

        append_menu_item(
            density_menu,
            CMD_DENSITY_COMPACT,
            "Compact",
            state.theme.density == Density::Compact,
        );
        append_menu_item(
            density_menu,
            CMD_DENSITY_COMFORTABLE,
            "Comfortable",
            state.theme.density == Density::Comfortable,
        );
        append_menu_item(
            density_menu,
            CMD_DENSITY_SPACIOUS,
            "Spacious",
            state.theme.density == Density::Spacious,
        );

        append_menu_item(
            size_menu,
            CMD_SIZE_COMPACT,
            "Compact 760×540",
            state.theme.width == 760 && state.theme.height == 540,
        );
        append_menu_item(
            size_menu,
            CMD_SIZE_STANDARD,
            "Standard 900×640",
            state.theme.width == 900 && state.theme.height == 640,
        );
        append_menu_item(
            size_menu,
            CMD_SIZE_WIDE,
            "Wide 1120×720",
            state.theme.width == 1120 && state.theme.height == 720,
        );

        append_menu_item(
            background_menu,
            CMD_BACKGROUND_IMAGE,
            "Arka plan resmi seç...",
            false,
        );
        append_menu_item(
            background_menu,
            CMD_BACKGROUND_IMAGE_CLEAR,
            "Arka plan resmini kaldır",
            state.theme.background_image.is_none(),
        );
        append_menu_separator(background_menu);
        append_menu_item(
            background_menu,
            CMD_BACKGROUND_FIT_FILL,
            "Yerleşim: Fill",
            state.theme.background_fit == BackgroundFit::Fill,
        );
        append_menu_item(
            background_menu,
            CMD_BACKGROUND_FIT_FIT,
            "Yerleşim: Fit",
            state.theme.background_fit == BackgroundFit::Fit,
        );
        append_menu_item(
            background_menu,
            CMD_BACKGROUND_FIT_STRETCH,
            "Yerleşim: Stretch",
            state.theme.background_fit == BackgroundFit::Stretch,
        );
        append_menu_separator(background_menu);
        for (id, opacity) in [
            (CMD_BACKGROUND_IMAGE_OPACITY_20, 20_u8),
            (CMD_BACKGROUND_IMAGE_OPACITY_35, 35),
            (CMD_BACKGROUND_IMAGE_OPACITY_60, 60),
            (CMD_BACKGROUND_IMAGE_OPACITY_100, 100),
        ] {
            append_menu_item(
                background_menu,
                id,
                &format!("Resim opaklığı: %{opacity}"),
                state.theme.background_image_opacity == opacity,
            );
        }

        append_menu_submenu(menu, preset_menu, "Preset");
        append_menu_submenu(menu, theme_menu, "Tema");
        append_menu_submenu(menu, backdrop_menu, "Efekt");
        append_menu_submenu(menu, opacity_menu, "Pencere saydamlığı");
        append_menu_submenu(menu, color_menu, "Renkler");
        append_menu_submenu(menu, density_menu, "Sonuç yoğunluğu");
        append_menu_submenu(menu, size_menu, "Panel boyutu");
        append_menu_submenu(menu, background_menu, "Arka plan");
        append_menu_separator(menu);
        append_menu_item(
            menu,
            CMD_DEFAULT_APPS,
            "Windows varsayılan arama ayarları...",
            false,
        );
        append_menu_item(menu, CMD_ADVANCED_THEME, "Tema dosyasını aç...", false);

        let mut button_rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut point = Point { x: 0, y: 0 };
        if get_window_rect(state.theme_button, &mut button_rect) != 0 {
            point.x = button_rect.left;
            point.y = button_rect.bottom + scale_px(4, state.dpi);
        } else if get_cursor_pos(&mut point) == 0 {
            let _ = destroy_menu(menu);
            clear_theme_menu_visuals();
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
        clear_theme_menu_visuals();

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
                center_search_window(
                    hwnd,
                    state.theme.width,
                    state.theme.height,
                    state.dpi,
                    state.resident,
                );
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
        let new_high_contrast = system_high_contrast_enabled();
        let new_palette = resolve_palette(&state.theme, state.dark, new_high_contrast);
        // Create replacement brushes before touching live HBRUSH handles.
        let new_brushes = [
            create_solid_brush(new_palette.background.colorref()),
            create_solid_brush(new_palette.surface.colorref()),
            create_solid_brush(new_palette.accent.colorref()),
            create_solid_brush(new_palette.muted.colorref()),
        ];
        if new_brushes.iter().any(|brush| brush.is_null()) {
            for brush in new_brushes {
                if !brush.is_null() {
                    let _ = delete_object(brush as Hgdiobj);
                }
            }
            return;
        }
        let old_brushes = [
            state.background_brush,
            state.surface_brush,
            state.accent_brush,
            state.muted_brush,
        ];
        state.palette = new_palette;
        state.high_contrast = new_high_contrast;
        state.background_brush = new_brushes[0];
        state.surface_brush = new_brushes[1];
        state.accent_brush = new_brushes[2];
        state.muted_brush = new_brushes[3];
        // WNDCLASSEX stores the original class brush as a handle. Replace it
        // before releasing the old brush to avoid dangling class resources.
        let _ = set_class_long_ptr_w(hwnd, GCLP_HBRBACKGROUND, state.background_brush as isize);
        for brush in old_brushes {
            if !brush.is_null() {
                let _ = delete_object(brush as Hgdiobj);
            }
        }
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
        let mut parsed = parse_gui_search_query(query);
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
        // A prefix match is unsafe on unresolved dot segments: a path
        // beneath C:\\Projects\\..\\Secrets is not inside Projects.
        if has_dot_path_segment(path) || has_dot_path_segment(scope) {
            return false;
        }
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

    fn parse_gui_search_query(query: &str) -> search_core::ParsedSearchQuery {
        let mut parsed = parse_search_query(query);
        // Search Tool's indexed volume paths use Windows backslashes. Accept
        // forward slashes in user path:/in: filters just as normalize_scope
        // accepts them, without changing free-text or other filter semantics.
        if let Some(needle) = &mut parsed.filters.path_contains {
            if needle.contains('/') {
                *needle = needle.replace('/', "\\");
            }
        }
        parsed
    }

    fn path_matches_explicit_filter(path: &str, needle: Option<&str>) -> bool {
        needle.is_none_or(|needle| search_core::store::normalize_name(path).contains(needle))
    }

    fn apply_scope_filter(parsed: &mut search_core::ParsedSearchQuery, scope: Option<&str>) {
        if let Some(scope) = scope {
            let scope_needle = scope_filter_needle(scope);
            // The search engine has one path_contains slot. When a user's
            // explicit path filter already names a descendant of this scope,
            // use the narrower filter for candidate retrieval instead of
            // overwriting it with the broader scope prefix. refresh_results
            // still checks *both* constraints on every reconstructed path.
            // For an unrelated or generic path filter, keep the scoped
            // pushdown to avoid searching thousands of unrelated directories.
            let explicit_nested = parsed
                .filters
                .path_contains
                .as_deref()
                .is_some_and(|explicit| {
                    explicit.starts_with(&scope_needle) && explicit.len() > scope_needle.len()
                });
            if !explicit_nested {
                parsed.filters.path_contains = Some(scope_needle);
            }
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

    fn palette_with_system_contrast(
        normal: Palette,
        colors: Option<(Rgb, Rgb, Rgb, Rgb)>,
    ) -> Palette {
        let Some((background, text, highlight, highlight_text)) = colors else {
            return normal;
        };
        Palette {
            background,
            surface: background,
            text,
            muted: text,
            accent: highlight,
            selected_text: highlight_text,
        }
    }

    fn rgb_from_colorref(color: u32) -> Rgb {
        Rgb::new(color as u8, (color >> 8) as u8, (color >> 16) as u8)
    }

    unsafe fn system_high_contrast_enabled() -> bool {
        let mut settings = HighContrastW {
            cb_size: std::mem::size_of::<HighContrastW>() as u32,
            flags: 0,
            default_scheme: null_mut(),
        };
        system_parameters_info_w(
            SPI_GETHIGHCONTRAST,
            settings.cb_size,
            (&mut settings as *mut HighContrastW).cast(),
            0,
        ) != 0
            && settings.flags & HCF_HIGHCONTRASTON != 0
    }

    fn resolve_palette(theme: &UiTheme, dark: bool, high_contrast: bool) -> Palette {
        let normal = theme.palette(dark);
        if !high_contrast {
            return normal;
        }
        unsafe {
            palette_with_system_contrast(
                normal,
                Some((
                    rgb_from_colorref(get_sys_color(COLOR_WINDOW)),
                    rgb_from_colorref(get_sys_color(COLOR_WINDOWTEXT)),
                    rgb_from_colorref(get_sys_color(COLOR_HIGHLIGHT)),
                    rgb_from_colorref(get_sys_color(COLOR_HIGHLIGHTTEXT)),
                )),
            )
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
        fn corrupted_listbox_item_data_never_resolves_another_file() {
            assert_eq!(verified_selected_result_index(0, 0, 3), Some(0));
            assert_eq!(verified_selected_result_index(2, 2, 3), Some(2));
            assert_eq!(verified_selected_result_index(-1, 0, 3), None);
            assert_eq!(verified_selected_result_index(0, -1, 3), None);
            assert_eq!(verified_selected_result_index(0, 1, 3), None);
            assert_eq!(verified_selected_result_index(1, 0, 3), None);
            assert_eq!(verified_selected_result_index(3, 3, 3), None);
            assert_eq!(verified_selected_result_index(0, 0, 0), None);
            assert_eq!(verified_selected_result_index(0, isize::MAX, 3), None);
        }

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
            assert!(!path_is_within_scope(
                r"C:\Projects\..\Secrets\private.txt",
                r"C:\Projects"
            ));
            assert!(!path_is_within_scope(
                r"C:\Projects/../Secrets/private.txt",
                r"C:\Projects"
            ));
            assert!(!path_is_within_scope(
                r"C:\Projects\safe.txt",
                r"C:\Projects\..\Secrets"
            ));
            assert!(path_is_within_scope(
                r"C:\Projects\.config\release..txt",
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
        fn gui_path_filters_accept_forward_slashes_without_changing_query_text() {
            for (query, expected) in [
                (r#"report path:"C:/Projects/docs""#, r"c:\projects\docs"),
                (r#"report in:C:/Projects/docs"#, r"c:\projects\docs"),
                (r#"report path:"C:\Projects/docs""#, r"c:\projects\docs"),
            ] {
                let parsed = parse_gui_search_query(query);
                assert_eq!(parsed.text, "report");
                assert_eq!(parsed.filters.path_contains.as_deref(), Some(expected));
                assert!(path_matches_explicit_filter(
                    r"C:\Projects\docs\report.txt",
                    parsed.filters.path_contains.as_deref()
                ));
                assert!(!path_matches_explicit_filter(
                    r"C:\Projects\src\report.txt",
                    parsed.filters.path_contains.as_deref()
                ));
                let mut scoped = parsed.clone();
                apply_scope_filter(&mut scoped, Some(r"C:\Projects"));
                assert_eq!(scoped.filters.path_contains.as_deref(), Some(expected));
            }
            let parsed = parse_gui_search_query("readme.md");
            assert_eq!(parsed.text, "readme.md");
            assert!(parsed.filters.path_contains.is_none());
            let parsed = parse_gui_search_query("notes ext:txt");
            assert_eq!(parsed.text, "notes");
            assert_eq!(parsed.filters.extension.as_deref(), Some("txt"));
        }

        #[test]
        fn nested_explicit_path_filter_is_not_lost_to_broader_scope_pushdown() {
            let mut nested = parse_search_query(r#"report path:"C:\Projects\docs""#);
            let explicit = nested.filters.path_contains.clone().expect("path filter");
            apply_scope_filter(&mut nested, Some(r"C:\Projects"));
            assert_eq!(
                nested.filters.path_contains.as_deref(),
                Some(explicit.as_str())
            );
            assert!(path_is_within_scope(
                r"C:\Projects\docs\report.txt",
                r"C:\Projects"
            ));
            assert!(path_matches_explicit_filter(
                r"C:\Projects\docs\report.txt",
                Some(&explicit)
            ));
            assert!(!path_matches_explicit_filter(
                r"C:\Projects\src\report.txt",
                Some(&explicit)
            ));

            // Unrelated or ambiguous path filters must still prefer the
            // Explorer scope candidate limit, then post-filter explicitly.
            for other in [
                r#"report path:"C:\Projects-old\docs""#,
                "report path:docs",
                r#"report path:"C:\Other\docs""#,
            ] {
                let mut query = parse_search_query(other);
                apply_scope_filter(&mut query, Some(r"C:\Projects"));
                assert_eq!(
                    query.filters.path_contains.as_deref(),
                    Some(r"c:\projects\")
                );
            }
            // Unscoped searches must retain the user's original filter.
            let mut unscoped = parse_search_query(r#"report path:"C:\Projects\docs""#);
            apply_scope_filter(&mut unscoped, None);
            assert_eq!(
                unscoped.filters.path_contains.as_deref(),
                Some(explicit.as_str())
            );
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
        fn resident_explicit_search_request_is_visible_at_launch() {
            assert!(should_show_at_launch(false, true, true));
            assert!(!should_show_at_launch(false, true, false));
            assert!(!should_show_at_launch(true, true, true));
            assert!(!should_show_at_launch(true, false, true));
            assert!(should_show_at_launch(false, false, false));
        }

        #[test]
        fn taskbar_search_flyout_uses_measured_windows11_geometry() {
            let monitor = Rect {
                left: 0,
                top: 0,
                right: 3440,
                bottom: 1440,
            };
            let work = Rect {
                left: 0,
                top: 0,
                right: 3440,
                bottom: 1392,
            };
            assert_eq!(
                taskbar_search_rect(work, monitor, 780, 720, 96),
                Rect {
                    left: 1330,
                    top: 660,
                    right: 2110,
                    bottom: 1380
                }
            );
        }

        #[test]
        fn taskbar_search_flyout_tracks_taskbar_not_screen_center() {
            let work = Rect {
                left: 0,
                top: 0,
                right: 1600,
                bottom: 860,
            };
            assert_eq!(
                taskbar_search_rect(work, work, 900, 640, 96),
                Rect {
                    left: 350,
                    top: 208,
                    right: 1250,
                    bottom: 848
                }
            );
            let ultra_wide = Rect {
                left: 0,
                top: 0,
                right: 3440,
                bottom: 1392,
            };
            assert_eq!(
                taskbar_search_rect(ultra_wide, ultra_wide, 900, 640, 96),
                Rect {
                    left: 1270,
                    top: 740,
                    right: 2170,
                    bottom: 1380
                }
            );
        }

        #[test]
        fn taskbar_search_flyout_respects_top_left_and_right_work_areas() {
            let monitor = Rect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            };
            let top = Rect {
                left: 0,
                top: 48,
                right: 1920,
                bottom: 1080,
            };
            assert_eq!(
                taskbar_search_rect(top, monitor, 900, 640, 96),
                Rect {
                    left: 510,
                    top: 60,
                    right: 1410,
                    bottom: 700
                }
            );

            let left = Rect {
                left: 48,
                top: 0,
                right: 1920,
                bottom: 1080,
            };
            assert_eq!(
                taskbar_search_rect(left, monitor, 900, 640, 96),
                Rect {
                    left: 60,
                    top: 220,
                    right: 960,
                    bottom: 860
                }
            );

            let right = Rect {
                left: 0,
                top: 0,
                right: 1872,
                bottom: 1080,
            };
            assert_eq!(
                taskbar_search_rect(right, monitor, 900, 640, 96),
                Rect {
                    left: 960,
                    top: 220,
                    right: 1860,
                    bottom: 860
                }
            );
        }

        #[test]
        fn taskbar_search_flyout_handles_scaled_monitor_work_area() {
            let monitor = Rect {
                left: 0,
                top: 0,
                right: 2560,
                bottom: 1440,
            };
            let work = Rect {
                left: 0,
                top: 0,
                right: 2560,
                bottom: 1390,
            };
            assert_eq!(
                taskbar_search_rect(work, monitor, 900, 640, 120),
                Rect {
                    left: 717,
                    top: 575,
                    right: 1842,
                    bottom: 1375
                }
            );
        }

        #[test]
        fn taskbar_search_flyout_handles_small_and_negative_work_areas() {
            let small = Rect {
                left: 0,
                top: 0,
                right: 600,
                bottom: 400,
            };
            assert_eq!(
                taskbar_search_rect(small, small, 900, 640, 96),
                Rect {
                    left: 12,
                    top: 12,
                    right: 588,
                    bottom: 388
                }
            );
            let left_monitor = Rect {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1040,
            };
            assert_eq!(
                taskbar_search_rect(left_monitor, left_monitor, 900, 640, 96),
                Rect {
                    left: -1410,
                    top: 388,
                    right: -510,
                    bottom: 1028
                }
            );
        }

        #[test]
        fn enter_opens_best_match_only_from_query_with_results() {
            assert!(should_select_best_match(true, -1, 1));
            assert!(should_select_best_match(true, -1, 30));
            assert!(!should_select_best_match(true, -1, 0));
            assert!(!should_select_best_match(false, -1, 5));
            assert!(!should_select_best_match(true, 0, 5));
            assert!(!should_select_best_match(true, 3, 5));
        }

        #[test]
        fn native_details_use_two_columns_at_reference_width() {
            let client = Rect {
                left: 0,
                top: 0,
                right: 780,
                bottom: 720,
            };
            let (list, detail) = native_result_columns(client, 96).expect("reference flyout");
            assert_eq!(
                list,
                Rect {
                    left: 24,
                    top: 158,
                    right: 396,
                    bottom: 696
                }
            );
            assert_eq!(
                detail,
                Rect {
                    left: 412,
                    top: 158,
                    right: 756,
                    bottom: 696
                }
            );
            assert!(detail.right - detail.left > 300);
        }

        #[test]
        fn system_contrast_palette_uses_windows_foreground_and_background() {
            let normal = UiTheme::default().palette(true);
            assert_eq!(palette_with_system_contrast(normal, None), normal);
            let window = Rgb::new(2, 4, 6);
            let text = Rgb::new(248, 249, 250);
            let highlight = Rgb::new(12, 20, 31);
            let selected = Rgb::new(255, 250, 199);
            let palette =
                palette_with_system_contrast(normal, Some((window, text, highlight, selected)));
            assert_eq!(palette.background, window);
            assert_eq!(palette.surface, window);
            assert_eq!(palette.text, text);
            assert_eq!(palette.muted, text);
            assert_eq!(palette.accent, highlight);
            assert_eq!(palette.selected_text, selected);
            assert_eq!(rgb_from_colorref(0x00_24_12_F0), Rgb::new(240, 18, 36));
        }

        #[test]
        fn malformed_native_labels_are_rejected_without_discarding_valid_rows() {
            let mut row = ResultRow {
                name: "notes.txt".into(),
                path: "C:\\Demo\\notes.txt".into(),
                is_directory: false,
            };
            assert_eq!(
                verified_result_accessible_label(&row),
                Some(result_accessible_label(&row))
            );
            row.name = "invalid\0spoof.txt".into();
            assert!(verified_result_accessible_label(&row).is_none());
            row.name = "notes.txt".into();
            row.path = "C:\\Demo\\invalid\0spoof.txt".into();
            assert!(verified_result_accessible_label(&row).is_none());
            row.path = "C:\\Demo\\notes.txt".into();
            row.name = "x".repeat(MAX_NATIVE_LABEL_U16 + 1);
            assert!(verified_result_accessible_label(&row).is_none());
            row.name = "notes.txt".into();
            assert!(verified_result_accessible_label(&row).is_some());
        }

        #[test]
        fn deleted_or_type_changed_index_result_cannot_be_opened() {
            use std::{
                fs,
                time::{SystemTime, UNIX_EPOCH},
            };

            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time after unix epoch")
                .as_nanos();
            let folder = env::temp_dir().join(format!(
                "search-tool-open-preflight-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir(&folder).expect("create isolated temporary test folder");
            let file = folder.join("fixture.txt");
            let file_path = file.to_string_lossy().into_owned();
            let directory_path = folder.to_string_lossy().into_owned();
            assert!(selected_path_still_openable(&directory_path, true));
            assert!(!selected_path_still_openable(&directory_path, false));
            assert!(!selected_path_still_openable(&file_path, false));
            fs::write(&file, b"safe-open-guard-fixture").expect("write synthetic fixture");
            assert!(selected_path_still_openable(&file_path, false));
            assert!(!selected_path_still_openable(&file_path, true));
            fs::remove_file(&file).expect("delete synthetic fixture");
            assert!(!selected_path_still_openable(&file_path, false));
            fs::remove_dir(&folder).expect("remove isolated temporary test folder");
            assert!(!selected_path_still_openable(&directory_path, true));
        }

        #[test]
        fn scoped_open_rechecks_resolved_filesystem_containment() {
            use std::{
                fs,
                time::{SystemTime, UNIX_EPOCH},
            };
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time after unix epoch")
                .as_nanos();
            let root = env::temp_dir().join(format!(
                "search-tool-scope-preflight-{}-{unique}",
                std::process::id()
            ));
            let inside = root.join("Projects");
            let sibling = root.join("Projects-old");
            fs::create_dir_all(&inside).expect("create isolated scoped fixture");
            fs::create_dir(&sibling).expect("create sibling fixture");
            let good = inside.join("allowed.txt");
            let outside = sibling.join("private.txt");
            fs::write(&good, b"in-scope").expect("write in-scope fixture");
            fs::write(&outside, b"out-of-scope").expect("write sibling fixture");
            let scope = inside.to_string_lossy();
            assert!(selected_path_within_scope(
                &good.to_string_lossy(),
                Some(&scope)
            ));
            assert!(selected_path_within_scope(
                &inside.to_string_lossy(),
                Some(&scope)
            ));
            assert!(!selected_path_within_scope(
                &outside.to_string_lossy(),
                Some(&scope)
            ));
            assert!(!selected_path_within_scope(
                &good.to_string_lossy(),
                Some(&sibling.to_string_lossy())
            ));
            assert!(!selected_path_within_scope(
                &inside.join("missing.txt").to_string_lossy(),
                Some(&scope)
            ));
            assert!(selected_path_within_scope(&outside.to_string_lossy(), None));

            // An ordinary directory prefix check accepts this path, but the
            // junction resolves to the sibling outside the search scope.
            let junction = inside.join("linked-outside");
            let created = std::process::Command::new("cmd.exe")
                .args(["/C", "mklink", "/J"])
                .arg(&junction)
                .arg(&sibling)
                .output()
                .expect("create isolated Windows junction fixture");
            assert!(
                created.status.success(),
                "mklink /J failed: {}",
                String::from_utf8_lossy(&created.stderr)
            );
            let linked_outside = junction.join("private.txt");
            assert!(path_is_within_scope(
                &linked_outside.to_string_lossy(),
                &scope
            ));
            assert!(!selected_path_within_scope(
                &linked_outside.to_string_lossy(),
                Some(&scope)
            ));
            fs::remove_dir(&junction).expect("remove isolated junction without following target");
            fs::remove_dir_all(&root).expect("remove isolated scoped fixture");
            assert!(!selected_path_within_scope(
                &good.to_string_lossy(),
                Some(&scope)
            ));
        }

        #[test]
        fn unresolvable_or_drive_relative_results_cannot_be_opened() {
            let rooted = "C:\\Users\\Demo\\real.txt";
            assert_eq!(
                verified_result_path(Ok(rooted.to_string())).as_deref(),
                Some(rooted)
            );
            assert_eq!(
                verified_result_path(Ok("D:/legitimate/path.txt".to_string())).as_deref(),
                Some("D:/legitimate/path.txt")
            );
            assert_eq!(
                verified_result_path(Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "orphaned parent record",
                ))),
                None
            );
            assert_eq!(verified_result_path(Ok("C:wrong.txt".into())), None);
            assert_eq!(verified_result_path(Ok("relative.txt".into())), None);
            assert_eq!(
                verified_result_path(Ok("C:\\truncated\0wrong.txt".into())),
                None
            );
            assert_eq!(verified_result_path(Ok(String::new())), None);
            // The index may be stale or malformed. Windows path resolution
            // must not escape a lexical folder scope via dot components.
            for path in [
                r"C:\Projects\..\Secrets\private.txt",
                r"C:\Projects\.\visible.txt",
                "C:/Projects/../Secrets/private.txt",
                "C:\\Projects/..\\Secrets/private.txt",
                r"C:\..\Windows\system.ini",
            ] {
                assert_eq!(verified_result_path(Ok(path.into())), None, "{path}");
            }
            for path in [
                r"C:\Projects\.git\config",
                r"C:\Projects\release..txt",
                r"C:\Projects\subdir\file.txt",
            ] {
                assert_eq!(verified_result_path(Ok(path.into())).as_deref(), Some(path));
            }
        }

        #[test]
        fn query_down_retains_valid_selection_or_selects_first() {
            assert_eq!(query_down_target(-1, 0), None);
            assert_eq!(query_down_target(0, 0), None);
            assert_eq!(query_down_target(-1, 3), Some(0));
            assert_eq!(query_down_target(0, 3), Some(0));
            assert_eq!(query_down_target(1, 3), Some(1));
            assert_eq!(query_down_target(2, 3), Some(2));
            assert_eq!(query_down_target(3, 3), Some(0));
        }

        #[test]
        fn refreshed_result_selection_tracks_full_path_not_duplicate_name() {
            let mut rows = vec![
                ResultRow {
                    name: "same.txt".into(),
                    path: "C:\\first\\same.txt".into(),
                    is_directory: false,
                },
                ResultRow {
                    name: "same.txt".into(),
                    path: "D:\\other\\same.txt".into(),
                    is_directory: false,
                },
            ];
            assert_eq!(
                refreshed_selection_index(&rows, Some(("D:\\other\\same.txt", false))),
                Some(1)
            );
            assert_eq!(
                refreshed_selection_index(&rows, Some(("C:\\first\\same.txt", false))),
                Some(0)
            );
            assert_eq!(
                refreshed_selection_index(&rows, Some(("E:\\gone.txt", false))),
                Some(0)
            );
            // A path which changed its kind is not the selected object.
            // Use the first valid result instead of preserving that selection.
            rows[1].is_directory = true;
            assert_eq!(
                refreshed_selection_index(&rows, Some(("D:\\other\\same.txt", false))),
                Some(0)
            );
            assert_eq!(
                refreshed_selection_index(&rows, Some(("D:\\other\\same.txt", true))),
                Some(1)
            );
            assert_eq!(refreshed_selection_index(&rows, None), Some(0));
            assert_eq!(
                refreshed_selection_index(&[], Some(("D:\\other\\same.txt", false))),
                None
            );
        }

        #[test]
        fn accessible_result_name_changes_notify_only_visible_popups() {
            let no_results = accessible_results_name(0);
            let three_results = accessible_results_name(3);
            assert!(should_notify_result_name_change(
                &no_results,
                &three_results,
                true
            ));
            assert!(!should_notify_result_name_change(
                &three_results,
                &three_results,
                true
            ));
            assert!(!should_notify_result_name_change(
                &no_results,
                &three_results,
                false
            ));
            assert!(should_notify_result_name_change(
                &three_results,
                &no_results,
                true
            ));
        }

        #[test]
        fn result_count_accessible_name_is_localized_and_unambiguous() {
            assert_eq!(accessible_results_name(0), "Arama sonuçları (0 sonuç)");
            assert_eq!(accessible_results_name(1), "Arama sonuçları (1 sonuç)");
            assert_eq!(accessible_results_name(3), "Arama sonuçları (3 sonuç)");
        }

        #[test]
        fn filter_accessible_names_are_language_and_state_explicit() {
            assert_eq!(accessible_filter_name("Tümü", true), "Tümü (seçili)");
            assert_eq!(accessible_filter_name("Tümü", false), "Tümü");
            assert_eq!(accessible_filter_name("İçerik", true), "İçerik (seçili)");
            assert!(!accessible_filter_name("Dosyalar", true).contains('•'));
        }

        #[test]
        fn hidden_result_list_restores_focus_only_when_popup_is_visible() {
            assert!(should_restore_query_focus(true, true));
            assert!(!should_restore_query_focus(false, true));
            assert!(!should_restore_query_focus(true, false));
            assert!(!should_restore_query_focus(false, false));
        }

        #[test]
        fn ime_composition_keeps_popup_shortcuts_out_of_edit() {
            assert!(!popup_shortcuts_allowed(true, true));
            assert!(popup_shortcuts_allowed(true, false));
            // Both the regular keyboard path and Shell-bridge Enter must
            // respect an active composition in the native EDIT.
            assert!(popup_shortcuts_allowed(false, true));
            assert!(popup_shortcuts_allowed(false, false));
            // Native EDIT still receives its key messages: the outer loop
            // merely skips global shortcut handling, never discards the key.
            assert!(should_handle_dialog_tab(WM_KEYDOWN, VK_TAB as usize));
        }

        #[test]
        fn only_tab_is_handled_by_dialog_keyboard_translation() {
            assert!(should_handle_dialog_tab(WM_KEYDOWN, VK_TAB as usize));
            assert!(!should_handle_dialog_tab(WM_KEYUP, VK_TAB as usize));
            assert!(!should_handle_dialog_tab(WM_SYSKEYDOWN, VK_TAB as usize));
            assert!(!should_handle_dialog_tab(WM_KEYDOWN, VK_RETURN));
            assert!(!should_handle_dialog_tab(WM_KEYDOWN, VK_DOWN));
            assert!(!should_handle_dialog_tab(WM_KEYDOWN, VK_ESCAPE));
        }

        #[test]
        fn enter_never_opens_search_results_from_other_controls() {
            let edit = menu_id(1);
            let list = menu_id(2);
            let theme = menu_id(14);
            let open_button = menu_id(19);
            assert!(should_route_result_enter(edit, edit, list));
            assert!(should_route_result_enter(list, edit, list));
            assert!(!should_route_result_enter(theme, edit, list));
            assert!(!should_route_result_enter(open_button, edit, list));
            assert!(!should_route_result_enter(null_mut(), edit, list));
        }

        #[test]
        fn shell_icons_resolve_synthetic_types_and_release_handles() {
            // Neither query needs the file or directory to exist on disk.
            for (name, attributes) in [
                (".txt", FILE_ATTRIBUTE_NORMAL),
                ("folder", FILE_ATTRIBUTE_DIRECTORY),
            ] {
                let mut info: ShFileInfoW = unsafe { std::mem::zeroed() };
                let name = wide(name);
                let result = unsafe {
                    sh_get_file_info_w(
                        name.as_ptr(),
                        attributes,
                        &mut info,
                        std::mem::size_of::<ShFileInfoW>() as u32,
                        SHGFI_ICON | SHGFI_SMALLICON | SHGFI_USEFILEATTRIBUTES,
                    )
                };
                assert_ne!(result, 0, "synthetic Shell icon lookup failed");
                assert!(!info.icon.is_null());
                assert_ne!(unsafe { destroy_icon(info.icon) }, 0);
            }
        }

        #[test]
        fn shell_icons_use_bounded_type_keys_without_file_io() {
            assert_eq!(shell_icon_key("photo.PNG", false), ".png");
            assert_eq!(shell_icon_key("archive.tar.GZ", false), ".gz");
            assert_eq!(shell_icon_key("README", false), "file");
            assert_eq!(shell_icon_key("photo.PNG", true), "folder");
            assert!(shell_icon_key(&format!("file.{}", "x".repeat(300)), false).len() <= 25);
        }

        #[test]
        fn detail_content_clears_stale_data_between_searches() {
            let first = ResultRow {
                name: "SearchTool Notes.md".to_string(),
                path: r"C:\Users\Demo\SearchTool Notes.md".to_string(),
                is_directory: false,
            };
            let next = ResultRow {
                name: "Reports".to_string(),
                path: r"C:\Users\Demo\Reports".to_string(),
                is_directory: true,
            };
            assert_eq!(
                detail_content(Some(&first)),
                ("SearchTool Notes.md", "Dosya", first.path.as_str())
            );
            assert_eq!(detail_content(None), ("", "", ""));
            assert_eq!(
                detail_content(Some(&next)),
                ("Reports", "Klasör", next.path.as_str())
            );
            assert_eq!(detail_content(None), ("", "", ""));
        }

        #[test]
        fn native_details_collapse_on_compact_screen() {
            assert!(native_result_columns(
                Rect {
                    left: 0,
                    top: 0,
                    right: 700,
                    bottom: 720
                },
                96
            )
            .is_none());
            assert!(native_result_columns(
                Rect {
                    left: 0,
                    top: 0,
                    right: 780,
                    bottom: 260
                },
                96
            )
            .is_none());
            let offset_client = Rect {
                left: 0,
                top: 0,
                right: 975,
                bottom: 900,
            };
            let (list, detail) = native_result_columns(offset_client, 120).unwrap();
            assert!(list.right < detail.left && detail.right <= offset_client.right);
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
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|arg| arg == "--ui-selftest") {
            // A CI-only hidden test must never block on a modal MessageBox.
            if let Some(position) = args.iter().position(|arg| arg == "--ui-selftest-report") {
                if let Some(path) = args.get(position + 1) {
                    let _ = std::fs::write(path, format!("FAIL: {error}"));
                }
            }
        } else {
            windows_app::show_error(&error.to_string());
        }
        std::process::exit(1);
    }
}
