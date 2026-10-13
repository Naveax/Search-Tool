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
    const WM_GETFONT: u32 = 0x0031;
    const WM_SHELL_BRIDGE_BEGIN: u32 = 0x8000 + 0x51;
    const WM_SHELL_BRIDGE_CHAR: u32 = 0x8000 + 0x52;
    const WM_SHELL_BRIDGE_KEY: u32 = 0x8000 + 0x53;
    const WM_THEME_BUTTON_HOT: u32 = 0x8000 + 0x54;

    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TOPMOST: u32 = 0x0000_0008;
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
    const EM_LIMITTEXT: u32 = 0x00C5;
    const EM_GETLIMITTEXT: u32 = 0x00D5;
    const EM_SETMARGINS: u32 = 0x00D3;
    const EM_GETMARGINS: u32 = 0x00D4;
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
    const SHELL_BRIDGE_KEY_VK_SHIFT: u32 = 18;

    static SHELL_BRIDGE_WINDOW: AtomicUsize = AtomicUsize::new(0);
    static SHELL_BRIDGE_ACTIVE: AtomicBool = AtomicBool::new(false);
    // BEGIN is posted asynchronously; active=true alone cannot distinguish
    // an older cancelled BEGIN from a newer currently active request.
    static SHELL_BRIDGE_BEGIN_GENERATION: AtomicUsize = AtomicUsize::new(0);
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
        // Zero means the visible popup belongs to a manual hotkey/IPC
        // request, not the optional asynchronous Shell keyboard bridge.
        bridge_session_generation: usize,
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
        // Publish the new generation *while holding the scope mutex*.
        // Otherwise an older posted BEGIN can observe the old generation
        // paired with the new scope between these two updates.
        // Generation zero is reserved for untagged/stale messages.
        let mut generation = SHELL_BRIDGE_BEGIN_GENERATION
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        if generation == 0 {
            generation = SHELL_BRIDGE_BEGIN_GENERATION
                .fetch_add(1, Ordering::AcqRel)
                .wrapping_add(1);
        }
        *pending_scope = scope;
        drop(pending_scope);
        let hwnd = SHELL_BRIDGE_WINDOW.load(Ordering::Acquire) as Hwnd;
        if hwnd.is_null() || post_message_w(hwnd, WM_SHELL_BRIDGE_BEGIN, generation, 0) == 0 {
            clear_pending_bridge_scope_after_failed_post(generation);
            return false;
        }
        true
    }

    // A posted BEGIN can be delivered after its keyboard interception was
    // cancelled. Never resurrect the Search popup for that stale request.
    fn bridge_begin_is_current(
        active: bool,
        incoming_generation: usize,
        latest_generation: usize,
    ) -> bool {
        active && incoming_generation != 0 && incoming_generation == latest_generation
    }

    // A failed PostMessage for generation N must never erase a newer
    // generation N+1's pending Explorer folder scope.
    fn bridge_failed_post_may_clear_scope(
        failed_generation: usize,
        latest_generation: usize,
    ) -> bool {
        failed_generation != 0 && failed_generation == latest_generation
    }

    fn clear_pending_bridge_scope_after_failed_post(failed_generation: usize) {
        if let Ok(mut pending_scope) = SHELL_BRIDGE_SCOPE.lock() {
            // The newer generation and scope are both published under this
            // mutex, so a delayed failure from N cannot clear N+1's scope.
            if bridge_failed_post_may_clear_scope(
                failed_generation,
                SHELL_BRIDGE_BEGIN_GENERATION.load(Ordering::Acquire),
            ) {
                *pending_scope = None;
            }
        }
    }

    // CHAR and KEY are also asynchronous. A delayed event from request N
    // must not act on the visible query of a new request N+1.
    fn bridge_input_is_current(incoming_generation: usize, latest_generation: usize) -> bool {
        incoming_generation != 0 && incoming_generation == latest_generation
    }

    // A resident popup can be reopened manually without starting a new
    // keyboard-bridge generation. Only input from its actual owner applies.
    fn bridge_input_belongs_to_displayed_session(
        incoming_generation: usize,
        latest_generation: usize,
        displayed_bridge_generation: usize,
    ) -> bool {
        bridge_input_is_current(incoming_generation, latest_generation)
            && incoming_generation == displayed_bridge_generation
    }

    fn take_bridge_scope() -> Option<String> {
        SHELL_BRIDGE_SCOPE
            .lock()
            .ok()
            .and_then(|mut scope| scope.take())
    }

    // A newer producer may replace the pending scope after the first BEGIN
    // check but before the UI thread acquires the mutex to consume it.
    // Recheck ownership while holding that same publication mutex.
    fn take_current_bridge_begin_scope(generation: usize) -> Option<Option<String>> {
        let mut pending_scope = SHELL_BRIDGE_SCOPE.lock().ok()?;
        if !bridge_begin_is_current(
            SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire),
            generation,
            SHELL_BRIDGE_BEGIN_GENERATION.load(Ordering::Acquire),
        ) {
            return None;
        }
        Some(pending_scope.take())
    }

    // WM_SHELL_BRIDGE_CHAR carries one full Unicode scalar value in lParam,
    // never an isolated UTF-16 surrogate. Backspace remains U+0008.
    unsafe fn post_bridge_char(codepoint: u32) -> bool {
        let hwnd = SHELL_BRIDGE_WINDOW.load(Ordering::Acquire) as Hwnd;
        let generation = SHELL_BRIDGE_BEGIN_GENERATION.load(Ordering::Acquire);
        !hwnd.is_null()
            && generation != 0
            && post_message_w(hwnd, WM_SHELL_BRIDGE_CHAR, generation, codepoint as isize) != 0
    }

    fn pack_bridge_key_payload(vk: u32, scan_code: u32, extended: bool, control: bool) -> isize {
        let mut packed =
            (scan_code as isize & 0xFFFF) | (((vk & 0xFF) as isize) << SHELL_BRIDGE_KEY_VK_SHIFT);
        if extended {
            packed |= 1 << 16;
        }
        if control {
            packed |= SHELL_BRIDGE_KEY_CONTROL;
        }
        packed
    }

    fn bridge_key_virtual_key(payload: isize) -> u32 {
        ((payload as usize >> SHELL_BRIDGE_KEY_VK_SHIFT) & 0xFF) as u32
    }

    unsafe fn post_bridge_key(vk: u32, scan_code: u32, extended: bool, control: bool) -> bool {
        let hwnd = SHELL_BRIDGE_WINDOW.load(Ordering::Acquire) as Hwnd;
        if hwnd.is_null() {
            return false;
        }
        let generation = SHELL_BRIDGE_BEGIN_GENERATION.load(Ordering::Acquire);
        if generation == 0 {
            return false;
        }
        // All routed virtual keys fit in 8 bits. Reserve the low 18 bits
        // for scan/extended/control and pack the VK above them. This leaves
        // wParam free for a full-width request generation, even on Win32.
        let packed = pack_bridge_key_payload(vk, scan_code, extended, control);
        post_message_w(hwnd, WM_SHELL_BRIDGE_KEY, generation, packed) != 0
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

    // Translate complete UTF-16 *scalars* before posting anything. An emoji
    // is one message, not two independently-queued surrogate halves; an
    // invalid translation must not post a valid prefix before failing.
    fn post_bridge_utf16_units(units: &[u16], mut post: impl FnMut(u32) -> bool) -> bool {
        // Iterator::all returns true on empty input. That used to swallow a
        // dead key or failed ToUnicodeEx translation even with no character.
        if units.is_empty() {
            return false;
        }
        let mut scalars = Vec::with_capacity(units.len());
        for decoded in char::decode_utf16(units.iter().copied()) {
            let Ok(scalar) = decoded else {
                return false;
            };
            if scalar == '\0' {
                return false;
            }
            scalars.push(scalar as u32);
        }
        scalars.into_iter().all(&mut post)
    }

    unsafe fn post_translated_bridge_chars(vk: u32, scan_code: u32) -> bool {
        let units = translate_bridge_chars(vk, scan_code);
        post_bridge_utf16_units(&units, |unit| post_bridge_char(unit))
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

    unsafe fn append_bridge_unicode_scalar(edit: Hwnd, codepoint: u32) {
        let (selection_start, selection_end) = bridge_selection(edit);

        if codepoint == VK_BACK {
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

        // Ignore malformed or unpaired codepoints, including NUL. UTF-16
        // length and selection limits are checked before native EDIT changes.
        let Some(character) = char::from_u32(codepoint).filter(|ch| *ch != '\0') else {
            return;
        };
        let mut encoded = [0u16; 2];
        let units = character.encode_utf16(&mut encoded);
        let text_len = get_window_text_length_w(edit).max(0) as usize;
        let selected = selection_end.saturating_sub(selection_start) as usize;
        if text_len
            .saturating_sub(selected)
            .saturating_add(units.len())
            > MAX_QUERY_U16.saturating_sub(1) as usize
        {
            return;
        }
        replace_bridge_selection(edit, units);
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

    // An already-hidden popup can receive late WA_INACTIVE/WM_CLOSE events.
    // Clean stale visual state without touching the global keyboard bridge:
    // a newer BEGIN may already own the pending scope and ACTIVE flag.
    unsafe fn clear_hidden_resident_chrome(hwnd: Hwnd, state: &mut State) {
        let _ = set_window_pos(
            hwnd,
            notopmost_window(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        if state.theme_button_hot {
            state.theme_button_hot = false;
            let _ = invalidate_rect(state.theme_button, null_mut(), 0);
        }
    }

    unsafe fn hide_bridge_overlay(hwnd: Hwnd, state: &mut State) {
        // A cancellation Escape can still be queued after the hook drops
        // ACTIVE. The next manual Search opening must never inherit it.
        state.bridge_session_generation = 0;
        // Some IMEs do not send a final composition-end after dismissal.
        // A new Search session must not inherit the old preedit guard.
        state.ime_composing = false;
        SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
        if let Ok(mut pending_scope) = SHELL_BRIDGE_SCOPE.lock() {
            *pending_scope = None;
        }
        show_window(hwnd, SW_HIDE);
        state.intent_model = None;
        // A hidden resident popup cannot own keyboard focus. Do not retain
        // the highlighted search-border state if EN_KILLFOCUS never arrives.
        state.edit_focused = false;
        // Hiding can bypass WM_MOUSELEAVE. Release temporary topmost and
        // Appearance hover before the next resident popup session.
        clear_hidden_resident_chrome(hwnd, state);
    }

    // Manual hotkey and explicit IPC requests take ownership away from the
    // optional shell keyboard bridge without dismissing the GUI window.
    unsafe fn handoff_bridge_to_manual_session(hwnd: Hwnd, state: &mut State) {
        SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
        state.bridge_session_generation = 0;
        // A delayed EDIT START from the dismissed session can still arrive
        // after hide_bridge_overlay cleared its guard. Do not carry the old
        // IME preedit state into a manual hotkey or explicit IPC takeover.
        state.ime_composing = false;
        let _ = take_bridge_scope();
        // IPC/manual Search must not inherit the bridge overlay's temporary
        // topmost z-order, even when it takes over the already visible HWND.
        let _ = set_window_pos(
            hwnd,
            notopmost_window(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        if state.theme_button_hot {
            state.theme_button_hot = false;
            let _ = invalidate_rect(state.theme_button, null_mut(), 0);
        }
    }

    // Keep successful launches on the same resident dismissal path as Escape
    // and focus loss. The regression invokes this with an isolated hidden HWND
    // instead of ShellExecute, so no actual file or app is launched.
    unsafe fn dismiss_resident_after_open(hwnd: Hwnd, state: &mut State) {
        if state.resident {
            hide_bridge_overlay(hwnd, state);
        }
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
                if key_down && !post_bridge_char(VK_BACK) {
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
                    let value = args.next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "--query requires a value")
                    })?;
                    apply_cli_query(&mut initial_request, &value)?;
                }
                "--scope" => {
                    let value = args.next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "--scope requires a value")
                    })?;
                    apply_cli_scope(&mut initial_request, &value)?;
                }
                "--search-uri" => {
                    let value = args.next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "--search-uri requires a value")
                    })?;
                    apply_cli_search_uri(&mut initial_request, &value)?;
                }
                _ if is_search_uri(&arg) => {
                    apply_cli_search_uri(&mut initial_request, &arg)?;
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
            bridge_session_generation: 0,
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

            // Dismissal may happen after a native child posted text/Enter
            // but before GetMessage consumes it. Never send those queued
            // messages to an invisible resident popup or interpret them as
            // Search shortcuts. Hotkeys and IPC are intentionally unaffected.
            let dismissed_input = unsafe {
                let state_ptr = get_window_long_ptr_w(hwnd, GWLP_USERDATA) as *const State;
                !state_ptr.is_null()
                    && should_drop_dismissed_popup_input(
                        (*state_ptr).resident,
                        is_window_visible(hwnd) != 0,
                        msg.message,
                        msg.hwnd == hwnd || (!msg.hwnd.is_null() && get_parent(msg.hwnd) == hwnd),
                    )
            };
            if dismissed_input {
                continue;
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
            // IME completion can arrive after the resident parent has been
            // dismissed. End composition bookkeeping, but never start a
            // background requery for an invisible Search popup.
            if !(*state_ptr).resident || is_window_visible(parent) != 0 {
                refresh_from_committed_edit_change(&mut *state_ptr);
            }
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
                    if is_window_visible(hwnd) != 0 {
                        // Real focus loss dismisses an onscreen resident
                        // popup and cancels its bridge ownership.
                        hide_bridge_overlay(hwnd, &mut *state_ptr);
                    } else {
                        // A delayed deactivation of an already-hidden popup
                        // must not cancel a newer pending Shell BEGIN.
                        clear_hidden_resident_chrome(hwnd, &mut *state_ptr);
                    }
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
                // A queued hover notification from a hidden native child is
                // not real pointer hover over a visible Search flyout.
                let hot = w_param != 0 && is_window_visible(hwnd) != 0;
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
                    // Native children keep their WS_VISIBLE style after the
                    // resident parent hides. A delayed focus notification
                    // must not highlight a dismissed Search popup.
                    if !state.resident || is_window_visible(hwnd) != 0 {
                        state.edit_focused = true;
                        invalidate_search_frame(hwnd, state);
                    }
                    return 0;
                }
                if source == state.edit && notification == EN_KILLFOCUS {
                    state.edit_focused = false;
                    invalidate_search_frame(hwnd, state);
                    return 0;
                }
                if source == state.edit && notification == EN_CHANGE {
                    // A dismissed resident flyout can still receive a queued
                    // EN_CHANGE from its native child. Do not silently requery
                    // after closure or overwrite the next session's status.
                    if !state.resident || is_window_visible(hwnd) != 0 {
                        refresh_from_committed_edit_change(state);
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
                        // WM_COMMAND can arrive after WM_SIZE has hidden the
                        // source chip. A stale BN_CLICKED must never change
                        // the selected category or run a hidden action.
                        // A child WS_VISIBLE style is not sufficient: the
                        // dismissed parent cannot receive a real user click.
                        if is_window_visible(hwnd) == 0
                            || get_window_long_ptr_w(source, GWL_STYLE) as u32 & WS_VISIBLE == 0
                        {
                            return 0;
                        }
                        // Re-selecting an active mode remains a no-op.
                        select_search_category(state, mode);
                        return 0;
                    }
                    if source == state.detail_open {
                        // A queued click can outlive a responsive resize or
                        // selection loss. A hidden Open button may not launch
                        // files or clear a still-valid cached selection.
                        // WS_VISIBLE on a child does not imply its parent is
                        // onscreen. Ignore queued Open clicks after the flyout
                        // loses activation or is otherwise dismissed.
                        if is_window_visible(hwnd) != 0
                            && get_window_long_ptr_w(source, GWL_STYLE) as u32 & WS_VISIBLE != 0
                        {
                            let _ = open_selected(hwnd, state);
                        }
                        return 0;
                    }
                    if source == state.theme_button {
                        // Ignore an Appearance click after the parent popup
                        // is dismissed, even if the child keeps WS_VISIBLE.
                        // A stale BN_CLICKED must not create a popup menu on
                        // the user's desktop after Search has closed.
                        if is_window_visible(hwnd) != 0
                            && get_window_long_ptr_w(source, GWL_STYLE) as u32 & WS_VISIBLE != 0
                        {
                            show_theme_menu(hwnd, state);
                        }
                        return 0;
                    }
                }
                if source == state.list && notification == LBN_SELCHANGE {
                    // A selection message queued before WM_SIZE may arrive
                    // after the ListBox was hidden. Preserve its cached best
                    // match until the window can display a full result row.
                    // A dismissed parent stays invisible while a native
                    // ListBox child can retain WS_VISIBLE. Old selection
                    // notifications must not clear a cached valid selection.
                    if is_window_visible(hwnd) == 0
                        || get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE == 0
                    {
                        return 0;
                    }
                    process_list_selection_change(state);
                    return 0;
                }
                if source == state.list && notification == LBN_DBLCLK {
                    // A hidden ListBox is not actionable; an old double-click
                    // must not clear the selection or attempt ShellExecute.
                    if is_window_visible(hwnd) != 0
                        && get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE != 0
                    {
                        let _ = open_selected(hwnd, state);
                    }
                }
                0
            }
            WM_SHELL_BRIDGE_BEGIN if !state_ptr.is_null() => {
                // The hook posts BEGIN asynchronously. Cancellation can win
                // before it is dispatched; consuming that old BEGIN would
                // reopen Search and steal the user's next typing session.
                if !bridge_begin_is_current(
                    SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire),
                    w_param,
                    SHELL_BRIDGE_BEGIN_GENERATION.load(Ordering::Acquire),
                ) {
                    // A stale BEGIN from generation N must not consume the
                    // pending Explorer scope belonging to generation N+1.
                    // For a cancelled current request only, clear its scope;
                    // recheck state while holding the scope mutex to avoid
                    // racing a newer producer.
                    if let Ok(mut pending) = SHELL_BRIDGE_SCOPE.lock() {
                        if !SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire)
                            && SHELL_BRIDGE_BEGIN_GENERATION.load(Ordering::Acquire) == w_param
                        {
                            *pending = None;
                        }
                    }
                    return 0;
                }
                // Do not split the generation check and pending-scope take
                // across a producer's lock acquisition: N must not consume
                // the new scope published for N+1.
                let Some(scope) = take_current_bridge_begin_scope(w_param) else {
                    return 0;
                };
                let state = &mut *state_ptr;
                state.bridge_session_generation = w_param;
                state.scope = scope;
                state.intent_model = None;
                // BEGIN starts a new Search edit session, never inheriting an
                // old IME composition that survived a hidden EDIT event.
                state.ime_composing = false;
                // Explicitly reset the previous cached results before
                // displaying a new bridge session. The hidden-parent
                // EN_CHANGE guard intentionally ignores this native EDIT
                // notification while Search is still dismissed.
                set_query(state, "");
                show_bridge_overlay(hwnd, state);
                0
            }
            WM_SHELL_BRIDGE_CHAR if !state_ptr.is_null() => {
                // Delayed text from a previous bridge generation may arrive
                // while an entirely different Search session is now visible.
                if bridge_input_belongs_to_displayed_session(
                    w_param,
                    SHELL_BRIDGE_BEGIN_GENERATION.load(Ordering::Acquire),
                    (*state_ptr).bridge_session_generation,
                ) && SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire)
                    && is_window_visible(hwnd) != 0
                {
                    // Full Unicode scalar, including non-BMP characters.
                    append_bridge_unicode_scalar((*state_ptr).edit, l_param as u32);
                }
                0
            }
            WM_SHELL_BRIDGE_KEY if !state_ptr.is_null() => {
                // Preserve cancellation Escape for this generation even after
                // the hook marks itself inactive. Reject all older key events.
                let vk = bridge_key_virtual_key(l_param);
                if !bridge_input_belongs_to_displayed_session(
                    w_param,
                    SHELL_BRIDGE_BEGIN_GENERATION.load(Ordering::Acquire),
                    (*state_ptr).bridge_session_generation,
                ) || is_window_visible(hwnd) == 0
                    || (!SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire) && vk != VK_ESCAPE as u32)
                {
                    return 0;
                }
                let state = &mut *state_ptr;
                let scan_code = (l_param as u32) & 0xFFFF;
                let extended = ((l_param as u32) & (1 << 16)) != 0;
                let control = (l_param & SHELL_BRIDGE_KEY_CONTROL) != 0;
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
                    // A manually invoked popup must not inherit an old
                    // asynchronous bridge session or its z-order state.
                    handoff_bridge_to_manual_session(hwnd, state);
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
                // A malformed local sender can supply a null COPYDATASTRUCT.
                // Dereferencing it crashes the resident GUI before any of the
                // normal payload-length guards can reject the request.
                if l_param == 0 {
                    return 0;
                }
                let state = &mut *state_ptr;
                let copy = &*(l_param as *const CopyDataStruct);
                let bytes = copy.cb_data as usize;
                // Bound the declared byte length before forming any slice
                // from an untrusted sender's memory. Unknown message types,
                // odd lengths and oversized payloads are not requests.
                if !copy.lp_data.is_null() && valid_copydata_length(copy.dw_data, bytes) {
                    let words = slice::from_raw_parts(copy.lp_data as *const u16, bytes / 2);
                    let request = match copy.dw_data {
                        1 => decode_legacy_ipc_query(words),
                        2 => decode_ipc_request(words),
                        _ => None,
                    };
                    if let Some(request) = request {
                        handoff_bridge_to_manual_session(hwnd, state);
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
                if is_window_visible(hwnd) != 0 {
                    hide_bridge_overlay(hwnd, &mut *state_ptr);
                } else {
                    // Old CLOSE messages are harmless once dismissed. They
                    // must not revoke a newer asynchronous bridge request.
                    clear_hidden_resident_chrome(hwnd, &mut *state_ptr);
                }
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

        // The native EDIT's default limit is larger than MAX_QUERY_U16,
        // while refresh_results reads at most that many UTF-16 units.
        // Constrain real typing/paste as well as already-validated CLI/URI/IPC
        // requests so displayed text cannot silently differ from the query.
        send_message_w(state.edit, EM_LIMITTEXT, MAX_QUERY_U16 as Wparam, 0);

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
                state.detail_header,
                state.detail_name,
                state.detail_kind,
                state.detail_path,
                state.detail_open,
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
        if !state.edit.is_null() {
            // WM_DPICHANGED must scale the EDIT inset with the new font;
            // otherwise its text starts at the old logical margin.
            let margin = scale_px(14, dpi).clamp(0, u16::MAX as i32) as u32;
            let packed = margin | (margin << 16);
            send_message_w(
                state.edit,
                EM_SETMARGINS,
                EC_LEFTMARGIN | EC_RIGHTMARGIN,
                packed as Lparam,
            );
        }
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
        // The path label ends 172+70 logical pixels below the detail top.
        // The Open button is bottom-20-40. Keep 16 pixels between them;
        // otherwise short flyouts place a clickable action over file paths.
        // Collapse to the full-width results list before this can happen.
        let min_detail_height = scale_px(172, dpi)
            + scale_px(70, dpi)
            + scale_px(16, dpi)
            + scale_px(20, dpi)
            + scale_px(40, dpi);
        if bottom - top < min_detail_height {
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

    fn native_category_columns(category_space: i32, dpi: u32) -> i32 {
        let min_width = scale_px(80, dpi);
        let gap = scale_px(8, dpi);
        if category_space < 2 * min_width + gap {
            1
        } else if category_space < 4 * min_width + 3 * gap {
            2
        } else {
            4
        }
    }

    fn native_result_list_has_room(client: Rect, dpi: u32, row_height: i32) -> bool {
        if native_result_columns(client, dpi).is_some() {
            return true;
        }
        let margin = scale_px(24, dpi);
        let content_width = (client.right - client.left - margin * 2).max(1);
        // Having vertical space is not enough: a paper-thin list is not
        // readable and must not remain a keyboard Tab target.
        if content_width < scale_px(120, dpi) {
            return false;
        }
        let tab_gap = scale_px(8, dpi);
        let category_space =
            (content_width - scale_px(32, dpi).min(content_width) - tab_gap).max(1);
        let tab_height = scale_px(34, dpi);
        let category_rows = 4 / native_category_columns(category_space, dpi);
        let tab_y = scale_px(32, dpi) + scale_px(36, dpi) + scale_px(12, dpi);
        let tab_rows_height = tab_height * category_rows + tab_gap * (category_rows - 1);
        let list_y = tab_y
            + tab_rows_height
            + scale_px(12, dpi)
            + scale_px(STATUS_HEIGHT, dpi)
            + scale_px(8, dpi);
        client.bottom - list_y - margin >= scale_px(row_height, dpi).max(1)
    }

    fn classic_category_columns(content_width: i32, dpi: u32) -> i32 {
        let min_width = scale_px(94, dpi);
        let gap = scale_px(8, dpi);
        if content_width < 2 * min_width + gap {
            1
        } else if content_width < 4 * min_width + 3 * gap {
            2
        } else {
            4
        }
    }

    fn classic_category_rows_height(client: Rect, dpi: u32) -> i32 {
        let margin = scale_px(MARGIN, dpi);
        let width = (client.right - client.left - margin * 2).max(1);
        let columns = classic_category_columns(width, dpi);
        let rows = 4 / columns;
        scale_px(TAB_HEIGHT, dpi) * rows + scale_px(8, dpi) * (rows - 1)
    }

    fn classic_stacked_header_offset(client: Rect, dpi: u32) -> i32 {
        let width = (client.right - client.left - scale_px(MARGIN * 2, dpi)).max(1);
        // A 108px theme action and a legible 130px heading cannot share a
        // narrow row. Give both full width instead of clipping the title.
        if width < scale_px(260, dpi) {
            scale_px(8, dpi) + scale_px(34, dpi)
        } else {
            0
        }
    }

    fn classic_result_list_has_room(client: Rect, dpi: u32, row_height: i32) -> bool {
        let content_width = client.right - client.left - scale_px(MARGIN * 2, dpi);
        if content_width < scale_px(120, dpi) {
            return false;
        }
        let title_y = scale_px(12, dpi);
        let subtitle_y = title_y + scale_px(TITLE_HEIGHT, dpi);
        let search_y = subtitle_y
            + scale_px(18, dpi)
            + scale_px(12, dpi)
            + classic_stacked_header_offset(client, dpi);
        let tabs_y = search_y + scale_px(52, dpi) + scale_px(12, dpi);
        let status_y = tabs_y + classic_category_rows_height(client, dpi) + scale_px(10, dpi);
        let list_y = status_y + scale_px(STATUS_HEIGHT, dpi) + scale_px(6, dpi);
        client.bottom - list_y - scale_px(MARGIN, dpi) >= scale_px(row_height, dpi).max(1)
    }

    // The query field must have enough interior width to read and edit text.
    // A one-pixel native Edit is a misleading keyboard Tab stop, not an
    // accessible search input. Both themes apply this guard on WM_SIZE/DPI.
    fn query_edit_has_room(content_width: i32, border: i32, dpi: u32) -> bool {
        content_width - border * 2 >= scale_px(120, dpi)
    }

    // Height alone is not enough to expose an actionable category. The
    // native pill can use its compact 64px presentation, whereas the
    // classic label needs its existing 94px minimum to stay readable.
    fn category_chip_has_room(width: i32, minimum_logical_width: i32, dpi: u32) -> bool {
        width >= scale_px(minimum_logical_width, dpi)
    }

    // The native ellipsis fits a compact square, whereas classic Appearance
    // has a text label and must not remain an unreadable keyboard Tab stop.
    fn appearance_button_has_room(width: i32, compact: bool, dpi: u32) -> bool {
        width >= scale_px(if compact { 32 } else { 94 }, dpi)
    }

    // When native navigation is clipped, EDIT is only a valid focus fallback
    // if its entire hit area still fits. Otherwise focus the parent flyout,
    // never another hidden child. This is also used by the tiny HWND test.
    fn clipped_focus_target(edit_fits: bool, edit: Hwnd, parent: Hwnd) -> Hwnd {
        if edit_fits {
            edit
        } else {
            parent
        }
    }

    fn result_list_has_room(state: &State, client: Rect) -> bool {
        let row_height = state.theme.result_row_height();
        if state.resident
            && state.theme.preset == ThemePreset::Native
            && supports_modern_frame(state.os_build)
        {
            native_result_list_has_room(client, state.dpi, row_height)
        } else {
            classic_result_list_has_room(client, state.dpi, row_height)
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
            // A prior tiny classic layout may have hidden the EDIT. Restore
            // its Tab stop only when the native input fully fits the client.
            let edit_fits = search_top + search_height <= rect.bottom
                && query_edit_has_room(content_width, border, state.dpi);
            if !edit_fits && is_window_visible(hwnd) != 0 && get_focus() == state.edit {
                set_focus(hwnd);
            }
            show_window(state.edit, if edit_fits { SW_SHOW } else { SW_HIDE });
            let tab_y = search_top + search_height + scale_px(12, state.dpi);
            let tab_height = scale_px(34, state.dpi);
            let tab_gap = scale_px(8, state.dpi);
            let appearance_width = scale_px(32, state.dpi).min(content_width);
            let category_space = (content_width - appearance_width - tab_gap).max(1);
            let desired = [64, 100, 116, 88].map(|w| scale_px(w, state.dpi));
            let desired_total = desired.iter().sum::<i32>() + tab_gap * 3;
            let columns = native_category_columns(category_space, state.dpi);
            let category_rows = 4 / columns;
            let compact_width = ((category_space - tab_gap * (columns - 1)) / columns).max(1);
            let mut next_x = margin;
            for (index, tab) in state.tabs.iter().enumerate() {
                let (x, y, tab_width) = if columns < 4 {
                    (
                        margin + (index as i32 % columns) * (compact_width + tab_gap),
                        tab_y + (index as i32 / columns) * (tab_height + tab_gap),
                        compact_width,
                    )
                } else {
                    let width = if desired_total <= category_space {
                        desired[index]
                    } else {
                        compact_width
                    };
                    let x = next_x;
                    next_x += width + tab_gap;
                    (x, tab_y, width)
                };
                move_window(*tab, x, y, tab_width, tab_height, 1);
                // A control below the client rectangle is still WS_VISIBLE
                // unless we hide it. Keep clipped category chips out of the
                // native Tab chain and restore them when space returns.
                let fits = y >= rect.top
                    && y + tab_height <= rect.bottom
                    && category_chip_has_room(tab_width, 64, state.dpi);
                if !fits && is_window_visible(hwnd) != 0 && get_focus() == *tab {
                    set_focus(clipped_focus_target(edit_fits, state.edit, hwnd));
                }
                show_window(*tab, if fits { SW_SHOW } else { SW_HIDE });
            }
            move_window(
                state.theme_button,
                (rect.right - margin - appearance_width).max(margin),
                tab_y,
                appearance_width,
                tab_height,
                1,
            );
            let theme_fits = tab_y >= rect.top
                && tab_y + tab_height <= rect.bottom
                && appearance_button_has_room(appearance_width, true, state.dpi);
            if !theme_fits && is_window_visible(hwnd) != 0 && get_focus() == state.theme_button {
                set_focus(clipped_focus_target(edit_fits, state.edit, hwnd));
            }
            show_window(
                state.theme_button,
                if theme_fits { SW_SHOW } else { SW_HIDE },
            );
            let tab_rows_height = tab_height * category_rows + tab_gap * (category_rows - 1);
            let status_y = tab_y + tab_rows_height + scale_px(12, state.dpi);
            let status_height = scale_px(STATUS_HEIGHT, state.dpi);
            move_window(
                state.status,
                margin,
                status_y,
                content_width,
                status_height,
                1,
            );
            show_window(
                state.status,
                if status_y >= rect.top && status_y + status_height <= rect.bottom {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            let list_y = status_y + status_height + scale_px(8, state.dpi);
            let columns = native_result_columns(rect, state.dpi);
            // On small work areas or high DPI, the results can be entirely
            // below the client rectangle. Do not leave an offscreen ListBox
            // in the keyboard Tab order without space for one complete row.
            let list_has_room = result_list_has_room(state, rect);
            if let Some((left, detail)) = columns {
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
            set_result_list_visible(hwnd, state, !state.results.is_empty() && list_has_room);
            // Re-evaluate the selected detail *after* restoring ListBox
            // visibility; otherwise a window expanded from a clipped state
            // may retain empty detail text even with a valid selection.
            update_detail_controls(state);
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
        let header_offset = classic_stacked_header_offset(rect, state.dpi);
        let header_stacked = header_offset > 0;
        let search_y = subtitle_y + subtitle_height + scale_px(12, state.dpi) + header_offset;
        let tabs_y = search_y + search_height + scale_px(12, state.dpi);
        let status_y =
            tabs_y + classic_category_rows_height(rect, state.dpi) + scale_px(10, state.dpi);
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

        let heading_width = if header_stacked {
            width
        } else {
            (width - theme_width - scale_px(12, state.dpi)).max(1)
        };
        move_window(state.title, margin, title_y, heading_width, title_height, 1);
        move_window(
            state.subtitle,
            margin,
            subtitle_y,
            heading_width,
            subtitle_height,
            1,
        );
        let theme_y = if header_stacked {
            subtitle_y + subtitle_height + scale_px(8, state.dpi)
        } else {
            title_y + scale_px(4, state.dpi)
        };
        let theme_height = scale_px(34, state.dpi);
        move_window(
            state.theme_button,
            if header_stacked {
                margin
            } else {
                margin + (width - theme_width).max(0)
            },
            theme_y,
            if header_stacked { width } else { theme_width },
            theme_height,
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
        let theme_actual_width = if header_stacked { width } else { theme_width };
        let theme_fits = theme_y + theme_height <= rect.bottom
            && appearance_button_has_room(
                theme_actual_width,
                state.theme.preset == ThemePreset::Native,
                state.dpi,
            );
        let edit_fits = search_y + search_height <= rect.bottom
            && query_edit_has_room(width, search_border, state.dpi);
        // Return focus to the remaining reachable control. At very short
        // heights neither may fit; focus the parent rather than an offscreen
        // child, and never alter focus for an invisible synthetic window.
        if is_window_visible(hwnd) != 0 {
            let focused = get_focus();
            if !edit_fits && focused == state.edit {
                set_focus(if theme_fits { state.theme_button } else { hwnd });
            } else if !theme_fits && focused == state.theme_button {
                set_focus(if edit_fits { state.edit } else { hwnd });
            }
        }
        show_window(state.edit, if edit_fits { SW_SHOW } else { SW_HIDE });
        show_window(
            state.theme_button,
            if theme_fits { SW_SHOW } else { SW_HIDE },
        );

        let tab_gap = scale_px(8, state.dpi);
        let columns = classic_category_columns(width, state.dpi);
        let tab_width = if columns == 4 {
            scale_px(94, state.dpi)
        } else {
            ((width - tab_gap * (columns - 1)) / columns).max(1)
        };
        for (index, tab) in state.tabs.iter().enumerate() {
            let x = margin + (index as i32 % columns) * (tab_width + tab_gap);
            let y = tabs_y + (index as i32 / columns) * (tab_height + tab_gap);
            move_window(*tab, x, y, tab_width, tab_height, 1);
            let fits = y >= rect.top
                && y + tab_height <= rect.bottom
                && category_chip_has_room(tab_width, 94, state.dpi);
            if !fits && is_window_visible(hwnd) != 0 && get_focus() == *tab {
                set_focus(clipped_focus_target(edit_fits, state.edit, hwnd));
            }
            show_window(*tab, if fits { SW_SHOW } else { SW_HIDE });
        }
        move_window(state.status, margin, status_y, width, status_height, 1);
        show_window(
            state.status,
            if status_y >= rect.top && status_y + status_height <= rect.bottom {
                SW_SHOW
            } else {
                SW_HIDE
            },
        );
        move_window(state.list, margin, list_y, width, list_height, 1);
        let list_visible = !state.results.is_empty() && result_list_has_room(state, rect);
        set_result_list_visible(hwnd, state, list_visible);
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

    // Cache-preservation checks may read a hidden ListBox, but must still
    // validate its native count, selected item-data and accessibility label.
    // Never use this function to authorize Open or populate visible details.
    unsafe fn verified_cached_selected_row(state: &State) -> Option<&ResultRow> {
        if !native_result_count_matches(state) {
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

    unsafe fn selected_detail_row(state: &State) -> Option<&ResultRow> {
        // Hidden results must never expose actionable file details, even
        // though a matching selected row may be retained for refresh.
        if get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE == 0 {
            return None;
        }
        verified_cached_selected_row(state)
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

    // The native detail Open button is an actual Tab stop. When its card
    // disappears, focus must move to a still-accessible result list, the
    // query field, or finally the parent if neither child is visible.
    fn hidden_detail_focus_target(
        list_visible: bool,
        edit_visible: bool,
        list: Hwnd,
        edit: Hwnd,
        parent: Hwnd,
    ) -> Hwnd {
        if list_visible {
            list
        } else {
            clipped_focus_target(edit_visible, edit, parent)
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
        let detail_visible = row.is_some();
        if !detail_visible
            && !parent.is_null()
            && is_window_visible(parent) != 0
            && get_focus() == state.detail_open
        {
            let list_visible =
                get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE != 0;
            let edit_visible =
                get_window_long_ptr_w(state.edit, GWL_STYLE) as u32 & WS_VISIBLE != 0;
            set_focus(hidden_detail_focus_target(
                list_visible,
                edit_visible,
                state.list,
                state.edit,
                parent,
            ));
        }
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
            show_window(control, if detail_visible { SW_SHOW } else { SW_HIDE });
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

    // The visible native-button handler and hidden synthetic tests share
    // one category transition without fabricating a click on a hidden HWND.
    unsafe fn select_search_category(state: &mut State, mode: SearchMode) {
        if state.mode == mode {
            return;
        }
        state.mode = mode;
        update_tab_labels(state);
        refresh_results(state);
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

    // Whenever a visible flyout hides its focused ListBox, transfer focus
    // to EDIT only when EDIT is also visible. With two clipped controls,
    // focus the parent rather than an inaccessible child. This applies
    // to WM_SIZE and query refresh in both native and classic themes.
    // A hidden/offscreen test HWND cannot steal the physical desktop focus.
    unsafe fn set_result_list_visible(parent: Hwnd, state: &State, visible: bool) {
        if !visible
            && !parent.is_null()
            && should_restore_query_focus(is_window_visible(parent) != 0, get_focus() == state.list)
        {
            let edit_visible =
                get_window_long_ptr_w(state.edit, GWL_STYLE) as u32 & WS_VISIBLE != 0;
            set_focus(clipped_focus_target(edit_visible, state.edit, parent));
        }
        show_window(state.list, if visible { SW_SHOW } else { SW_HIDE });
    }

    // Toggle the results list only after the synchronous query settles.
    // In every theme query refresh and WM_SIZE honor the same row space;
    // a clipped list must never reenter keyboard traversal after a requery.
    // Never give focus to a hidden self-test window or an inactive popup.
    unsafe fn update_result_list_visibility(state: &State, has_results: bool) {
        let parent = get_parent(state.list);
        if has_results {
            let mut client = Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            let has_room = !parent.is_null()
                && get_client_rect(parent, &mut client) != 0
                && result_list_has_room(state, client);
            set_result_list_visible(parent, state, has_room);
        } else {
            set_result_list_visible(parent, state, false);
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
            verified_cached_selected_row(state).map(|row| (row.path.clone(), row.is_directory))
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
            update_result_list_visibility(state, false);
            return;
        }
        let mut buffer = vec![0_u16; len as usize + 1];
        let copied = get_window_text_w(state.edit, buffer.as_mut_ptr(), len + 1);
        if copied <= 0 {
            update_result_list_visibility(state, false);
            return;
        }
        let query = String::from_utf16_lossy(&buffer[..copied as usize]);
        let query = query.trim();
        if query.is_empty() {
            set_idle_status(state);
            update_result_list_visibility(state, false);
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
            update_result_list_visibility(state, false);
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
            update_result_list_visibility(state, false);
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
        // The result count and the current layout decide visibility in every
        // theme. Native selection, however, is intentionally separate from
        // the classic theme's unselected-on-requery behavior.
        update_result_list_visibility(state, count > 0);
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

    fn verified_draw_result_index(
        item_id: u32,
        item_data: usize,
        result_count: usize,
    ) -> Option<usize> {
        (usize::try_from(item_id).ok() == Some(item_data) && item_data < result_count)
            .then_some(item_data)
    }

    unsafe fn verified_draw_result_row(
        state: &State,
        item_id: u32,
        item_data: usize,
    ) -> Option<&ResultRow> {
        let index = verified_draw_result_index(item_id, item_data, state.results.len())?;
        // WM_DRAWITEM's slot/index pair is necessary but not sufficient.
        // A same-count label replacement must not paint a cached file whose
        // name and path differ from the ListBox's accessible item string.
        if !native_result_count_matches(state) {
            return None;
        }
        let row = state.results.get(index)?;
        let expected = verified_result_accessible_label(row)?;
        native_result_label_matches(state.list, index, &expected).then_some(row)
    }

    fn result_row_text_geometry(card: &Rect, dpi: u32, high_contrast: bool) -> (bool, Rect, Rect) {
        // In narrow one-column flyouts the badge and icon would leave as
        // little as four pixels for the file name. Prioritize title and path;
        // the complete name, type and path remain in the native ListBox label.
        let decorated = card.right - card.left >= scale_px(260, dpi);
        let text_inset = if decorated {
            if high_contrast {
                14
            } else {
                36
            }
        } else {
            12
        };
        let title = Rect {
            left: card.left + scale_px(text_inset, dpi),
            top: card.top + scale_px(6, dpi),
            right: if decorated {
                (card.right - scale_px(86, dpi)).max(card.left + scale_px(40, dpi))
            } else {
                card.right - scale_px(12, dpi)
            },
            bottom: card.top + scale_px(32, dpi),
        };
        let path = Rect {
            left: card.left + scale_px(text_inset, dpi),
            top: card.top + scale_px(31, dpi),
            right: card.right - scale_px(12, dpi),
            bottom: card.bottom - scale_px(5, dpi),
        };
        (decorated, title, path)
    }

    unsafe fn draw_result_row(state: &State, draw: &DrawItemStruct) {
        // Clear invalid rows rather than leaving stale pixels from a former
        // result. Painting may never substitute another ListBox item's data.
        fill_rect(draw.hdc, &draw.rc_item, state.background_brush);
        let Some(row) = verified_draw_result_row(state, draw.item_id, draw.item_data) else {
            return;
        };
        let selected = draw.item_state & ODS_SELECTED != 0;

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

        let (show_decorations, mut title_rect, mut path_rect) =
            result_row_text_geometry(&card, state.dpi, state.high_contrast);
        let icon_key = shell_icon_key(&row.name, row.is_directory);
        if show_decorations && !state.high_contrast {
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
        if show_decorations {
            draw_text_w(
                draw.hdc,
                badge.as_ptr(),
                -1,
                &mut badge_rect,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }

        let path = wide(&row.path);
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

    // A resident Search window may have queued keyboard input after it was
    // dismissed; do not route those keys to invisible native child controls.
    fn should_drop_dismissed_popup_input(
        resident: bool,
        parent_visible: bool,
        message: u32,
        belongs_to_popup: bool,
    ) -> bool {
        resident
            && !parent_visible
            && belongs_to_popup
            && matches!(
                message,
                WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP
                    | 0x0102 // WM_CHAR
                    | 0x0103 // WM_DEADCHAR
                    | 0x0106 // WM_SYSCHAR
                    | 0x0107 // WM_SYSDEADCHAR
                    | 0x0109 // WM_UNICHAR
                    | 0x0286 // WM_IME_CHAR
            )
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

    // Visible native LBN_SELCHANGE and the hidden synthetic corruption
    // fixture share the same row-integrity and detail-update logic. Only
    // the actual window message requires a visible parent and ListBox.
    unsafe fn process_list_selection_change(state: &mut State) {
        // A row whose native item-data or text no longer matches the cache
        // cannot stay selected for keyboard and accessibility consumers.
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
        if get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE == 0 {
            // A clipped result must not become keyboard-focusable, but a
            // harmless Down key must not erase its verified cached selection.
            return false;
        }
        if !native_result_count_matches(state) {
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
        if get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE == 0 {
            // A clipped result must not become keyboard-focusable, but a
            // harmless Down key must not erase its verified cached selection.
            return false;
        }
        if !native_result_count_matches(state) {
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
        // Keyboard/bridge Enter can arrive when the flyout has too little
        // space to display results. Never launch a hidden row or revoke the
        // cached selection merely because it is temporarily not actionable.
        if get_window_long_ptr_w(state.list, GWL_STYLE) as u32 & WS_VISIBLE == 0 {
            return false;
        }
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
            dismiss_resident_after_open(hwnd, state);
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

    // A committed native EDIT event and the synthetic hidden-window fixture
    // share one IME-aware search refresh. Production WM_COMMAND applies the
    // additional parent-visibility guard before reaching this routine.
    unsafe fn refresh_from_committed_edit_change(state: &mut State) {
        // Partial/preedit IME text is not a committed search query. The
        // native subclass refreshes when composition is finalized.
        if !state.ime_composing && !state.programmatic_edit_update {
            refresh_results(state);
        }
    }

    // Hidden Win32 EDIT controls do not always emit automatic EN_CHANGE for
    // SetWindowTextW. Exercise the same committed query-processing routine
    // directly without pretending that the hidden parent was clicked/typed.
    unsafe fn drive_hidden_edit_change(
        _hwnd: Hwnd,
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
        refresh_from_committed_edit_change(&mut *state_ptr);
        Ok(())
    }

    unsafe fn run_hidden_ui_regression(hwnd: Hwnd, state_ptr: *mut State) -> io::Result<()> {
        require_ui_selftest(is_window_visible(hwnd) == 0, "parent must stay hidden")?;
        // Verify the *actual native EDIT* limits user typing and paste to the
        // same UTF-16 budget used by CLI/URI/IPC and refresh_results. This
        // never-shown fixture does not type into or focus the real desktop.
        require_ui_selftest(
            send_message_w((*state_ptr).edit, EM_GETLIMITTEXT, 0, 0) == MAX_QUERY_U16 as Lresult,
            "native EDIT text limit differs from indexed-query UTF-16 budget",
        )?;
        // EM_REPLACESEL follows the same native limit as user paste, unlike
        // WM_SETTEXT. Exercise it without SendInput, focus, or visible UI.
        let oversized = wide(&"x".repeat(MAX_QUERY_U16 as usize + 1));
        send_message_w(
            (*state_ptr).edit,
            EM_REPLACESEL,
            1,
            oversized.as_ptr() as Lparam,
        );
        require_ui_selftest(
            get_window_text_length_w((*state_ptr).edit) == MAX_QUERY_U16,
            "native EDIT accepted more than the indexed query UTF-16 budget",
        )?;
        require_ui_selftest(
            set_window_text_w((*state_ptr).edit, wide("").as_ptr()) != 0,
            "native EDIT failed to restore empty synthetic query",
        )?;
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

        // Exercise the native EDIT and shared committed-query handler.
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).results_label) == accessible_results_name(3),
            "initial three search results did not update accessibility count",
        )?;
        // Exercise a complete non-BMP Unicode scalar through the same native
        // EDIT replacement path as the Shell bridge, never the real keyboard.
        // Backspace must erase the entire surrogate pair as one character.
        let edit = (*state_ptr).edit;
        let caret = read_control_text_for_test(edit).encode_utf16().count() as u32;
        set_bridge_selection(edit, caret, caret as i32);
        append_bridge_unicode_scalar(edit, 0x1F4C1);
        require_ui_selftest(
            read_control_text_for_test(edit) == "SearchTool📁"
                && (*state_ptr).results.len() == 3
                && is_window_visible(hwnd) == 0,
            "Unicode scalar bridge edit did not append one complete emoji",
        )?;
        append_bridge_unicode_scalar(edit, 0xD83D);
        require_ui_selftest(
            read_control_text_for_test(edit) == "SearchTool📁",
            "Unicode scalar bridge edit accepted an isolated high surrogate",
        )?;
        append_bridge_unicode_scalar(edit, VK_BACK);
        require_ui_selftest(
            read_control_text_for_test(edit) == "SearchTool" && (*state_ptr).results.len() == 3,
            "bridge Backspace did not erase complete Unicode surrogate pair",
        )?;
        // Keyboard input may remain queued for the native EDIT/ListBox when
        // the resident parent is dismissed. The outer GetMessage loop must
        // not route delayed Enter, Tab or character input to hidden children.
        require_ui_selftest(
            get_parent((*state_ptr).edit) == hwnd
                && get_parent((*state_ptr).list) == hwnd
                && is_window_visible(hwnd) == 0
                && should_drop_dismissed_popup_input(
                    (*state_ptr).resident,
                    false,
                    WM_KEYDOWN,
                    true,
                )
                && should_drop_dismissed_popup_input(
                    (*state_ptr).resident,
                    false,
                    0x0102, // WM_CHAR
                    true,
                ),
            "dismissed resident popup still routes queued keyboard input",
        )?;
        // Malformed external WM_COPYDATA can have a null payload header.
        // The never-shown HWND must not crash, open or alter a Search session.
        let null_copy_status = send_message_w(hwnd, WM_COPYDATA, 0, 0);
        require_ui_selftest(
            null_copy_status == 0
                && is_window_visible(hwnd) == 0
                && (*state_ptr).results.len() == 3
                && read_control_text_for_test((*state_ptr).edit) == "SearchTool",
            "null WM_COPYDATA header unexpectedly altered the hidden Search session",
        )?;
        // A type-2 IPC request needs two UTF-16 NUL-terminated fields.
        // Never activate even the isolated test window for a truncated
        // query/scope packet: it might otherwise steal real desktop focus.
        for malformed in [vec![0_u16], vec!['q' as u16, 0, 'C' as u16], vec![0, 0, 0]] {
            let request = CopyDataStruct {
                dw_data: 2,
                cb_data: (malformed.len() * 2) as u32,
                lp_data: malformed.as_ptr().cast(),
            };
            require_ui_selftest(
                send_message_w(
                    hwnd,
                    WM_COPYDATA,
                    0,
                    (&request as *const CopyDataStruct) as Lparam,
                ) == 0
                    && is_window_visible(hwnd) == 0
                    && (*state_ptr).results.len() == 3
                    && read_control_text_for_test((*state_ptr).edit) == "SearchTool",
                "malformed query/scope IPC packet activated a hidden resident popup",
            )?;
        }
        // The older type-1 IPC form also needs fail-closed UTF-16 and a
        // single optional terminator. Exercise its real native message
        // receiver with corrupt input, never with an actual open desktop.
        for malformed in [
            vec![0xD800_u16],
            vec![b'a' as u16, 0, b'b' as u16],
            vec![b'a' as u16, 0, 0],
        ] {
            let request = CopyDataStruct {
                dw_data: 1,
                cb_data: (malformed.len() * 2) as u32,
                lp_data: malformed.as_ptr().cast(),
            };
            require_ui_selftest(
                send_message_w(
                    hwnd,
                    WM_COPYDATA,
                    0,
                    (&request as *const CopyDataStruct) as Lparam,
                ) == 0
                    && is_window_visible(hwnd) == 0
                    && (*state_ptr).results.len() == 3
                    && read_control_text_for_test((*state_ptr).edit) == "SearchTool",
                "malformed legacy IPC query activated a hidden Search popup",
            )?;
        }
        // A nonempty but unusable Explorer scope must not degrade into an
        // unscoped IPC Search and reveal global results. Send the actual
        // Win32 COPYDATA packet only to this never-shown synthetic HWND.
        for invalid_scope in [
            r"\",
            "\"",
            "  ",
            "C:",
            "C:Private",
            "Private",
            r"\Private",
            r"\\server",
            r"C:\Private\..\Public",
            r"C:\Private\.\Reports",
            r"\\server\share\..\other",
        ] {
            let mut payload = "secret".encode_utf16().collect::<Vec<u16>>();
            payload.push(0);
            payload.extend(invalid_scope.encode_utf16());
            payload.push(0);
            let request = CopyDataStruct {
                dw_data: 2,
                cb_data: (payload.len() * 2) as u32,
                lp_data: payload.as_ptr().cast(),
            };
            require_ui_selftest(
                send_message_w(
                    hwnd,
                    WM_COPYDATA,
                    0,
                    (&request as *const CopyDataStruct) as Lparam,
                ) == 0
                    && is_window_visible(hwnd) == 0
                    && (*state_ptr).results.len() == 3
                    && read_control_text_for_test((*state_ptr).edit) == "SearchTool",
                "invalid Explorer scope silently activated global Search",
            )?;
        }
        // A native EDIT can queue an old EN_SETFOCUS after its resident
        // parent is dismissed. Never mark an invisible popup's search box
        // focused or repaint its focused border for the next session.
        (*state_ptr).edit_focused = false;
        require_ui_selftest(
            is_window_visible(hwnd) == 0
                && get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "hidden EDIT focus test requires resident parent hidden, child style visible",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_EDIT | (EN_SETFOCUS << 16),
            (*state_ptr).edit as Lparam,
        );
        require_ui_selftest(
            !(*state_ptr).edit_focused,
            "dismissed Search parent accepted delayed EDIT focus notification",
        )?;
        // A child EDIT can retain its WS_VISIBLE style after its resident
        // parent is dismissed. A queued old EN_CHANGE should not rerun the
        // index or overwrite the next session's cached status.
        let hidden_edit_marker = "Hidden parent must ignore stale EN_CHANGE";
        set_status(&*state_ptr, hidden_edit_marker);
        require_ui_selftest(
            is_window_visible(hwnd) == 0
                && get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "hidden EDIT event fixture needs a dismissed parent and visible-style child",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_EDIT | (EN_CHANGE << 16),
            (*state_ptr).edit as Lparam,
        );
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).status) == hidden_edit_marker
                && (*state_ptr).results.len() == 3,
            "dismissed Search parent accepted stale EN_CHANGE and repeated the index query",
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
        // A dismissed resident parent cannot receive real clicks even if
        // the native child still retains its WS_VISIBLE style. A queued
        // old BN_CLICKED must not switch the next session's category.
        let hidden_parent_mode = (*state_ptr).mode;
        let hidden_parent_category_status = read_control_text_for_test((*state_ptr).status);
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).tabs[1], GWL_STYLE) as u32 & WS_VISIBLE != 0
                && is_window_visible(hwnd) == 0,
            "stale category click fixture requires visible child style and hidden parent",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_FILES | (BN_CLICKED << 16),
            (*state_ptr).tabs[1] as Lparam,
        );
        require_ui_selftest(
            (*state_ptr).mode == hidden_parent_mode
                && read_control_text_for_test((*state_ptr).status) == hidden_parent_category_status,
            "hidden parent accepted stale category click and triggered requery",
        )?;
        // Exercise the same category transition code as the native handler,
        // without pretending a hidden parent has received a real user click.
        select_search_category(&mut *state_ptr, SearchMode::Files);
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
        select_search_category(&mut *state_ptr, SearchMode::Files);
        require_ui_selftest(
            (*state_ptr).mode == SearchMode::Files
                && (*state_ptr).results.len() == files_count
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == selected_files_row
                && read_control_text_for_test((*state_ptr).detail_path) == files_detail
                && read_control_text_for_test((*state_ptr).status) == noop_marker
                && read_control_text_for_test((*state_ptr).tabs[1]) == "Dosyalar (seçili)",
            "reselecting active Files category needlessly refreshed results",
        )?;
        select_search_category(&mut *state_ptr, SearchMode::All);
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
        // A WS_VISIBLE child inside an invisible parent retains the bit but
        // cannot be clicked by a person. Old queued BN_CLICKED / LBN_DBLCLK
        // notifications must not open stale results after the flyout closes.
        let hidden_parent_status = read_control_text_for_test((*state_ptr).status);
        let hidden_parent_path = read_control_text_for_test((*state_ptr).detail_path);
        let hidden_parent_selection = send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0);
        require_ui_selftest(
            is_window_visible(hwnd) == 0
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    != 0
                && hidden_parent_selection == 0,
            "hidden parent Open fixture requires a visible-style selected child",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_DETAIL_OPEN | (BN_CLICKED << 16),
            (*state_ptr).detail_open as Lparam,
        );
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).status) == hidden_parent_status
                && read_control_text_for_test((*state_ptr).detail_path) == hidden_parent_path
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == hidden_parent_selection,
            "hidden parent accepted stale Open click despite visible child style",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_LIST | (LBN_DBLCLK << 16),
            (*state_ptr).list as Lparam,
        );
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).status) == hidden_parent_status
                && read_control_text_for_test((*state_ptr).detail_path) == hidden_parent_path
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == hidden_parent_selection,
            "hidden parent accepted stale ListBox double-click",
        )?;

        // A selected ListBox child may retain WS_VISIBLE when its parent is
        // dismissed. Old queued LBN_SELCHANGE must not clear the previous
        // selection or detail if its cached row becomes temporarily invalid.
        require_ui_selftest(
            is_window_visible(hwnd) == 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 1) >= 0
                && selected_detail_row(&*state_ptr).is_none(),
            "hidden ListBox selection fixture could not invalidate its selected item mapping",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_LIST | (LBN_SELCHANGE << 16),
            (*state_ptr).list as Lparam,
        );
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == hidden_parent_selection
                && read_control_text_for_test((*state_ptr).detail_path) == hidden_parent_path
                && read_control_text_for_test((*state_ptr).status) == hidden_parent_status,
            "hidden parent accepted stale ListBox selection-change notification",
        )?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 0) >= 0,
            "hidden ListBox mapping fixture could not restore its original item mapping",
        )?;

        // Queued Shell bridge character input must not mutate the query
        // after a resident Search popup has already been dismissed.
        let hidden_query = read_control_text_for_test((*state_ptr).edit);
        send_message_w(hwnd, WM_SHELL_BRIDGE_CHAR, b'X' as usize, 0);
        require_ui_selftest(
            read_control_text_for_test((*state_ptr).edit) == hidden_query,
            "hidden Search popup accepted a stale Shell bridge character",
        )?;

        // Resize only this invisible test HWND across the compact-height
        // threshold. The native results remain available and the detail
        // card must disappear *including its cached accessible file path*;
        // resizing back must restore the selected card without a new search.
        let mut original_window = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut original_client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        require_ui_selftest(
            get_window_rect(hwnd, &mut original_window) != 0
                && get_client_rect(hwnd, &mut original_client) != 0,
            "cannot measure hidden flyout before compact layout regression",
        )?;
        let outer_height = original_window.bottom - original_window.top;
        let non_client_height = outer_height - (original_client.bottom - original_client.top);
        let compact_height = scale_px(470, (*state_ptr).dpi) + non_client_height;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                original_window.right - original_window.left,
                compact_height,
                SWP_NOZORDER,
            ) != 0,
            "cannot resize hidden flyout to compact height",
        )?;
        let mut compact_client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        require_ui_selftest(
            get_client_rect(hwnd, &mut compact_client) != 0
                && native_result_columns(compact_client, (*state_ptr).dpi).is_none()
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty(),
            "compact layout retained overlapping or actionable detail controls",
        )?;
        require_ui_selftest(
            get_next_dlg_tab_item(hwnd, (*state_ptr).list, 0) == (*state_ptr).theme_button
                && get_next_dlg_tab_item(hwnd, (*state_ptr).theme_button, 0) == (*state_ptr).edit
                && get_next_dlg_tab_item(hwnd, (*state_ptr).edit, 1) == (*state_ptr).theme_button,
            "compact layout Tab traversal reached hidden detail Open action",
        )?;
        // The synthetic result path is deliberately absent on disk. A queued
        // click from the Open HWND, once hidden by layout, must not attempt
        // any file operation or clear its otherwise valid selection/status.
        let compact_selection = send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0);
        require_ui_selftest(
            compact_selection == 0
                && !selected_path_still_openable(
                    &(&(*state_ptr).results)[0].path,
                    (&(*state_ptr).results)[0].is_directory,
                ),
            "hidden Open event fixture requires a nonexistent synthetic path",
        )?;
        let compact_status = read_control_text_for_test((*state_ptr).status);
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_DETAIL_OPEN | (BN_CLICKED << 16),
            (*state_ptr).detail_open as Lparam,
        );
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == compact_selection
                && read_control_text_for_test((*state_ptr).status) == compact_status
                && (*state_ptr).results.len() == 3,
            "hidden Open action accepted a stale BN_CLICKED command",
        )?;
        // In a severely constrained work area the results list can have
        // zero room for even one row. A clipped but WS_VISIBLE ListBox
        // must not remain keyboard-focusable below the client rectangle.
        let tiny_height = scale_px(170, (*state_ptr).dpi) + non_client_height;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                original_window.right - original_window.left,
                tiny_height,
                SWP_NOZORDER,
            ) != 0,
            "cannot resize hidden flyout to one-row-unavailable height",
        )?;
        let mut tiny_client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        require_ui_selftest(
            get_client_rect(hwnd, &mut tiny_client) != 0
                && native_result_columns(tiny_client, (*state_ptr).dpi).is_none()
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0)
                    == (*state_ptr).theme_button
                && read_control_text_for_test((*state_ptr).detail_path).is_empty(),
            "too-short flyout exposes offscreen ListBox in keyboard Tab order",
        )?;
        // ListBox commands can be queued before a WM_SIZE hides its HWND.
        // Once clipped, a stale selection/double-click event must not clear
        // the cached best match or mutate the visible status. No files are
        // opened: the fixture uses only nonexistent synthetic paths.
        let hidden_list_selection = send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0);
        let hidden_list_status = read_control_text_for_test((*state_ptr).status);
        require_ui_selftest(
            hidden_list_selection == 0
                && !selected_path_still_openable(
                    &(&(*state_ptr).results)[0].path,
                    (&(*state_ptr).results)[0].is_directory,
                ),
            "hidden list command test requires a selected synthetic missing file",
        )?;
        for notification in [LBN_SELCHANGE, LBN_DBLCLK] {
            send_message_w(
                hwnd,
                WM_COMMAND,
                ID_LIST | (notification << 16),
                (*state_ptr).list as Lparam,
            );
            require_ui_selftest(
                send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == hidden_list_selection
                    && read_control_text_for_test((*state_ptr).status) == hidden_list_status
                    && (*state_ptr).results.len() == 3,
                "hidden ListBox processed a stale selection or double-click notification",
            )?;
        }
        // A hidden native ListBox still has a valid cached selection. An
        // identical query refresh must preserve that file, not silently
        // substitute the first result merely because the list is clipped.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETCURSEL, 1, 0) == 1,
            "could not select second cached result in hidden native list",
        )?;
        // Down and Enter in the query must not focus/open a hidden ListBox,
        // nor revoke a verified cached result while there is no row space.
        // The synthetic selected file is confirmed missing on disk, so no
        // actual file or application can be launched from this fixture.
        require_ui_selftest(
            !selected_path_still_openable(
                &(&(*state_ptr).results)[1].path,
                (&(*state_ptr).results)[1].is_directory,
            ),
            "hidden key fixture requires a nonexistent selected file",
        )?;
        let hidden_key_status = read_control_text_for_test((*state_ptr).status);
        require_ui_selftest(
            !prepare_query_down_selection(&*state_ptr)
                && !prepare_search_enter_selection(&*state_ptr, true)
                && !open_selected(hwnd, &mut *state_ptr)
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 1
                && verified_cached_selected_row(&*state_ptr).is_some()
                && selected_detail_row(&*state_ptr).is_none()
                && read_control_text_for_test((*state_ptr).status) == hidden_key_status,
            "clipped native Down or Enter revoked a verified cached selection",
        )?;
        // The Shell bridge's Enter bypasses the ordinary GetMessage
        // keyboard route. Exercise its real WM_COMMAND-adjacent handler
        // using only a synthetic HWND message, not physical key input.
        send_message_w(hwnd, WM_SHELL_BRIDGE_KEY, VK_RETURN, 0);
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 1
                && verified_cached_selected_row(&*state_ptr).is_some()
                && read_control_text_for_test((*state_ptr).status) == hidden_key_status
                && read_control_text_for_test((*state_ptr).detail_path).is_empty(),
            "Shell bridge Enter revoked a clipped native result selection",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 1
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "clipped native query refresh changed the previously selected file",
        )?;
        require_ui_selftest(
            verified_cached_selected_row(&*state_ptr).is_some()
                && selected_detail_row(&*state_ptr).is_none()
                && read_control_text_for_test((*state_ptr).detail_path).is_empty(),
            "hidden cached selection became actionable after query refresh",
        )?;
        // A hidden selection with the wrong native item-data is not safe to
        // restore: a requery must reject it and choose a verified Best match.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETITEMDATA, 1, 0) >= 0
                && verified_cached_selected_row(&*state_ptr).is_none(),
            "could not corrupt hidden selection mapping for fail-closed check",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                && verified_cached_selected_row(&*state_ptr).is_some()
                && selected_detail_row(&*state_ptr).is_none(),
            "corrupted hidden selection was reused after query refresh",
        )?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0,
            "could not restore first cached result for subsequent fixture checks",
        )?;
        // A new query while the flyout remains clipped must not undo the
        // WM_SIZE visibility decision and reintroduce hidden Tab targets.
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0)
                    == (*state_ptr).theme_button
                && read_control_text_for_test((*state_ptr).detail_path).is_empty(),
            "query refresh reopened offscreen ListBox in too-short flyout",
        )?;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                original_window.right - original_window.left,
                outer_height,
                SWP_NOZORDER,
            ) != 0,
            "cannot restore hidden flyout after compact layout regression",
        )?;
        let mut restored_client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        require_ui_selftest(
            get_client_rect(hwnd, &mut restored_client) != 0
                && native_result_columns(restored_client, (*state_ptr).dpi).is_some()
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                && read_control_text_for_test((*state_ptr).detail_name) == initial
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path
                && get_window_long_ptr_w((*state_ptr).detail_open, GWL_STYLE) as u32 & WS_VISIBLE
                    != 0
                && get_next_dlg_tab_item(hwnd, (*state_ptr).theme_button, 0)
                    == (*state_ptr).detail_open
                && is_window_visible(hwnd) == 0,
            "restoring two-column layout lost selected accessible details",
        )?;
        // A tall but almost zero-width flyout can technically fit result
        // rows vertically while being far too thin to render a file name.
        // Exclude such an unusable ListBox from the Tab chain.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(130, (*state_ptr).dpi) + (original_window.right - original_window.left)
                    - (original_client.right - original_client.left),
                outer_height,
                SWP_NOZORDER,
            ) != 0
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "very narrow native flyout retained an unreadable result Tab target",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "very narrow native flyout retained an unreadable query Tab target",
        )?;
        require_ui_selftest(
            (*state_ptr)
                .tabs
                .iter()
                .all(|tab| get_window_long_ptr_w(*tab, GWL_STYLE) as u32 & WS_VISIBLE == 0),
            "very narrow native flyout retained unreadable category Tab targets",
        )?;
        // Even if a pre-resize click was queued, a now-hidden narrow chip
        // cannot change the active filter or cached search results.
        let narrow_native_mode = (*state_ptr).mode;
        let (narrow_native_chip, narrow_native_id) = if narrow_native_mode == SearchMode::Folders {
            ((*state_ptr).tabs[3], ID_CONTENT)
        } else {
            ((*state_ptr).tabs[2], ID_FOLDERS)
        };
        send_message_w(
            hwnd,
            WM_COMMAND,
            narrow_native_id | (BN_CLICKED << 16),
            narrow_native_chip as Lparam,
        );
        require_ui_selftest(
            (*state_ptr).mode == narrow_native_mode && (*state_ptr).results.len() == 3,
            "width-clipped native category accepted a stale click",
        )?;
        // Below the native 32px ellipsis minimum the Appearance button
        // must also leave the Tab chain. The synthetic parent stays hidden.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(60, (*state_ptr).dpi) + (original_window.right - original_window.left)
                    - (original_client.right - original_client.left),
                outer_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && (*state_ptr).results.len() == 3,
            "too-narrow native Appearance icon remained in keyboard Tab order",
        )?;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                original_window.right - original_window.left,
                outer_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0,
            "restoring width did not restore native result list",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "restoring width did not restore native query Tab target",
        )?;
        require_ui_selftest(
            (*state_ptr)
                .tabs
                .iter()
                .all(|tab| get_window_long_ptr_w(*tab, GWL_STYLE) as u32 & WS_VISIBLE != 0),
            "restoring width did not restore native category Tab targets",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "restoring width did not restore native Appearance icon",
        )?;

        // Native->classic theme transitions must recover a list that a tiny
        // native flyout had hidden. Change only in-memory fixture state, no
        // user theme preference or installed package is ever modified.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                original_window.right - original_window.left,
                tiny_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "cannot prepare hidden native list before classic theme transition",
        )?;
        (*state_ptr).theme.preset = ThemePreset::Graphite;
        resize_controls(hwnd, &mut *state_ptr);
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0)
                    == (*state_ptr).theme_button,
            "clipped classic theme requery reopened an unusable native result list",
        )?;
        // The classic ListBox must also ignore old selection/double-click
        // commands after its last complete row has been clipped. Set only a
        // synthetic cached selection; no physical user input or file launch.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETCURSEL, 0, 0) == 0,
            "could not prepare classic hidden ListBox selection",
        )?;
        let classic_hidden_status = read_control_text_for_test((*state_ptr).status);
        for notification in [LBN_SELCHANGE, LBN_DBLCLK] {
            send_message_w(
                hwnd,
                WM_COMMAND,
                ID_LIST | (notification << 16),
                (*state_ptr).list as Lparam,
            );
            require_ui_selftest(
                send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                    && read_control_text_for_test((*state_ptr).status) == classic_hidden_status
                    && (*state_ptr).results.len() == 3,
                "hidden classic ListBox processed a stale selection or double-click",
            )?;
        }
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                original_window.right - original_window.left,
                outer_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0) == (*state_ptr).list
                && read_control_text_for_test((*state_ptr).detail_path).is_empty(),
            "switching from clipped native to classic theme lost cached result list",
        )?;
        // Classic themes must also keep all four filter buttons inside a
        // narrow client rather than letting the last tabs spill offscreen.
        let classic_nonclient_width = (original_window.right - original_window.left)
            - (original_client.right - original_client.left);
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(360, (*state_ptr).dpi) + classic_nonclient_width,
                outer_height,
                SWP_NOZORDER,
            ) != 0,
            "could not resize classic flyout to narrow client width",
        )?;
        let mut classic_first = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut classic_second = classic_first;
        let mut classic_third = classic_first;
        let mut classic_fourth = classic_first;
        let mut classic_status = classic_first;
        require_ui_selftest(
            get_window_rect((*state_ptr).tabs[0], &mut classic_first) != 0
                && get_window_rect((*state_ptr).tabs[1], &mut classic_second) != 0
                && get_window_rect((*state_ptr).tabs[2], &mut classic_third) != 0
                && get_window_rect((*state_ptr).tabs[3], &mut classic_fourth) != 0
                && get_window_rect((*state_ptr).status, &mut classic_status) != 0
                && classic_first.top == classic_second.top
                && classic_third.top == classic_fourth.top
                && classic_third.top >= classic_first.bottom + scale_px(8, (*state_ptr).dpi)
                && classic_first.right < classic_second.left
                && classic_third.right < classic_fourth.left
                && classic_status.top > classic_fourth.bottom
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3,
            "classic narrow-width filters did not wrap to two rows",
        )?;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(360, (*state_ptr).dpi) + classic_nonclient_width,
                scale_px(190, (*state_ptr).dpi) + non_client_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).tabs[0], GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).tabs[1], GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).tabs[2], GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).tabs[3], GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).status, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[1], 0)
                    == (*state_ptr).theme_button,
            "short classic flyout retained offscreen category or list Tab targets",
        )?;
        let classic_mode = (*state_ptr).mode;
        let (classic_hidden_index, classic_hidden_id) = if classic_mode == SearchMode::Folders {
            (3, ID_CONTENT)
        } else {
            (2, ID_FOLDERS)
        };
        let classic_count = (*state_ptr).results.len();
        send_message_w(
            hwnd,
            WM_COMMAND,
            classic_hidden_id | (BN_CLICKED << 16),
            (*state_ptr).tabs[classic_hidden_index] as Lparam,
        );
        require_ui_selftest(
            (*state_ptr).mode == classic_mode
                && (*state_ptr).results.len() == classic_count
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == classic_count as isize,
            "hidden classic category accepted a stale BN_CLICKED command",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "classic narrow/short requery exposed clipped results",
        )?;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(210, (*state_ptr).dpi) + classic_nonclient_width,
                outer_height,
                SWP_NOZORDER,
            ) != 0,
            "could not resize classic flyout to extra-narrow width",
        )?;
        let mut classic_stack = [Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }; 4];
        let mut classic_stack_readable = true;
        for (index, rect) in classic_stack.iter_mut().enumerate() {
            classic_stack_readable &= get_window_rect((*state_ptr).tabs[index], rect) != 0;
        }
        let mut narrow_title = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut narrow_subtitle = narrow_title;
        let mut narrow_theme = narrow_title;
        let mut narrow_search = narrow_title;
        require_ui_selftest(
            get_window_rect((*state_ptr).title, &mut narrow_title) != 0
                && get_window_rect((*state_ptr).subtitle, &mut narrow_subtitle) != 0
                && get_window_rect((*state_ptr).theme_button, &mut narrow_theme) != 0
                && get_window_rect((*state_ptr).edit, &mut narrow_search) != 0
                && narrow_title.right - narrow_title.left >= scale_px(130, (*state_ptr).dpi)
                && narrow_subtitle.right - narrow_subtitle.left >= scale_px(130, (*state_ptr).dpi)
                && narrow_theme.top >= narrow_subtitle.bottom + scale_px(6, (*state_ptr).dpi)
                && narrow_search.top >= narrow_theme.bottom + scale_px(8, (*state_ptr).dpi),
            "extra-narrow classic flyout clipped heading beside appearance button",
        )?;
        require_ui_selftest(
            classic_stack_readable
                && classic_stack
                    .windows(2)
                    .all(|pair| pair[1].top >= pair[0].bottom + scale_px(8, (*state_ptr).dpi))
                && classic_stack[0].right - classic_stack[0].left >= scale_px(94, (*state_ptr).dpi)
                && get_window_long_ptr_w((*state_ptr).tabs[3], GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "extra-narrow classic flyout did not stack readable filter buttons",
        )?;
        // A tall but 130px-wide classic flyout has under 100px of result
        // content. Preserve cached rows but keep its ListBox out of Tab.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(130, (*state_ptr).dpi) + classic_nonclient_width,
                outer_height,
                SWP_NOZORDER,
            ) != 0
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "very narrow classic flyout retained an unreadable result Tab target",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "very narrow classic flyout retained an unreadable query Tab target",
        )?;
        require_ui_selftest(
            (*state_ptr)
                .tabs
                .iter()
                .all(|tab| get_window_long_ptr_w(*tab, GWL_STYLE) as u32 & WS_VISIBLE == 0),
            "very narrow classic flyout retained unreadable category Tab targets",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "very narrow classic flyout retained an unreadable Appearance Tab target",
        )?;
        let narrow_classic_mode = (*state_ptr).mode;
        let (narrow_classic_chip, narrow_classic_id) = if narrow_classic_mode == SearchMode::Folders
        {
            ((*state_ptr).tabs[3], ID_CONTENT)
        } else {
            ((*state_ptr).tabs[2], ID_FOLDERS)
        };
        send_message_w(
            hwnd,
            WM_COMMAND,
            narrow_classic_id | (BN_CLICKED << 16),
            narrow_classic_chip as Lparam,
        );
        require_ui_selftest(
            (*state_ptr).mode == narrow_classic_mode && (*state_ptr).results.len() == 3,
            "width-clipped classic category accepted a stale click",
        )?;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(210, (*state_ptr).dpi) + classic_nonclient_width,
                outer_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "restoring width did not restore classic result list",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "restoring width did not restore classic query Tab target",
        )?;
        require_ui_selftest(
            (*state_ptr)
                .tabs
                .iter()
                .all(|tab| get_window_long_ptr_w(*tab, GWL_STYLE) as u32 & WS_VISIBLE != 0),
            "restoring width did not restore classic category Tab targets",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "restoring width did not restore classic Appearance Tab target",
        )?;
        // The 210px stacked classic header must not expose an offscreen
        // Appearance button or EDIT to Tab when client height is tiny.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(210, (*state_ptr).dpi) + classic_nonclient_width,
                scale_px(90, (*state_ptr).dpi) + non_client_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).tabs[0], GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "very-short classic window retained clipped Appearance or query Tab targets",
        )?;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(210, (*state_ptr).dpi) + classic_nonclient_width,
                scale_px(120, (*state_ptr).dpi) + non_client_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE
                    != 0
                && get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "classic height transition retained a clipped query Tab target",
        )?;
        // Native uses a shorter, higher search field. Transitioning back
        // from the clipped classic layout must restore the EDIT Tab target.
        (*state_ptr).theme.preset = ThemePreset::Native;
        resize_controls(hwnd, &mut *state_ptr);
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE
                    != 0
                && read_control_text_for_test((*state_ptr).edit) == "SearchTool",
            "switching a tiny classic flyout to native did not restore query input",
        )?;
        (*state_ptr).theme.preset = ThemePreset::Graphite;
        resize_controls(hwnd, &mut *state_ptr);
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "classic theme did not re-hide an offscreen query after native switch",
        )?;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                original_window.right - original_window.left,
                outer_height,
                SWP_NOZORDER,
            ) != 0,
            "could not restore classic flyout after narrow reflow check",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE
                    != 0
                && read_control_text_for_test((*state_ptr).edit) == "SearchTool",
            "expanding classic flyout did not restore query or Appearance action",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "")?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0)
                    == (*state_ptr).theme_button,
            "empty classic search exposed a focusable empty results list",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "classic search did not restore results after an empty query",
        )?;
        (*state_ptr).theme.preset = ThemePreset::Native;
        resize_controls(hwnd, &mut *state_ptr);
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && read_control_text_for_test((*state_ptr).detail_path).is_empty()
                && prepare_query_down_selection(&*state_ptr)
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path,
            "returning from classic to native theme blocked keyboard result selection",
        )?;

        // A narrow Windows 11 work area must reflow four category actions
        // into two usable rows, rather than shrink all four to tiny labels.
        // Exercise native HWND geometry and Tab order without foreground UI.
        let non_client_width = (original_window.right - original_window.left)
            - (original_client.right - original_client.left);
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(360, (*state_ptr).dpi) + non_client_width,
                outer_height,
                SWP_NOZORDER,
            ) != 0,
            "cannot resize hidden flyout to narrow width",
        )?;
        let mut narrow_client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut tab0 = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut tab1 = tab0;
        let mut tab2 = tab0;
        let mut tab3 = tab0;
        let mut status_rect = tab0;
        require_ui_selftest(
            get_client_rect(hwnd, &mut narrow_client) != 0
                && native_result_columns(narrow_client, (*state_ptr).dpi).is_none()
                && get_window_rect((*state_ptr).tabs[0], &mut tab0) != 0
                && get_window_rect((*state_ptr).tabs[1], &mut tab1) != 0
                && get_window_rect((*state_ptr).tabs[2], &mut tab2) != 0
                && get_window_rect((*state_ptr).tabs[3], &mut tab3) != 0
                && get_window_rect((*state_ptr).status, &mut status_rect) != 0
                && tab0.top == tab1.top
                && tab2.top == tab3.top
                && tab2.top >= tab0.bottom + scale_px(8, (*state_ptr).dpi)
                && tab0.right < tab1.left
                && tab2.right < tab3.left
                && tab0.right - tab0.left >= scale_px(80, (*state_ptr).dpi)
                && status_rect.top >= tab2.bottom + scale_px(8, (*state_ptr).dpi)
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[3], 0) == (*state_ptr).list,
            "narrow flyout category chips fail to reflow into usable rows",
        )?;
        // With even less horizontal space, two 60px chips are still
        // unreadable; stack all four before their widths collapse.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(210, (*state_ptr).dpi) + non_client_width,
                outer_height,
                SWP_NOZORDER,
            ) != 0,
            "cannot resize hidden flyout to extra-narrow width",
        )?;
        let mut slim_client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut slim_tab0 = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut slim_tab1 = slim_tab0;
        let mut slim_tab2 = slim_tab0;
        let mut slim_tab3 = slim_tab0;
        let mut slim_status = slim_tab0;
        require_ui_selftest(
            get_client_rect(hwnd, &mut slim_client) != 0
                && native_result_columns(slim_client, (*state_ptr).dpi).is_none()
                && get_window_rect((*state_ptr).tabs[0], &mut slim_tab0) != 0
                && get_window_rect((*state_ptr).tabs[1], &mut slim_tab1) != 0
                && get_window_rect((*state_ptr).tabs[2], &mut slim_tab2) != 0
                && get_window_rect((*state_ptr).tabs[3], &mut slim_tab3) != 0
                && get_window_rect((*state_ptr).status, &mut slim_status) != 0
                && slim_tab1.top >= slim_tab0.bottom + scale_px(8, (*state_ptr).dpi)
                && slim_tab2.top >= slim_tab1.bottom + scale_px(8, (*state_ptr).dpi)
                && slim_tab3.top >= slim_tab2.bottom + scale_px(8, (*state_ptr).dpi)
                && slim_tab0.right - slim_tab0.left >= scale_px(80, (*state_ptr).dpi)
                && slim_status.top >= slim_tab3.bottom + scale_px(8, (*state_ptr).dpi)
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE != 0,
            "extra-narrow flyout did not stack readable category buttons",
        )?;
        // Combining the 4x1 navigation and a short work area must hide
        // category buttons whose complete click area falls below the client.
        // A hidden child still has WS_VISIBLE unless we explicitly clear it.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(210, (*state_ptr).dpi) + non_client_width,
                scale_px(170, (*state_ptr).dpi) + non_client_height,
                SWP_NOZORDER,
            ) != 0,
            "cannot resize hidden flyout to simultaneously narrow/short",
        )?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).tabs[0], GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).tabs[1], GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).tabs[2], GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).tabs[3], GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).status, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[1], 0)
                    == (*state_ptr).theme_button
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3,
            "clipped category controls remain keyboard reachable in short/narrow flyout",
        )?;
        // A queued BN_CLICKED from a chip that became clipped must be
        // ignored even though we can still send a synthetic command to its
        // HWND. Its mode and cached search results must remain unchanged.
        let clipped_mode = (*state_ptr).mode;
        let clipped_count = (*state_ptr).results.len();
        require_ui_selftest(
            clipped_mode != SearchMode::Folders,
            "cannot exercise hidden Folders category from active Folders mode",
        )?;
        send_message_w(
            hwnd,
            WM_COMMAND,
            ID_FOLDERS | (BN_CLICKED << 16),
            (*state_ptr).tabs[2] as Lparam,
        );
        require_ui_selftest(
            (*state_ptr).mode == clipped_mode
                && (*state_ptr).results.len() == clipped_count
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == clipped_count as isize,
            "hidden category chip accepted a stale BN_CLICKED command",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            get_window_long_ptr_w((*state_ptr).tabs[2], GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).tabs[3], GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).status, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_next_dlg_tab_item(hwnd, (*state_ptr).tabs[1], 0)
                    == (*state_ptr).theme_button
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3,
            "requery exposed clipped category or status controls",
        )?;
        // Exercise both sides of the native query-height breakpoint without
        // giving the offscreen HWND physical focus.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(210, (*state_ptr).dpi) + non_client_width,
                scale_px(60, (*state_ptr).dpi) + non_client_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).tabs[0], GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && get_window_long_ptr_w((*state_ptr).list, GWL_STYLE) as u32 & WS_VISIBLE == 0,
            "tiny native flyout retained clipped query or navigation controls",
        )?;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                scale_px(210, (*state_ptr).dpi) + non_client_width,
                scale_px(72, (*state_ptr).dpi) + non_client_height,
                SWP_NOZORDER,
            ) != 0
                && get_window_long_ptr_w((*state_ptr).edit, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).tabs[0], GWL_STYLE) as u32 & WS_VISIBLE == 0
                && get_window_long_ptr_w((*state_ptr).theme_button, GWL_STYLE) as u32 & WS_VISIBLE
                    == 0
                && read_control_text_for_test((*state_ptr).edit) == "SearchTool",
            "native query field was not restored at its minimum fit height",
        )?;
        require_ui_selftest(
            set_window_pos(
                hwnd,
                null_mut(),
                original_window.left,
                original_window.top,
                original_window.right - original_window.left,
                outer_height,
                SWP_NOZORDER,
            ) != 0,
            "cannot restore original flyout width",
        )?;
        let mut wide_tab0 = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut wide_tab3 = wide_tab0;
        require_ui_selftest(
            get_window_rect((*state_ptr).tabs[0], &mut wide_tab0) != 0
                && get_window_rect((*state_ptr).tabs[3], &mut wide_tab3) != 0
                && wide_tab0.top == wide_tab3.top
                && get_window_long_ptr_w((*state_ptr).tabs[2], GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).tabs[3], GWL_STYLE) as u32 & WS_VISIBLE != 0
                && get_window_long_ptr_w((*state_ptr).status, GWL_STYLE) as u32 & WS_VISIBLE != 0
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path,
            "full-width flyout failed to restore category row or selection",
        )?;

        // Programmatic LB_SETCURSEL does not emit a selection notification.
        // Exercise its shared selection logic while the fixture stays hidden,
        // rather than spoofing a real user click on an invisible parent.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETCURSEL, 1, 0) >= 0,
            "could not select the second synthetic result",
        )?;
        process_list_selection_change(&mut *state_ptr);
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
        // The same selection processing used by a visible native
        // LBN_SELCHANGE must roll back a corrupted row. Invoke it directly
        // without forging a user notification on an invisible test parent.
        require_ui_selftest(
            send_message_w((*state_ptr).list, LB_SETITEMDATA, 0, 1) >= 0,
            "could not prepare corrupted native selection-change fixture",
        )?;
        process_list_selection_change(&mut *state_ptr);
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
                && verified_draw_result_row(&*state_ptr, 0, 0).is_none()
                && read_control_text_for_test((*state_ptr).detail_path) == previous_detail_path,
            "substituted native row fixture did not invalidate visible rendering",
        )?;
        // A real double-click requires a visible parent; this fixture must
        // remain offscreen. Exercise its shared Open mapping preflight
        // directly to confirm corrupt native labels clear stale selection.
        let invalid_opened = open_selected(hwnd, &mut *state_ptr);
        require_ui_selftest(
            !invalid_opened
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) < 0
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
                && verified_draw_result_row(&*state_ptr, 0, 0).is_some()
                && verified_draw_result_row(&*state_ptr, 1, 1).is_some()
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
                && verified_draw_result_row(&*state_ptr, 0, 0).is_none()
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
            !(*state_ptr).ime_composing && (*state_ptr).results.len() == 3,
            "hidden EDIT IME completion did not clear guard without offscreen query",
        )?;
        // A visible edit uses this same committed-query path. Exercise it
        // directly; the synthetic parent must never be made visible here.
        refresh_from_committed_edit_change(&mut *state_ptr);
        require_ui_selftest(
            (*state_ptr).results.is_empty()
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 0
                && read_control_text_for_test((*state_ptr).detail_name).is_empty(),
            "committed IME query did not refresh results",
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
            !(*state_ptr).ime_composing && (*state_ptr).results.len() == 3,
            "hidden EDIT focus loss must release IME guard without a requery",
        )?;
        refresh_from_committed_edit_change(&mut *state_ptr);
        require_ui_selftest(
            (*state_ptr).results.is_empty(),
            "committed IME focus-loss query did not refresh",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;
        require_ui_selftest(
            !(*state_ptr).ime_composing && (*state_ptr).results.len() == 3,
            "normal search failed after IME focus-loss cleanup",
        )?;

        // A dismissed popup can receive a delayed IME completion on its EDIT
        // after focus moved away. That notification may finish composition
        // bookkeeping but must not start an offscreen search or erase status.
        let stale_ime_marker = "Dismissed IME completion must not run Search";
        set_status(&*state_ptr, stale_ime_marker);
        let _ = send_message_w((*state_ptr).edit, WM_IME_STARTCOMPOSITION, 0, 0);
        drive_hidden_edit_change(hwnd, state_ptr, "SearchToolNoMatchZZZ")?;
        require_ui_selftest(
            (*state_ptr).ime_composing
                && (*state_ptr).results.len() == 3
                && is_window_visible(hwnd) == 0,
            "dismissed IME completion fixture did not retain prior results",
        )?;
        let _ = send_message_w((*state_ptr).edit, WM_IME_ENDCOMPOSITION, 0, 0);
        require_ui_selftest(
            !(*state_ptr).ime_composing
                && (*state_ptr).results.len() == 3
                && read_control_text_for_test((*state_ptr).status) == stale_ime_marker,
            "dismissed IME finalization started an offscreen index query",
        )?;
        drive_hidden_edit_change(hwnd, state_ptr, "SearchTool")?;

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

        // Exercise per-monitor DPI notifications on the invisible test HWND.
        // Native results and keyboard order must survive a 125% transition
        // and the return to the original scaling without a new search.
        let mut old_window = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        require_ui_selftest(
            get_window_rect(hwnd, &mut old_window) != 0,
            "could not measure hidden flyout before DPI transition",
        )?;
        let old_dpi = (*state_ptr).dpi;
        let new_dpi = old_dpi * 5 / 4;
        let new_window = Rect {
            left: old_window.left,
            top: old_window.top,
            right: old_window.left + (old_window.right - old_window.left) * 5 / 4,
            bottom: old_window.top + (old_window.bottom - old_window.top) * 5 / 4,
        };
        send_message_w(
            hwnd,
            WM_DPICHANGED,
            ((new_dpi as usize) << 16) | new_dpi as usize,
            &new_window as *const Rect as Lparam,
        );
        let mut new_client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        require_ui_selftest(
            (*state_ptr).dpi == new_dpi
                && get_client_rect(hwnd, &mut new_client) != 0
                && native_result_columns(new_client, new_dpi).is_some()
                && send_message_w((*state_ptr).list, LB_GETCOUNT, 0, 0) == 3
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path
                && get_next_dlg_tab_item(hwnd, (*state_ptr).theme_button, 0)
                    == (*state_ptr).detail_open
                && is_window_visible(hwnd) == 0,
            "high-DPI hidden native flyout lost details or Tab order",
        )?;
        let target_margin = scale_px(14, new_dpi).min(u16::MAX as i32) as usize;
        let observed_margins = send_message_w((*state_ptr).edit, EM_GETMARGINS, 0, 0) as usize;
        require_ui_selftest(
            [
                (*state_ptr).detail_header,
                (*state_ptr).detail_name,
                (*state_ptr).detail_kind,
                (*state_ptr).detail_path,
                (*state_ptr).detail_open,
            ]
            .iter()
            .all(|&control| {
                send_message_w(control, WM_GETFONT, 0, 0) == (*state_ptr).ui_font as isize
            }) && observed_margins & 0xffff == target_margin
                && (observed_margins >> 16) & 0xffff == target_margin,
            "high-DPI detail fonts or query padding retained the previous DPI",
        )?;
        send_message_w(
            hwnd,
            WM_DPICHANGED,
            ((old_dpi as usize) << 16) | old_dpi as usize,
            &old_window as *const Rect as Lparam,
        );
        let mut reset_client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        require_ui_selftest(
            (*state_ptr).dpi == old_dpi
                && get_client_rect(hwnd, &mut reset_client) != 0
                && native_result_columns(reset_client, old_dpi).is_some()
                && send_message_w((*state_ptr).list, LB_GETCURSEL, 0, 0) == 0
                && read_control_text_for_test((*state_ptr).edit) == "SearchTool"
                && read_control_text_for_test((*state_ptr).detail_path)
                    == (&(*state_ptr).results)[0].path
                && get_next_dlg_tab_item(hwnd, (*state_ptr).theme_button, 0)
                    == (*state_ptr).detail_open
                && [
                    (*state_ptr).detail_header,
                    (*state_ptr).detail_name,
                    (*state_ptr).detail_kind,
                    (*state_ptr).detail_path,
                    (*state_ptr).detail_open,
                ]
                .iter()
                .all(|&control| {
                    send_message_w(control, WM_GETFONT, 0, 0) == (*state_ptr).ui_font as isize
                })
                && (send_message_w((*state_ptr).edit, EM_GETMARGINS, 0, 0) as usize & 0xffff)
                    == scale_px(14, old_dpi).min(u16::MAX as i32) as usize
                && is_window_visible(hwnd) == 0,
            "DPI restoration lost selected result or Tab order",
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

        // The resident popup can be made temporarily topmost while shown.
        // A later WA_INACTIVE must remove that z-order state on dismissal.
        // This test changes only the hidden synthetic HWND, never the user's
        // foreground, pointer, keyboard or installed production window.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                topmost_window(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            ) != 0
                && get_window_long_ptr_w(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0
                && is_window_visible(hwnd) == 0,
            "hidden popup could not enter synthetic topmost precondition",
        )?;
        let _ = send_message_w(hwnd, WM_ACTIVATE, 0, 0);
        require_ui_selftest(
            is_window_visible(hwnd) == 0
                && get_window_long_ptr_w(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST == 0,
            "resident popup dismissal retained topmost z-order",
        )?;
        // A delayed WM_MOUSEMOVE from the Appearance child may be delivered
        // after its hidden parent is dismissed. Never create an invisible
        // hover highlight that leaks into the next popup session.
        send_message_w(hwnd, WM_THEME_BUTTON_HOT, 1, 0);
        require_ui_selftest(
            !(*state_ptr).theme_button_hot,
            "hidden popup accepted a stale Appearance-button hover",
        )?;
        // Dismissal itself must clear a previously hot Appearance control,
        // even when WM_MOUSELEAVE never arrives before the window hides.
        (*state_ptr).theme_button_hot = true;
        send_message_w(hwnd, WM_ACTIVATE, 0, 0);
        require_ui_selftest(
            !(*state_ptr).theme_button_hot,
            "resident popup dismissal retained stale Appearance hover",
        )?;
        // Old WA_INACTIVE/WM_CLOSE notifications can arrive after the parent
        // is already hidden, but while the hook has published a newer BEGIN.
        // Those stale window messages must not cancel that newer generation
        // or erase its pending Explorer scope.
        let held_generation = SHELL_BRIDGE_BEGIN_GENERATION.swap(71, Ordering::AcqRel);
        SHELL_BRIDGE_ACTIVE.store(true, Ordering::Release);
        {
            let mut pending = SHELL_BRIDGE_SCOPE
                .lock()
                .map_err(|_| io::Error::other("bridge scope mutex poisoned"))?;
            *pending = Some(String::from(r"C:\PendingAfterHiddenDismissFixture"));
        }
        send_message_w(hwnd, WM_ACTIVATE, 0, 0);
        send_message_w(hwnd, WM_CLOSE, 0, 0);
        require_ui_selftest(
            is_window_visible(hwnd) == 0
                && SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire)
                && SHELL_BRIDGE_BEGIN_GENERATION.load(Ordering::Acquire) == 71
                && SHELL_BRIDGE_SCOPE.lock().is_ok_and(|pending| {
                    pending.as_deref() == Some(r"C:\PendingAfterHiddenDismissFixture")
                }),
            "hidden old dismissal cancelled a newer pending Shell bridge session",
        )?;
        SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
        SHELL_BRIDGE_BEGIN_GENERATION.store(held_generation, Ordering::Release);
        let _ = take_bridge_scope();
        // Successful ShellExecute used to hide the resident HWND without
        // releasing temporary topmost or the optional bridge's active state.
        // Exercise just the post-success cleanup on a synthetic hidden HWND.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                topmost_window(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            ) != 0
                && get_window_long_ptr_w(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0
                && is_window_visible(hwnd) == 0,
            "successful Open cleanup fixture could not set hidden topmost state",
        )?;
        SHELL_BRIDGE_ACTIVE.store(true, Ordering::Release);
        dismiss_resident_after_open(hwnd, &mut *state_ptr);
        require_ui_selftest(
            is_window_visible(hwnd) == 0
                && get_window_long_ptr_w(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST == 0
                && !SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire),
            "successful resident Open retained topmost or active keyboard bridge",
        )?;
        // An asynchronous BEGIN may still be queued after cancellation.
        // Reject it without showing even the synthetic test window, changing
        // an existing query or allowing a stale Explorer scope to survive.
        let query_before_cancelled_begin = read_control_text_for_test((*state_ptr).edit);
        let scope_before_cancelled_begin = (*state_ptr).scope.clone();
        {
            let mut pending_scope = SHELL_BRIDGE_SCOPE
                .lock()
                .map_err(|_| io::Error::other("bridge scope mutex poisoned"))?;
            *pending_scope = Some(String::from(r"C:\CancelledSearchFixture"));
        }
        send_message_w(hwnd, WM_SHELL_BRIDGE_BEGIN, 0, 0);
        require_ui_selftest(
            is_window_visible(hwnd) == 0
                && read_control_text_for_test((*state_ptr).edit) == query_before_cancelled_begin
                && (*state_ptr).scope == scope_before_cancelled_begin
                && !SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire)
                && SHELL_BRIDGE_SCOPE
                    .lock()
                    .is_ok_and(|pending| pending.is_none()),
            "cancelled asynchronous bridge BEGIN reopened or modified Search",
        )?;
        // When request N+1 is active, the earlier queued BEGIN from N must
        // neither show a flyout nor consume N+1's pending Explorer scope.
        let prior_generation = SHELL_BRIDGE_BEGIN_GENERATION.swap(2, Ordering::AcqRel);
        SHELL_BRIDGE_ACTIVE.store(true, Ordering::Release);
        {
            let mut pending = SHELL_BRIDGE_SCOPE
                .lock()
                .map_err(|_| io::Error::other("bridge scope mutex poisoned"))?;
            *pending = Some(String::from(r"C:\NewerSearchFixture"));
        }
        send_message_w(hwnd, WM_SHELL_BRIDGE_BEGIN, 1, 0);
        require_ui_selftest(
            is_window_visible(hwnd) == 0
                && read_control_text_for_test((*state_ptr).edit) == query_before_cancelled_begin
                && (*state_ptr).scope == scope_before_cancelled_begin
                && SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire)
                && SHELL_BRIDGE_SCOPE
                    .lock()
                    .is_ok_and(|pending| pending.as_deref() == Some(r"C:\NewerSearchFixture")),
            "old BEGIN displaced a newer active Shell bridge request",
        )?;
        SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
        SHELL_BRIDGE_BEGIN_GENERATION.store(prior_generation, Ordering::Release);
        let _ = take_bridge_scope();
        // Native offscreen dismissal must revoke ownership of a bridge
        // session and clear old IME preedit state. Some IMEs do not dispatch
        // a final EDIT WM_IME_ENDCOMPOSITION after the popup is dismissed;
        // a later manual/bridge session must not inherit its composing guard.
        send_message_w((*state_ptr).edit, WM_IME_STARTCOMPOSITION, 0, 0);
        require_ui_selftest(
            (*state_ptr).ime_composing && is_window_visible(hwnd) == 0,
            "IME-dismissal regression could not stage synthetic old composition",
        )?;
        (*state_ptr).bridge_session_generation = 8;
        SHELL_BRIDGE_ACTIVE.store(true, Ordering::Release);
        hide_bridge_overlay(hwnd, &mut *state_ptr);
        require_ui_selftest(
            !(*state_ptr).ime_composing,
            "resident dismissal retained stale IME composition across sessions",
        )?;
        require_ui_selftest(
            (*state_ptr).bridge_session_generation == 0
                && !SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire)
                && !bridge_input_belongs_to_displayed_session(
                    8,
                    8,
                    (*state_ptr).bridge_session_generation,
                )
                && is_window_visible(hwnd) == 0,
            "resident dismissal retained stale keyboard-bridge session ownership",
        )?;
        // A failed PostMessage for request N can finish after N+1 published
        // its pending Explorer scope. The failure path must clear only its
        // own generation; no message or real window is posted here.
        let previous_post_generation = SHELL_BRIDGE_BEGIN_GENERATION.swap(12, Ordering::AcqRel);
        {
            let mut pending = SHELL_BRIDGE_SCOPE
                .lock()
                .map_err(|_| io::Error::other("bridge scope mutex poisoned"))?;
            *pending = Some(String::from(r"C:\NewerPostScopeFixture"));
        }
        clear_pending_bridge_scope_after_failed_post(11);
        require_ui_selftest(
            SHELL_BRIDGE_SCOPE
                .lock()
                .is_ok_and(|pending| pending.as_deref() == Some(r"C:\NewerPostScopeFixture")),
            "failed old bridge BEGIN post erased newer Explorer scope",
        )?;
        clear_pending_bridge_scope_after_failed_post(12);
        require_ui_selftest(
            SHELL_BRIDGE_SCOPE
                .lock()
                .is_ok_and(|pending| pending.is_none()),
            "failed current bridge BEGIN post retained stale Explorer scope",
        )?;
        // Even when an earlier precheck saw N, the scope consumer must not
        // take N+1's publication if generation changed before the mutex.
        SHELL_BRIDGE_BEGIN_GENERATION.store(14, Ordering::Release);
        SHELL_BRIDGE_ACTIVE.store(true, Ordering::Release);
        {
            let mut pending = SHELL_BRIDGE_SCOPE
                .lock()
                .map_err(|_| io::Error::other("bridge scope mutex poisoned"))?;
            *pending = Some(String::from(r"C:\NewestBeginScopeFixture"));
        }
        require_ui_selftest(
            take_current_bridge_begin_scope(13).is_none()
                && SHELL_BRIDGE_SCOPE
                    .lock()
                    .is_ok_and(|pending| pending.as_deref() == Some(r"C:\NewestBeginScopeFixture")),
            "old BEGIN consumed newer Explorer scope after a stale precheck",
        )?;
        require_ui_selftest(
            take_current_bridge_begin_scope(14)
                == Some(Some(String::from(r"C:\NewestBeginScopeFixture")))
                && SHELL_BRIDGE_SCOPE
                    .lock()
                    .is_ok_and(|pending| pending.is_none()),
            "current BEGIN failed to consume its own Explorer scope",
        )?;
        SHELL_BRIDGE_ACTIVE.store(false, Ordering::Release);
        SHELL_BRIDGE_BEGIN_GENERATION.store(previous_post_generation, Ordering::Release);
        // A valid IPC search or manual hotkey may take over while the
        // bridge's temporary HWND_TOPMOST style is still set. Exercise
        // takeover against only our never-shown synthetic Win32 HWND.
        require_ui_selftest(
            set_window_pos(
                hwnd,
                topmost_window(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            ) != 0
                && get_window_long_ptr_w(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0
                && is_window_visible(hwnd) == 0,
            "manual bridge handoff fixture could not set hidden topmost state",
        )?;
        (*state_ptr).bridge_session_generation = 19;
        (*state_ptr).theme_button_hot = true;
        SHELL_BRIDGE_ACTIVE.store(true, Ordering::Release);
        // A delayed old EDIT START can arrive after the last dismissal;
        // the next explicitly manual or IPC session cannot inherit it.
        send_message_w((*state_ptr).edit, WM_IME_STARTCOMPOSITION, 0, 0);
        require_ui_selftest(
            (*state_ptr).ime_composing && is_window_visible(hwnd) == 0,
            "old IME START did not stage the delayed manual handoff fixture",
        )?;
        handoff_bridge_to_manual_session(hwnd, &mut *state_ptr);
        require_ui_selftest(
            !(*state_ptr).ime_composing,
            "manual Search takeover inherited a delayed hidden IME composition",
        )?;
        require_ui_selftest(
            (*state_ptr).bridge_session_generation == 0
                && !(*state_ptr).theme_button_hot
                && !SHELL_BRIDGE_ACTIVE.load(Ordering::Acquire)
                && get_window_long_ptr_w(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST == 0
                && is_window_visible(hwnd) == 0,
            "manual bridge handoff retained temporary topmost z-order",
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

    fn valid_copydata_length(kind: usize, bytes: usize) -> bool {
        if bytes < 2 || !bytes.is_multiple_of(2) {
            return false;
        }
        match kind {
            1 => bytes <= (MAX_QUERY_U16 as usize + 1) * 2,
            2 => bytes <= 65_536 * 2,
            _ => false,
        }
    }

    fn decode_legacy_ipc_query(words: &[u16]) -> Option<SearchRequest> {
        // Preserve both legacy raw UTF-16 and a single trailing NUL, but
        // never reinterpret a malformed surrogate or ignore a hidden suffix.
        let end = match words.iter().position(|&value| value == 0) {
            Some(index) if index + 1 == words.len() => index,
            Some(_) => return None,
            None => words.len(),
        };
        if end > MAX_QUERY_U16 as usize {
            return None;
        }
        let query = String::from_utf16(&words[..end]).ok()?;
        Some(SearchRequest {
            query: (!query.trim().is_empty()).then(|| query.trim().to_string()),
            scope: None,
        })
    }

    fn decode_ipc_request(words: &[u16]) -> Option<SearchRequest> {
        // Type 2 is exactly query\0scope\0. An unterminated scope was
        // previously accepted, letting a truncated/malformed message change
        // the current Search session instead of failing closed.
        let first_end = words.iter().position(|&value| value == 0)?;
        if first_end > MAX_QUERY_U16 as usize {
            return None;
        }
        let rest = words.get(first_end + 1..)?;
        let second_end = rest.iter().position(|&value| value == 0)?;
        // The second terminator must end the complete message, and a scope
        // cannot exceed the supported extended Windows path length.
        if second_end > 32_767 || second_end + 1 != rest.len() {
            return None;
        }
        // Never reinterpret an invalid UTF-16 query or scope through U+FFFD:
        // the sender's actual requested Search identity could change.
        let query = String::from_utf16(&words[..first_end]).ok()?;
        let scope = String::from_utf16(&rest[..second_end]).ok()?;
        // Only an explicitly empty second field means global Search. A
        // nonempty location that normalizes away (root separators, quotes,
        // whitespace) must not silently remove the requested scope.
        let scope = validated_explicit_scope(&scope)?;
        Some(SearchRequest {
            query: (!query.trim().is_empty()).then(|| query.trim().to_string()),
            scope,
        })
    }

    // Keep CLI behavior in testable functions. Historically an invalid
    // --scope silently set scope=None, and an invalid --search-uri left a
    // preceding --query intact as a global query.
    // External requests must fit the same UTF-16 edit/query limit as native
    // IPC; otherwise the GUI shows more text than refresh_results can read.
    fn validated_external_query(value: &str) -> Option<Option<String>> {
        let trimmed = value.trim();
        if trimmed.contains('\0') || trimmed.encode_utf16().count() > MAX_QUERY_U16 as usize {
            return None;
        }
        Some((!trimmed.is_empty()).then(|| trimmed.to_string()))
    }

    // Keep command-line query validation independently testable without
    // launching a foreground Search surface.
    fn apply_cli_query(request: &mut SearchRequest, value: &str) -> io::Result<()> {
        let validated = validated_external_query(value).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid --query text or length",
            )
        })?;
        if let Some(query) = validated {
            request.query = Some(query);
        }
        Ok(())
    }

    fn apply_cli_scope(request: &mut SearchRequest, value: &str) -> io::Result<()> {
        // An explicitly invalid folder must abort before the GUI can launch
        // the previous query with scope=None (all indexed files).
        let scope = validated_explicit_scope(value).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invalid --scope location")
        })?;
        request.scope = scope;
        Ok(())
    }

    fn apply_cli_search_uri(request: &mut SearchRequest, value: &str) -> io::Result<()> {
        // Do not ignore a malformed explicit URI and preserve a previous
        // --query as an unintended global Search request.
        let parsed = parse_search_uri(value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid Search URI"))?;
        *request = parsed;
        Ok(())
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
        let mut explicit_scope_seen = false;
        let mut explicit_query_seen = false;
        for pair in rest.split('&') {
            // A bare token within a keyed URI is malformed, not an
            // ignorable query parameter. In particular `&scope` used to
            // silently discard a requested directory and run globally.
            // Standalone unkeyed `search:meeting%20notes` remains supported.
            let Some((encoded_key, value)) = pair.split_once('=') else {
                if !pair.is_empty() && rest.contains('=') {
                    return None;
                }
                continue;
            };
            // URI parameter names can be percent encoded too. Ignoring
            // sc%6fpe as an unknown key would silently drop a requested
            // Explorer folder restriction and broaden Search globally.
            let key = percent_decode(encoded_key)?;
            if key.eq_ignore_ascii_case("query") || key.eq_ignore_ascii_case("q") {
                accept_uri_query(
                    &mut request,
                    &mut explicit_query_seen,
                    &percent_decode(value)?,
                )?;
                continue;
            }
            if key.eq_ignore_ascii_case("scope") || key.eq_ignore_ascii_case("location") {
                accept_uri_scope(
                    &mut request,
                    &mut explicit_scope_seen,
                    &percent_decode(value)?,
                )?;
                continue;
            }
            if key.eq_ignore_ascii_case("crumb") {
                let decoded = percent_decode(value)?;
                if let Some((kind, location)) = decoded.split_once(':') {
                    if kind.eq_ignore_ascii_case("location") {
                        accept_uri_scope(&mut request, &mut explicit_scope_seen, location)?;
                    }
                } else if decoded.eq_ignore_ascii_case("location") {
                    // A location crumb without its ':' separator is not a
                    // harmless, unknown Explorer filter: ignoring it would
                    // turn a requested scoped query into global Search.
                    return None;
                }
            }
        }
        if !rest.contains('=') {
            accept_uri_query(
                &mut request,
                &mut explicit_query_seen,
                &percent_decode(rest)?,
            )?;
        }
        (!request.is_empty()).then_some(request)
    }

    // An URI can repeat the same query through q/query aliases. Treat an
    // incompatible second value (even explicitly blank) as malformed rather
    // than running a different query from the one first specified.
    fn accept_uri_query(
        request: &mut SearchRequest,
        explicit_query_seen: &mut bool,
        value: &str,
    ) -> Option<()> {
        let parsed = validated_external_query(value)?;
        if *explicit_query_seen && request.query != parsed {
            return None;
        }
        *explicit_query_seen = true;
        request.query = parsed;
        Some(())
    }

    // Multiple URI scope/location/crumb parameters are only valid if they
    // agree. A later blank or different folder must not overwrite an earlier
    // restriction and silently broaden or redirect the search.
    fn accept_uri_scope(
        request: &mut SearchRequest,
        explicit_scope_seen: &mut bool,
        value: &str,
    ) -> Option<()> {
        let parsed = validated_explicit_scope(value)?;
        if *explicit_scope_seen {
            let same_scope = match (request.scope.as_deref(), parsed.as_deref()) {
                (None, None) => true,
                (Some(previous), Some(current)) => {
                    normalized_scope_key(previous) == normalized_scope_key(current)
                }
                _ => false,
            };
            if !same_scope {
                return None;
            }
        }
        *explicit_scope_seen = true;
        request.scope = parsed;
        Some(())
    }

    // Only a truly empty explicit scope denotes a global search. An
    // unusable nonempty location must fail closed across IPC and URI input.
    fn validated_explicit_scope(value: &str) -> Option<Option<String>> {
        if value.is_empty() {
            Some(None)
        } else {
            Some(Some(normalize_scope(value)?))
        }
    }

    fn normalize_scope(value: impl AsRef<str>) -> Option<String> {
        let value = value.as_ref().trim().trim_matches('"');
        if value.is_empty() {
            return None;
        }
        let mut normalized = value.replace('/', "\\");
        // Do not accept an unresolved dot segment as a scoped folder:
        // `C:\Private\..\Public` refers outside the claimed prefix after
        // Windows resolves it. Reject during input validation instead of
        // silently yielding zero results in the later lexical result guard.
        if has_dot_path_segment(&normalized) {
            return None;
        }
        // A Windows drive-relative path (C: or C:docs) is *not* C:\.
        // Interpreting C: as a lexical prefix for indexed absolute paths
        // would widen the intended scope to every file on the volume.
        let bytes = normalized.as_bytes();
        if bytes.len() >= 2
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes.get(2) != Some(&b'\\')
        {
            return None;
        }
        while normalized.ends_with('\\')
            && !(normalized.len() == 3 && normalized.as_bytes().get(1) == Some(&b':'))
        {
            normalized.pop();
        }
        // A scope must identify one complete Windows volume or UNC share.
        // Bare names and root-relative paths have no stable base directory;
        // a lexical index filter can otherwise search unrelated locations.
        let bytes = normalized.as_bytes();
        let drive_absolute = bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'\\';
        let unc_absolute = normalized.strip_prefix(r"\\").is_some_and(|tail| {
            let mut parts = tail.split('\\');
            let server = parts.next().unwrap_or_default();
            let share = parts.next().unwrap_or_default();
            !server.is_empty()
                && server != "."
                && server != "?"
                && !share.is_empty()
                && parts.all(|part| !part.is_empty())
        });
        (drive_absolute || unc_absolute).then_some(normalized)
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

    fn percent_decode(value: &str) -> Option<String> {
        let bytes = value.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'%' => {
                    // A malformed or truncated escape must never become an
                    // ignored URI key (e.g. `sc%ope`), silently deleting an
                    // explicitly requested folder restriction.
                    let hi = hex(*bytes.get(index + 1)?)?;
                    let lo = hex(*bytes.get(index + 2)?)?;
                    out.push((hi << 4) | lo);
                    index += 3;
                    continue;
                }
                b'+' => out.push(b' '),
                value => out.push(value),
            }
            index += 1;
        }
        // Never reinterpret malformed percent-encoded UTF-8 as U+FFFD or
        // accept a NUL that truncates the native Win32 EDIT/query string.
        String::from_utf8(out)
            .ok()
            .filter(|decoded| !decoded.contains('\0'))
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
        fn external_queries_must_not_silently_truncate_or_embed_nul() {
            // Native refresh_results reads only MAX_QUERY_U16 UTF-16 units.
            // A longer external query must not display one text but search a
            // different prefix. UTF-16 count matters for non-BMP characters.
            for value in [
                "x".repeat(MAX_QUERY_U16 as usize + 1),
                "😀".repeat(513),
                "public\0private".to_string(),
            ] {
                let initial = SearchRequest {
                    query: Some("before".into()),
                    scope: Some(r"C:\Private".into()),
                };
                let mut cli = initial.clone();
                assert!(
                    apply_cli_query(&mut cli, &value).is_err(),
                    "CLI accepted unsafe query with {} UTF-16 units",
                    value.encode_utf16().count()
                );
                assert_eq!(cli, initial, "rejected query changed the existing request");
                assert_eq!(
                    parse_search_uri(&format!("search:q={value}")),
                    None,
                    "URI accepted unsafe external query"
                );
            }
            let mut cli = SearchRequest::default();
            let max_query = "😀".repeat(512);
            apply_cli_query(&mut cli, &max_query).unwrap();
            assert_eq!(cli.query.as_deref(), Some(max_query.as_str()));
            assert_eq!(
                parse_search_uri(&format!("search:q={max_query}")),
                Some(cli)
            );
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
        fn malformed_copydata_lengths_are_rejected_before_slice_construction() {
            assert!(valid_copydata_length(1, 2));
            assert!(valid_copydata_length(1, (MAX_QUERY_U16 as usize + 1) * 2));
            assert!(!valid_copydata_length(1, (MAX_QUERY_U16 as usize + 2) * 2));
            assert!(valid_copydata_length(2, 65_536 * 2));
            assert!(!valid_copydata_length(2, (65_536 + 1) * 2));
            for kind in [1, 2, 99] {
                for bytes in [0, 1, 3, usize::MAX] {
                    assert!(!valid_copydata_length(kind, bytes));
                }
            }
            assert!(!valid_copydata_length(99, 2));
        }

        #[test]
        fn legacy_ipc_query_rejects_malformed_surrogates_and_hidden_suffix() {
            // Existing legacy raw-text packets are allowed with or without
            // one UTF-16 NUL terminator for backwards compatibility.
            let expected = SearchRequest {
                query: Some("notes".to_string()),
                scope: None,
            };
            assert_eq!(
                decode_legacy_ipc_query(&"notes".encode_utf16().collect::<Vec<_>>()),
                Some(expected.clone())
            );
            assert_eq!(
                decode_legacy_ipc_query(&[
                    'n' as u16, 'o' as u16, 't' as u16, 'e' as u16, 's' as u16, 0
                ]),
                Some(expected)
            );
            assert_eq!(
                decode_legacy_ipc_query(&[0]),
                Some(SearchRequest::default())
            );
            assert_eq!(
                decode_legacy_ipc_query(&[0xD800]),
                None,
                "unpaired high surrogate"
            );
            assert_eq!(
                decode_legacy_ipc_query(&[0xDFFF, 0]),
                None,
                "unpaired low surrogate"
            );
            assert_eq!(
                decode_legacy_ipc_query(&['a' as u16, 0, 'b' as u16]),
                None,
                "hidden suffix after NUL"
            );
            assert_eq!(
                decode_legacy_ipc_query(&['a' as u16, 0, 0]),
                None,
                "extra NUL padding"
            );
            assert_eq!(
                decode_legacy_ipc_query(&vec![b'x' as u16; MAX_QUERY_U16 as usize + 1]),
                None,
                "overlength unterminated legacy query"
            );
            let emoji = "file 📁";
            let mut terminated = emoji.encode_utf16().collect::<Vec<_>>();
            terminated.push(0);
            assert_eq!(
                decode_legacy_ipc_query(&terminated),
                Some(SearchRequest {
                    query: Some(emoji.to_string()),
                    scope: None
                })
            );
        }

        #[test]
        fn ipc_v2_rejects_unterminated_extra_and_oversized_fields() {
            assert_eq!(decode_ipc_request(&[0]), None, "missing scope terminator");
            assert_eq!(decode_ipc_request(&['q' as u16, 0, 'C' as u16]), None);
            assert_eq!(decode_ipc_request(&['q' as u16, 0, 0, 'x' as u16]), None);
            assert_eq!(decode_ipc_request(&['q' as u16, 0, 0, 0]), None);
            let long_query = vec!['x' as u16; MAX_QUERY_U16 as usize + 1];
            let mut payload = long_query;
            payload.extend([0, 0]);
            assert_eq!(
                decode_ipc_request(&payload),
                None,
                "query exceeds search UI limit"
            );
            let mut long_scope = vec![0];
            long_scope.extend(vec!['C' as u16; 32_768]);
            long_scope.push(0);
            assert_eq!(
                decode_ipc_request(&long_scope),
                None,
                "scope exceeds Win32 path limit"
            );
            assert_eq!(
                decode_ipc_request(&[0xD800, 0, 0]),
                None,
                "unpaired query surrogate"
            );
            assert_eq!(
                decode_ipc_request(&[0, 0xDFFF, 0]),
                None,
                "unpaired scope surrogate"
            );
            assert_eq!(decode_ipc_request(&[0, 0]), Some(SearchRequest::default()));
            let mut non_bmp = String::from("dosya📁").encode_utf16().collect::<Vec<u16>>();
            non_bmp.extend([0, 0]);
            assert_eq!(
                decode_ipc_request(&non_bmp),
                Some(SearchRequest {
                    query: Some(String::from("dosya📁")),
                    scope: None
                })
            );
        }

        #[test]
        fn ipc_empty_request_resets_to_global_search() {
            assert_eq!(decode_ipc_request(&[0, 0]), Some(SearchRequest::default()));
        }

        #[test]
        fn ipc_scope_must_not_silently_fall_back_to_global_search() {
            // An explicitly supplied but unusable Explorer location must
            // fail closed. Only a genuinely empty scope means global Search.
            for malformed_scope in [r"\", "\"", "  ", "\\\\"] {
                let mut payload = "secret".encode_utf16().collect::<Vec<u16>>();
                payload.push(0);
                payload.extend(malformed_scope.encode_utf16());
                payload.push(0);
                assert_eq!(
                    decode_ipc_request(&payload),
                    None,
                    "invalid nonempty scope {malformed_scope:?} broadened to global Search"
                );
            }
            let mut global = "secret".encode_utf16().collect::<Vec<u16>>();
            global.extend([0, 0]);
            assert_eq!(
                decode_ipc_request(&global),
                Some(SearchRequest {
                    query: Some(String::from("secret")),
                    scope: None
                })
            );
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
        fn search_uri_invalid_explicit_scope_does_not_broaden_to_global() {
            for uri in [
                "search:q=secret&scope=%5C",
                "search:q=secret&location=%22",
                "search:q=secret&crumb=location:%5C",
                "search:q=secret&scope=%20%20",
            ] {
                assert_eq!(
                    parse_search_uri(uri),
                    None,
                    "invalid location broadened Search: {uri}"
                );
            }
            assert_eq!(
                parse_search_uri("search:q=secret&scope="),
                Some(SearchRequest {
                    query: Some(String::from("secret")),
                    scope: None
                })
            );
        }

        #[test]
        fn uri_missing_equals_must_not_silently_drop_explicit_scope() {
            // A malformed scope token after a valid query previously vanished
            // from the parse, broadening a scoped request to global Search.
            for uri in [
                "search:q=secret&scope",
                "search:q=secret&location",
                "search:q=secret&crumb",
                "search:q=secret&sc%6fpe",
                "searchtool:q=secret&scope=C%3A%5CPrivate&location",
                "search:q=secret&unrecognized",
            ] {
                assert_eq!(
                    parse_search_uri(uri),
                    None,
                    "bare URI parameter was ignored: {uri}"
                );
            }
            assert_eq!(
                parse_search_uri("search:q=secret&scope=C%3A%5CPrivate&"),
                Some(SearchRequest {
                    query: Some("secret".into()),
                    scope: Some(r"C:\Private".into()),
                }),
                "harmless trailing ampersand remains compatible"
            );
            assert_eq!(
                parse_search_uri("search:meeting%20notes"),
                Some(SearchRequest {
                    query: Some("meeting notes".into()),
                    scope: None
                }),
                "legacy unkeyed Search URI remains supported"
            );
        }

        #[test]
        fn uri_percent_encoded_scope_keys_must_not_be_silently_ignored() {
            // Percent encoding also applies to parameter names. Ignoring an
            // escaped scope key turns a restricted query into global Search.
            for uri in [
                "search:q=secret&sc%6fpe=C%3A%5CPrivate",
                "search:q=secret&locati%6Fn=C%3A%5CPrivate",
                "search:q=secret&cr%75mb=location:C%3A%5CPrivate",
                "search:q=secret&%73cope=C%3A%5CPrivate",
            ] {
                assert_eq!(
                    parse_search_uri(uri),
                    Some(SearchRequest {
                        query: Some("secret".into()),
                        scope: Some(r"C:\Private".into()),
                    }),
                    "percent-encoded scope key was ignored: {uri}"
                );
            }
            for uri in [
                "search:q=secret&sc%6fpe=%5C",
                "search:q=secret&sc%6fpe=C%3A%5CPrivate&scope=",
                "search:q=secret&sc%00pe=C%3A%5CPrivate",
                "search:q=secret&sc%FFpe=C%3A%5CPrivate",
            ] {
                assert_eq!(
                    parse_search_uri(uri),
                    None,
                    "invalid encoded key was accepted: {uri}"
                );
            }
        }

        #[test]
        fn search_uri_rejects_malformed_percent_escapes_without_global_fallback() {
            // A half-escaped parameter key is not an unknown field: it is
            // malformed input that must not erase the intended scope.
            for uri in [
                "search:q=secret&sc%ope=C%3A%5CPrivate",
                "search:q=secret&sc%6pe=C%3A%5CPrivate",
                "search:q=secret&scope=C%3A%5CPrivate%",
                "search:q=secret&scope=C%3A%5CPrivate%Q1",
                "search:q=secret%",
                "search:q=secret%G0",
                "search:q=%1",
            ] {
                assert_eq!(
                    parse_search_uri(uri),
                    None,
                    "malformed percent escape was accepted: {uri}"
                );
            }
            assert_eq!(
                parse_search_uri("search:q=100%25+complete&scope=C%3A%5CPrivate"),
                Some(SearchRequest {
                    query: Some("100% complete".into()),
                    scope: Some(r"C:\Private".into()),
                }),
                "properly encoded literal percent must remain supported"
            );
        }

        #[test]
        fn search_uri_rejects_invalid_utf8_and_embedded_nul_bytes() {
            // Lossy UTF-8 replacement mutates the requested path/query.
            // An embedded NUL truncates Win32 EDIT strings while the parser
            // still retains everything following that terminator.
            for uri in [
                "search:q=%FF",
                "search:q=%C3%28",
                "search:q=public%00private",
                "search:q=secret&scope=C%3A%5CPrivate%00Other",
                "search:q=secret&scope=C%3A%5CPrivate%FF",
                "search:q=secret&crumb=location:C%3A%5CPrivate%00Other",
                "search:%FF",
            ] {
                assert_eq!(
                    parse_search_uri(uri),
                    None,
                    "malformed URI bytes were accepted: {uri}"
                );
            }
            assert_eq!(
                parse_search_uri("search:q=caf%C3%A9+%F0%9F%93%81&scope=C%3A%5CDocuments"),
                Some(SearchRequest {
                    query: Some("café 📁".into()),
                    scope: Some(r"C:\Documents".into()),
                }),
                "well-formed UTF-8 and non-BMP Unicode must remain supported"
            );
        }

        #[test]
        fn uri_location_crumb_missing_separator_must_not_drop_scope() {
            for uri in [
                "search:q=secret&crumb=location",
                "search:q=secret&crumb=LOCATION",
                "search:q=secret&scope=C%3A%5CPrivate&crumb=location",
            ] {
                assert_eq!(
                    parse_search_uri(uri),
                    None,
                    "malformed location crumb accepted: {uri}"
                );
            }
            assert_eq!(
                parse_search_uri("search:q=secret&crumb=kind:document"),
                Some(SearchRequest {
                    query: Some("secret".into()),
                    scope: None
                }),
                "non-location Explorer crumb remains compatible"
            );
        }

        #[test]
        fn search_uri_conflicting_query_aliases_must_not_replace_requested_text() {
            // q/query are aliases for the same field. A second conflicting
            // value, including blank, must not silently replace or preserve
            // an earlier query selected by a different URI component.
            for uri in [
                "search:q=secret&query=public",
                "search:query=public&q=secret",
                "search:q=secret&q=",
                "search:q=&query=secret",
                "search:q=secret&query=public&scope=C%3A%5CPrivate",
            ] {
                assert_eq!(
                    parse_search_uri(uri),
                    None,
                    "conflicting query aliases accepted: {uri}"
                );
            }
            assert_eq!(
                parse_search_uri("search:q=secret&query=secret&scope=C%3A%5CPrivate"),
                Some(SearchRequest {
                    query: Some("secret".into()),
                    scope: Some(r"C:\Private".into()),
                }),
                "equivalent repeated query aliases remain valid"
            );
        }

        #[test]
        fn search_uri_conflicting_explicit_scopes_cannot_override_first_location() {
            // URL query parameter order cannot silently broaden a restricted
            // Explorer search or switch to an unrelated directory.
            for uri in [
                "search:q=secret&scope=C%3A%5CPrivate&location=",
                "search:q=secret&scope=C%3A%5CPrivate&location=C%3A%5COther",
                "search:q=secret&crumb=location:C%3A%5CPrivate&scope=",
                "search:q=secret&location=&scope=C%3A%5CPrivate",
                "search:q=secret&scope=C%3A%5CPrivate&crumb=location:C%3A%5COther",
            ] {
                assert_eq!(
                    parse_search_uri(uri),
                    None,
                    "conflicting scope parameters were accepted: {uri}"
                );
            }
            assert_eq!(
                parse_search_uri("search:q=secret&scope=C%3A%5CPrivate&location=C%3A%5CPrivate"),
                Some(SearchRequest {
                    query: Some("secret".into()),
                    scope: Some(r"C:\Private".into())
                }),
                "repeating the same normalized location must stay compatible"
            );
            assert_eq!(
                parse_search_uri(
                    "searchtool:q=secret&scope=C%3A%5CPrivate&location=c%3A%5Cprivate"
                ),
                Some(SearchRequest {
                    query: Some("secret".into()),
                    scope: Some(r"c:\private".into()),
                }),
                "case-insensitive Windows location repeats must stay compatible"
            );
            assert_eq!(
                parse_search_uri("search:q=secret&scope=&location="),
                Some(SearchRequest {
                    query: Some("secret".into()),
                    scope: None
                }),
                "two explicitly blank global search scopes must stay compatible"
            );
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
        fn cli_explicit_invalid_scope_or_uri_cannot_turn_restricted_search_global() {
            let original = SearchRequest {
                query: Some("secret".into()),
                scope: Some(r"C:\Private".into()),
            };
            for scope in ["C:", "C:Private", "\\", "  "] {
                let mut request = original.clone();
                assert!(
                    apply_cli_scope(&mut request, scope).is_err(),
                    "CLI accepted invalid --scope={scope:?}"
                );
                assert_eq!(
                    request, original,
                    "invalid CLI scope changed the prior Search request"
                );
            }
            for uri in [
                "search:q=secret&scope=C%3A",
                "search:q=secret&scope=%5C",
                "search:q=secret&scope=C%3A%5CPrivate&location=",
                "search:",
            ] {
                let mut request = original.clone();
                assert!(
                    apply_cli_search_uri(&mut request, uri).is_err(),
                    "CLI ignored invalid search URI: {uri}"
                );
                assert_eq!(
                    request, original,
                    "invalid search URI changed previous Search request"
                );
            }
            let mut valid = original;
            apply_cli_scope(&mut valid, r"D:\Documents").unwrap();
            assert_eq!(valid.scope.as_deref(), Some(r"D:\Documents"));
            apply_cli_search_uri(&mut valid, "search:q=public&scope=").unwrap();
            assert_eq!(valid.query.as_deref(), Some("public"));
            assert_eq!(
                valid.scope, None,
                "explicitly global valid URI stays supported"
            );
            apply_cli_search_uri(&mut valid, "search:just%20text").unwrap();
            assert_eq!(
                valid.query.as_deref(),
                Some("just text"),
                "documented unkeyed URI query remains valid"
            );
        }

        #[test]
        fn explicit_scopes_with_dot_segments_must_fail_before_search() {
            // A lexical folder filter must not accept an unresolved path
            // that Windows could normalize into a different directory.
            for unsafe_scope in [
                r"C:\Private\..\Public",
                r"C:\Private\.\Reports",
                r"C:\Private\..",
                r"\\server\share\..\other",
                r"\\server\share\.\reports",
            ] {
                assert_eq!(
                    normalize_scope(unsafe_scope),
                    None,
                    "accepted unresolved scope {unsafe_scope:?}"
                );
                assert_eq!(validated_explicit_scope(unsafe_scope), None);
                let mut packet = "secret".encode_utf16().collect::<Vec<u16>>();
                packet.push(0);
                packet.extend(unsafe_scope.encode_utf16());
                packet.push(0);
                assert_eq!(
                    decode_ipc_request(&packet),
                    None,
                    "IPC accepted unresolved scope {unsafe_scope:?}"
                );
            }
            assert_eq!(
                parse_search_uri("search:q=secret&scope=C%3A%5CPrivate%5C..%5CPublic"),
                None
            );
            assert_eq!(
                normalize_scope(r"C:\Private\.config"),
                Some(r"C:\Private\.config".into())
            );
            assert_eq!(
                normalize_scope(r"C:\Private\release..txt"),
                Some(r"C:\Private\release..txt".into())
            );
            assert_eq!(
                normalize_scope(r"\\server\share\.config"),
                Some(r"\\server\share\.config".into())
            );
        }

        #[test]
        fn relative_or_incomplete_unc_scope_must_not_match_unrelated_locations() {
            // Scope boundaries must name an absolute directory. Treating a
            // bare folder or drive-root-relative path as a lexical substring
            // could search unrelated indexed volumes or working directories.
            for scope in [
                "Private",
                ".\\Private",
                "..\\Private",
                r"\Private",
                "/Private",
                r"\\server",
                r"\\server\",
                r"\\server\\share",
                r"\\\share",
            ] {
                assert_eq!(
                    normalize_scope(scope),
                    None,
                    "unsafe scope normalized: {scope:?}"
                );
                assert_eq!(validated_explicit_scope(scope), None);
                let mut ipc = "secret".encode_utf16().collect::<Vec<u16>>();
                ipc.push(0);
                ipc.extend(scope.encode_utf16());
                ipc.push(0);
                assert_eq!(
                    decode_ipc_request(&ipc),
                    None,
                    "IPC accepted unsafe scope {scope:?}"
                );
            }
            assert_eq!(parse_search_uri("search:q=secret&scope=Private"), None);
            assert_eq!(parse_search_uri("search:q=secret&scope=%5CPrivate"), None);
            assert_eq!(normalize_scope(r"C:\Private"), Some(r"C:\Private".into()));
            assert_eq!(
                normalize_scope(r"\\server\share"),
                Some(r"\\server\share".into())
            );
            assert_eq!(
                normalize_scope(r"\\server\share\folder"),
                Some(r"\\server\share\folder".into())
            );
        }

        #[test]
        fn drive_relative_scope_must_not_expand_to_volume_root() {
            // Windows C: denotes the drive's working directory, NOT C:\.
            // Using it as the lexical search scope admits all C:\ paths.
            for unsafe_scope in ["C:", "c:Private", "D:docs\\reports"] {
                assert_eq!(
                    normalize_scope(unsafe_scope),
                    None,
                    "drive-relative scope accepted: {unsafe_scope}"
                );
                assert_eq!(validated_explicit_scope(unsafe_scope), None);
                let mut ipc = "secret".encode_utf16().collect::<Vec<_>>();
                ipc.push(0);
                ipc.extend(unsafe_scope.encode_utf16());
                ipc.push(0);
                assert_eq!(
                    decode_ipc_request(&ipc),
                    None,
                    "drive-relative IPC scope accepted"
                );
            }
            assert_eq!(parse_search_uri("search:q=secret&scope=C%3A"), None);
            assert_eq!(parse_search_uri("search:q=secret&scope=C%3APrivate"), None);
            assert_eq!(normalize_scope(r"C:\"), Some(r"C:\".into()));
            assert_eq!(normalize_scope("C:/Private"), Some(r"C:\Private".into()));
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
        fn cancelled_shell_bridge_begin_cannot_reopen_search() {
            assert!(
                !bridge_begin_is_current(false, 1, 1),
                "cancelled bridge request revived Search"
            );
            assert!(
                bridge_begin_is_current(true, 2, 2),
                "active bridge request was rejected"
            );
            assert!(
                !bridge_begin_is_current(true, 1, 2),
                "old BEGIN displaced a newer active Search request"
            );
            assert!(
                !bridge_begin_is_current(true, 0, 0),
                "untagged BEGIN must never be trusted"
            );
        }

        #[test]
        fn stale_shell_bridge_char_and_key_cannot_affect_new_session() {
            assert!(
                bridge_input_is_current(6, 6),
                "current input must be preserved"
            );
            assert!(
                !bridge_input_is_current(5, 6),
                "stale event could change a newer search"
            );
            assert!(
                !bridge_input_is_current(0, 6),
                "untagged event could change a search"
            );
        }

        #[test]
        fn cancelled_bridge_escape_cannot_dismiss_manually_reopened_search() {
            assert!(bridge_input_belongs_to_displayed_session(8, 8, 8));
            assert!(
                !bridge_input_belongs_to_displayed_session(8, 8, 0),
                "cancelled old Escape could close a manually reopened Search popup"
            );
            assert!(!bridge_input_belongs_to_displayed_session(7, 8, 8));
            assert!(!bridge_input_belongs_to_displayed_session(0, 8, 8));
        }

        #[test]
        fn failed_old_begin_post_cannot_clear_newer_explorer_scope() {
            assert!(bridge_failed_post_may_clear_scope(3, 3));
            assert!(
                !bridge_failed_post_may_clear_scope(3, 4),
                "failed old PostMessage can erase newer Explorer scope"
            );
            assert!(!bridge_failed_post_may_clear_scope(0, 4));
        }

        #[test]
        fn empty_translated_bridge_key_must_not_be_swallowed() {
            let mut sent = Vec::new();
            assert!(post_bridge_utf16_units(&['A' as u16, 'B' as u16], |unit| {
                sent.push(unit);
                true
            }));
            assert_eq!(sent, ['A' as u32, 'B' as u32]);
            assert!(
                !post_bridge_utf16_units(&[], |_unit| true),
                "dead-key or untranslated key would be silently swallowed"
            );
            let mut attempts = 0;
            assert!(!post_bridge_utf16_units(
                &['A' as u16, 'B' as u16],
                |_unit| {
                    attempts += 1;
                    false
                }
            ));
            assert_eq!(attempts, 1, "posting should stop at first failed unit");
        }

        #[test]
        fn shell_bridge_unicode_scalar_posts_surrogate_pair_atomically() {
            let emoji: Vec<u16> = "📁".encode_utf16().collect();
            assert_eq!(emoji.len(), 2);
            let mut posted = Vec::new();
            assert!(post_bridge_utf16_units(&emoji, |value| {
                posted.push(value);
                true
            }));
            assert_eq!(
                posted,
                vec![0x1F4C1],
                "a surrogate pair must be posted as ONE Unicode character"
            );
            let mut invalid_posts = 0;
            assert!(!post_bridge_utf16_units(&[0xD83D], |_value| {
                invalid_posts += 1;
                true
            }));
            assert_eq!(
                invalid_posts, 0,
                "unpaired surrogate must never post a partial character"
            );
            let mut invalid_suffix_posts = 0;
            assert!(!post_bridge_utf16_units(&[b'A' as u16, 0xD83D], |_value| {
                invalid_suffix_posts += 1;
                true
            }));
            assert_eq!(
                invalid_suffix_posts, 0,
                "invalid translation cannot partially post its valid prefix"
            );
        }

        #[test]
        fn bridge_key_payload_preserves_vk_scan_and_modifiers() {
            for (vk, scan, extended, ctrl) in [
                (VK_RETURN as u32, 0x1C, false, false),
                (VK_ESCAPE as u32, 0x01, false, false),
                (VK_LEFT, 0x4B, true, false),
                (b'V' as u32, 0x2F, false, true),
                (VK_DELETE, 0x53, true, true),
            ] {
                let packed = pack_bridge_key_payload(vk, scan, extended, ctrl);
                assert_eq!(bridge_key_virtual_key(packed), vk);
                assert_eq!((packed as u32) & 0xFFFF, scan);
                assert_eq!((packed & (1 << 16)) != 0, extended);
                assert_eq!((packed & SHELL_BRIDGE_KEY_CONTROL) != 0, ctrl);
            }
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
        fn owner_draw_never_substitutes_another_results_row() {
            assert_eq!(verified_draw_result_index(0, 0, 3), Some(0));
            assert_eq!(verified_draw_result_index(2, 2, 3), Some(2));
            assert_eq!(verified_draw_result_index(0, 1, 3), None);
            assert_eq!(verified_draw_result_index(1, 0, 3), None);
            assert_eq!(verified_draw_result_index(u32::MAX, 0, 3), None);
            assert_eq!(verified_draw_result_index(3, 3, 3), None);
            assert_eq!(verified_draw_result_index(0, usize::MAX, 3), None);
            assert_eq!(verified_draw_result_index(0, 0, 0), None);
        }

        #[test]
        fn hiding_detail_open_chooses_only_visible_focus_targets() {
            let popup = menu_id(91);
            let edit = menu_id(92);
            let list = menu_id(93);
            assert_eq!(
                hidden_detail_focus_target(true, true, list, edit, popup),
                list
            );
            assert_eq!(
                hidden_detail_focus_target(true, false, list, edit, popup),
                list
            );
            assert_eq!(
                hidden_detail_focus_target(false, true, list, edit, popup),
                edit
            );
            assert_eq!(
                hidden_detail_focus_target(false, false, list, edit, popup),
                popup
            );
        }

        #[test]
        fn clipped_navigation_and_results_never_focus_a_hidden_query() {
            let popup = menu_id(91);
            let edit = menu_id(92);
            assert_eq!(clipped_focus_target(false, edit, popup), popup);
            assert_eq!(clipped_focus_target(true, edit, popup), edit);
        }

        #[test]
        fn dismissed_resident_keyboard_queue_never_routes_old_text_or_enter() {
            assert!(should_drop_dismissed_popup_input(
                true, false, WM_KEYDOWN, true
            ));
            assert!(should_drop_dismissed_popup_input(true, false, 0x0102, true));
            assert!(should_drop_dismissed_popup_input(
                true, false, WM_KEYUP, true
            ));
            assert!(!should_drop_dismissed_popup_input(
                true, true, WM_KEYDOWN, true
            ));
            assert!(!should_drop_dismissed_popup_input(
                false, false, WM_KEYDOWN, true
            ));
            assert!(!should_drop_dismissed_popup_input(
                true, false, WM_HOTKEY, true
            ));
            assert!(!should_drop_dismissed_popup_input(
                true,
                false,
                WM_COPYDATA,
                true
            ));
            assert!(!should_drop_dismissed_popup_input(
                true, false, WM_KEYDOWN, false
            ));
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
        fn compact_result_cards_reserve_actual_filename_width_at_dpi() {
            for dpi in [96, 120, 144] {
                let compact = Rect {
                    left: 0,
                    top: 0,
                    right: scale_px(112, dpi),
                    bottom: scale_px(56, dpi),
                };
                let (decorated, title, path) = result_row_text_geometry(&compact, dpi, false);
                assert!(!decorated, "icons/badges consume compact filename space");
                assert_eq!(title.left, scale_px(12, dpi));
                assert_eq!(title.right, compact.right - scale_px(12, dpi));
                assert!(title.right - title.left >= scale_px(80, dpi));
                assert_eq!(path.left, title.left);
                assert!(path.right > path.left);
                let large = Rect {
                    right: scale_px(320, dpi),
                    ..compact
                };
                let (decorated, title, _) = result_row_text_geometry(&large, dpi, false);
                assert!(
                    decorated,
                    "normal-size cards should retain type badge and icon"
                );
                assert_eq!(title.left, scale_px(36, dpi));
                assert!(title.right > title.left);
            }
        }

        #[test]
        fn appearance_button_requires_readable_width_at_dpi() {
            for dpi in [96, 120, 144] {
                for (compact, minimum) in [(true, 32), (false, 94)] {
                    let width = scale_px(minimum, dpi);
                    assert!(appearance_button_has_room(width, compact, dpi));
                    assert!(!appearance_button_has_room(width - 1, compact, dpi));
                    assert!(!appearance_button_has_room(1, compact, dpi));
                }
            }
        }

        #[test]
        fn category_chip_requires_readable_width_at_dpi() {
            for dpi in [96, 120, 144] {
                for minimum in [64, 94] {
                    let width = scale_px(minimum, dpi);
                    assert!(category_chip_has_room(width, minimum, dpi));
                    assert!(!category_chip_has_room(width - 1, minimum, dpi));
                    assert!(!category_chip_has_room(1, minimum, dpi));
                }
            }
        }

        #[test]
        fn query_input_needs_readable_interior_width_at_dpi() {
            for dpi in [96, 120, 144] {
                for border in [scale_px(1, dpi), scale_px(2, dpi)] {
                    let minimum = scale_px(120, dpi) + 2 * border;
                    assert!(query_edit_has_room(minimum, border, dpi));
                    assert!(!query_edit_has_room(minimum - 1, border, dpi));
                    assert!(!query_edit_has_room(1, border, dpi));
                }
            }
        }

        #[test]
        fn result_list_needs_minimum_readable_width_at_dpi() {
            for dpi in [96, 120, 144] {
                let tall = Rect {
                    left: 0,
                    top: 0,
                    right: scale_px(24, dpi) * 2 + scale_px(120, dpi),
                    bottom: scale_px(720, dpi),
                };
                assert!(native_result_list_has_room(tall, dpi, 56));
                assert!(!native_result_list_has_room(
                    Rect {
                        right: tall.right - 1,
                        ..tall
                    },
                    dpi,
                    56
                ));
                let classic = Rect {
                    right: scale_px(MARGIN, dpi) * 2 + scale_px(120, dpi),
                    ..tall
                };
                assert!(classic_result_list_has_room(classic, dpi, 56));
                assert!(!classic_result_list_has_room(
                    Rect {
                        right: classic.right - 1,
                        ..classic
                    },
                    dpi,
                    56
                ));
            }
        }

        #[test]
        fn classic_result_visibility_requires_one_complete_row_at_dpi() {
            for dpi in [96, 120, 144] {
                let short = Rect {
                    left: 0,
                    top: 0,
                    right: scale_px(780, dpi),
                    bottom: scale_px(170, dpi),
                };
                let tall = Rect {
                    bottom: scale_px(720, dpi),
                    ..short
                };
                assert!(!classic_result_list_has_room(short, dpi, 56));
                assert!(classic_result_list_has_room(tall, dpi, 56));
                // The classic 4/2/1-column choice and list height must share
                // the exact same DPI-rounded category-row geometry.
                assert_eq!(classic_category_columns(scale_px(410, dpi), dpi), 4);
                assert_eq!(classic_category_columns(scale_px(320, dpi), dpi), 2);
                assert_eq!(classic_category_columns(scale_px(170, dpi), dpi), 1);
                assert_eq!(
                    classic_stacked_header_offset(
                        Rect {
                            left: 0,
                            top: 0,
                            right: scale_px(210, dpi),
                            bottom: scale_px(720, dpi)
                        },
                        dpi
                    ),
                    scale_px(8, dpi) + scale_px(34, dpi)
                );
                assert_eq!(
                    classic_stacked_header_offset(
                        Rect {
                            left: 0,
                            top: 0,
                            right: scale_px(360, dpi),
                            bottom: scale_px(720, dpi)
                        },
                        dpi
                    ),
                    0
                );
                for (width, short_height, tall_height) in [(360, 310, 360), (210, 390, 480)] {
                    let narrow = Rect {
                        left: 0,
                        top: 0,
                        right: scale_px(width, dpi),
                        bottom: scale_px(short_height, dpi),
                    };
                    assert!(!classic_result_list_has_room(narrow, dpi, 56));
                    assert!(classic_result_list_has_room(
                        Rect {
                            bottom: scale_px(tall_height, dpi),
                            ..narrow
                        },
                        dpi,
                        56
                    ));
                }
            }
        }

        #[test]
        fn native_result_visibility_follows_row_space_after_two_row_reflow() {
            // The same pure geometry check drives WM_SIZE and query refresh.
            // Compact 2x2 and 4x1 category layouts consume extra rows.
            for (width, short_height, tall_height, dpi) in [
                (780, 170, 470, 96),
                (360, 250, 290, 96),
                (450, 320, 360, 120),
                (210, 350, 390, 96),
                (263, 445, 500, 120),
            ] {
                let short = Rect {
                    left: 0,
                    top: 0,
                    right: width,
                    bottom: short_height,
                };
                let tall = Rect {
                    bottom: tall_height,
                    ..short
                };
                assert!(!native_result_list_has_room(short, dpi, 56));
                assert!(native_result_list_has_room(tall, dpi, 56));
            }
            assert_eq!(native_category_columns(122, 96), 1);
            assert_eq!(native_category_columns(168, 96), 2);
            assert_eq!(native_category_columns(272, 96), 2);
            assert_eq!(native_category_columns(344, 96), 4);
            assert_eq!(native_category_columns(153, 120), 1);
            assert!(native_result_list_has_room(
                Rect {
                    left: 0,
                    top: 0,
                    right: 780,
                    bottom: 720
                },
                96,
                56
            ));
        }

        #[test]
        fn native_details_collapse_before_path_and_open_button_overlap() {
            // At 96 DPI: detail path ends 242px below the column top;
            // Open sits 60px above its bottom. Preserve a 16px gap.
            // The old 190px minimum allowed overlapping controls.
            let short = Rect {
                left: 0,
                top: 0,
                right: 780,
                bottom: 470,
            };
            assert!(native_result_columns(short, 96).is_none());
            let tall = Rect {
                bottom: 500,
                ..short
            };
            let (_, detail) = native_result_columns(tall, 96).expect("minimum useful detail");
            assert!(detail.bottom - detail.top >= 318);

            let short_125 = Rect {
                left: 0,
                top: 0,
                right: 975,
                bottom: 595,
            };
            assert!(native_result_columns(short_125, 120).is_none());
            let tall_125 = Rect {
                bottom: 700,
                ..short_125
            };
            let (_, detail) = native_result_columns(tall_125, 120).expect("125% detail");
            let path_bottom = detail.top + scale_px(172, 120) + scale_px(70, 120);
            let open_top = detail.bottom - scale_px(20, 120) - scale_px(40, 120);
            assert!(path_bottom + scale_px(16, 120) <= open_top);
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
