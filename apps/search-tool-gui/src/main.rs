#![cfg_attr(windows, windows_subsystem = "windows")]

mod theme;

#[cfg(not(windows))]
fn main() {
    eprintln!("search-tool-gui is only available on Windows");
}

#[cfg(windows)]
mod windows_app {
    use crate::theme::{self, Backdrop, Palette, ThemeMode, UiTheme};
    use search_core::{
        content_terms, parse_search_query, query_subject, relation_for_query, ItemTypeFilter,
        MultiLiveSearchStore, QueryIntent, TinyIntentModel, FLAG_DIRECTORY,
    };
    use std::{env, ffi::c_void, io, path::PathBuf, ptr::null_mut, slice, time::Instant};

    type Hwnd = *mut c_void;
    type Hinstance = *mut c_void;
    type Hicon = *mut c_void;
    type Hcursor = *mut c_void;
    type Hbrush = *mut c_void;
    type Hfont = *mut c_void;
    type Hdc = *mut c_void;
    type Hgdiobj = *mut c_void;
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

    const SW_HIDE: i32 = 0;
    const SW_SHOW: i32 = 5;
    const SW_RESTORE: i32 = 9;
    const SW_SHOWNORMAL: i32 = 1;

    const WM_NCCREATE: u32 = 0x0081;
    const WM_NCDESTROY: u32 = 0x0082;
    const WM_CREATE: u32 = 0x0001;
    const WM_DESTROY: u32 = 0x0002;
    const WM_SIZE: u32 = 0x0005;
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
    const WM_SETFONT: u32 = 0x0030;

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

    const EM_SETCUEBANNER: u32 = 0x1501;

    const ODS_SELECTED: u32 = 0x0001;
    const DT_LEFT: u32 = 0x0000;
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
    const VK_SPACE: u32 = 0x20;
    const VK_ESCAPE: usize = 0x1B;
    const VK_RETURN: usize = 0x0D;
    const VK_DOWN: usize = 0x28;

    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
    const ERROR_ALREADY_EXISTS: u32 = 183;

    const LWA_ALPHA: u32 = 0x0000_0002;
    const SPI_GETWORKAREA: u32 = 0x0030;
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

    const ID_EDIT: usize = 1;
    const ID_LIST: usize = 2;
    const ID_TITLE: usize = 3;
    const ID_STATUS: usize = 4;
    const ID_ALL: usize = 10;
    const ID_FILES: usize = 11;
    const ID_FOLDERS: usize = 12;
    const ID_CONTENT: usize = 13;
    const ID_THEME: usize = 14;

    const MARGIN: i32 = 18;
    const TITLE_HEIGHT: i32 = 28;
    const SEARCH_HEIGHT: i32 = 44;
    const TAB_HEIGHT: i32 = 32;
    const STATUS_HEIGHT: i32 = 24;
    const RESULT_ROW_HEIGHT: u32 = 58;

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
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
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

    #[link(name = "kernel32")]
    extern "system" {
        #[link_name = "GetModuleHandleW"]
        fn get_module_handle_w(module_name: *const u16) -> Hinstance;
        #[link_name = "CreateMutexW"]
        fn create_mutex_w(
            security_attributes: *mut c_void,
            initial_owner: i32,
            name: *const u16,
        ) -> *mut c_void;
        #[link_name = "GetLastError"]
        fn get_last_error() -> u32;
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
        #[link_name = "FindWindowW"]
        fn find_window_w(class_name: *const u16, window_name: *const u16) -> Hwnd;
        #[link_name = "SetForegroundWindow"]
        fn set_foreground_window(hwnd: Hwnd) -> i32;
        #[link_name = "SetProcessDpiAwarenessContext"]
        fn set_process_dpi_awareness_context(value: isize) -> i32;
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

    struct State {
        store: MultiLiveSearchStore,
        edit: Hwnd,
        list: Hwnd,
        title: Hwnd,
        status: Hwnd,
        tabs: [Hwnd; 4],
        theme_button: Hwnd,
        resident: bool,
        hotkey_registered: bool,
        intent_model: Option<TinyIntentModel>,
        model_path: PathBuf,
        initial_query: Option<String>,
        mode: SearchMode,
        results: Vec<ResultRow>,
        theme: UiTheme,
        palette: Palette,
        dark: bool,
        background_brush: Hbrush,
        surface_brush: Hbrush,
        accent_brush: Hbrush,
        ui_font: Hfont,
        title_font: Hfont,
        small_font: Hfont,
    }

    impl Drop for State {
        fn drop(&mut self) {
            unsafe {
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
            let _ = set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        theme::ensure_default_config();
        let ui_theme = UiTheme::load();
        let dark = match ui_theme.mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => system_prefers_dark(),
        };
        let palette = ui_theme.palette(dark);

        let mut resident = false;
        let mut smoke = false;
        let mut index_source = None;
        let mut initial_query = None;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--resident" => resident = true,
                "--smoke" => smoke = true,
                "--query" => {
                    if let Some(value) = args.next() {
                        initial_query = Some(value);
                    }
                }
                "--search-uri" => {
                    if let Some(value) = args.next() {
                        initial_query = parse_search_uri(&value);
                    }
                }
                _ if arg.starts_with("search:") || arg.starts_with("searchtool:") => {
                    initial_query = parse_search_uri(&arg);
                }
                _ if index_source.is_none() => index_source = Some(PathBuf::from(arg)),
                _ => {}
            }
        }
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
                if let Some(query) = initial_query.as_deref() {
                    unsafe { send_query_to_existing(existing, query) };
                }
                unsafe {
                    show_window(existing, SW_RESTORE);
                    set_foreground_window(existing);
                }
            }
            drop(mutex);
            return Ok(());
        }

        let store = if index_source.is_dir() {
            MultiLiveSearchStore::open_index_directory(&index_source)?
        } else {
            MultiLiveSearchStore::open_index(&index_source)?
        };

        let font_face = wide("Segoe UI Variable Text");
        let title_face = wide("Segoe UI Variable Display");
        let ui_font = unsafe {
            create_font_w(
                -18,
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
            )
        };
        let title_font = unsafe {
            create_font_w(
                -22,
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
            )
        };
        let small_font = unsafe {
            create_font_w(
                -14,
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
            )
        };

        let background_brush = unsafe { create_solid_brush(palette.background.colorref()) };
        let surface_brush = unsafe { create_solid_brush(palette.surface.colorref()) };
        let accent_brush = unsafe { create_solid_brush(palette.accent.colorref()) };
        if background_brush.is_null()
            || surface_brush.is_null()
            || accent_brush.is_null()
            || ui_font.is_null()
            || title_font.is_null()
            || small_font.is_null()
        {
            return Err(io::Error::last_os_error());
        }

        let mut state = Box::new(State {
            store,
            edit: null_mut(),
            list: null_mut(),
            title: null_mut(),
            status: null_mut(),
            tabs: [null_mut(); 4],
            theme_button: null_mut(),
            resident,
            hotkey_registered: false,
            intent_model: None,
            model_path: default_model_path(),
            initial_query,
            mode: SearchMode::All,
            results: Vec::new(),
            theme: ui_theme,
            palette,
            dark,
            background_brush,
            surface_brush,
            accent_brush,
            ui_font,
            title_font,
            small_font,
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
            center_search_window(hwnd, (*raw_state).theme.width, (*raw_state).theme.height);

            if smoke || ((*raw_state).resident && (*raw_state).initial_query.is_none()) {
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
                            show_window(hwnd, SW_HIDE);
                            (*state_ptr).intent_model = None;
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

                if let Some(query) = state.initial_query.take() {
                    set_query(state, &query);
                }
                set_focus(state.edit);
                0
            }
            WM_SIZE if !state_ptr.is_null() => {
                resize_controls(hwnd, &mut *state_ptr);
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
                        open_theme_config(hwnd, state);
                        return 0;
                    }
                }
                if source == state.list && notification == LBN_DBLCLK {
                    let _ = open_selected(hwnd, state);
                }
                0
            }
            WM_HOTKEY if !state_ptr.is_null() && w_param as i32 == HOTKEY_ID => {
                let state = &mut *state_ptr;
                if is_window_visible(hwnd) != 0 {
                    show_window(hwnd, SW_HIDE);
                    state.intent_model = None;
                } else {
                    center_search_window(hwnd, state.theme.width, state.theme.height);
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
                if copy.dw_data == 1
                    && !copy.lp_data.is_null()
                    && bytes >= 2
                    && bytes <= ((MAX_QUERY_U16 as usize + 1) * 2)
                    && bytes.is_multiple_of(2)
                {
                    let words = slice::from_raw_parts(copy.lp_data as *const u16, bytes / 2);
                    let end = words
                        .iter()
                        .position(|&value| value == 0)
                        .unwrap_or(words.len());
                    let query = String::from_utf16_lossy(&words[..end]);
                    set_query(state, query.trim());
                    center_search_window(hwnd, state.theme.width, state.theme.height);
                    show_window(hwnd, SW_RESTORE);
                    set_foreground_window(hwnd);
                    set_focus(state.edit);
                    return 1;
                }
                0
            }
            WM_MEASUREITEM if !state_ptr.is_null() => {
                let measure = &mut *(l_param as *mut MeasureItemStruct);
                if measure.ctl_id as usize == ID_LIST {
                    measure.item_height = RESULT_ROW_HEIGHT;
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
                if screen_to_client(hwnd, &mut point) != 0 && point.y >= 0 && point.y < 38 {
                    return 2;
                }
                result
            }
            WM_CLOSE if !state_ptr.is_null() && (*state_ptr).resident => {
                let state = &mut *state_ptr;
                state.intent_model = None;
                show_window(hwnd, SW_HIDE);
                0
            }
            WM_DESTROY => {
                post_quit_message(0);
                0
            }
            WM_NCDESTROY => {
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
            wide("Search Tool").as_ptr(),
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
            wide("Dosya, klasör ve içerik ara").as_ptr(),
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
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
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
            wide("Tema").as_ptr(),
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
        send_message_w(state.status, WM_SETFONT, state.small_font as Wparam, 1);

        let cue = wide("Dosya, uygulama, klasör veya içerik ara");
        send_message_w(state.edit, EM_SETCUEBANNER, 1, cue.as_ptr() as Lparam);
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

        if state.theme.alpha() < 255 {
            let _ = set_layered_window_attributes(hwnd, 0, state.theme.alpha(), LWA_ALPHA);
        }
    }

    unsafe fn center_search_window(hwnd: Hwnd, width: i32, height: i32) {
        let mut work = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if system_parameters_info_w(SPI_GETWORKAREA, 0, (&mut work as *mut Rect).cast(), 0) == 0 {
            return;
        }
        let available_width = work.right - work.left;
        let available_height = work.bottom - work.top;
        let width = width.min(available_width.max(1));
        let height = height.min(available_height.max(1));
        let x = work.left + (available_width - width) / 2;
        let y = work.top + ((available_height - height) / 5).max(24);
        let _ = set_window_pos(hwnd, null_mut(), x, y, width, height, SWP_NOZORDER);
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
        let width = (rect.right - rect.left - MARGIN * 2).max(1);
        let title_y = 10;
        let search_y = title_y + TITLE_HEIGHT + 8;
        let tabs_y = search_y + SEARCH_HEIGHT + 10;
        let status_y = tabs_y + TAB_HEIGHT + 8;
        let list_y = status_y + STATUS_HEIGHT + 4;
        let list_height = (rect.bottom - list_y - MARGIN).max(1);

        move_window(
            state.title,
            MARGIN,
            title_y,
            (width - 96).max(1),
            TITLE_HEIGHT,
            1,
        );
        move_window(
            state.theme_button,
            MARGIN + (width - 84).max(0),
            title_y,
            84,
            TITLE_HEIGHT,
            1,
        );
        move_window(state.edit, MARGIN, search_y, width, SEARCH_HEIGHT, 1);

        let tab_gap = 8;
        let tab_width = 94;
        for (index, tab) in state.tabs.iter().enumerate() {
            move_window(
                *tab,
                MARGIN + index as i32 * (tab_width + tab_gap),
                tabs_y,
                tab_width,
                TAB_HEIGHT,
                1,
            );
        }
        move_window(state.status, MARGIN, status_y, width, STATUS_HEIGHT, 1);
        move_window(state.list, MARGIN, list_y, width, list_height, 1);
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
            set_status(state, "Dosya, klasör ve içerik ara");
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
            set_status(state, "Dosya, klasör ve içerik ara");
            return;
        }

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
            }
        }

        let count = state.results.len();
        set_status(
            state,
            &format!("{count} sonuç  •  {:.1} ms", elapsed.as_secs_f64() * 1000.0),
        );
        invalidate_rect(state.list, null_mut(), 0);
    }

    fn search_for_mode(
        state: &mut State,
        query: &str,
    ) -> io::Result<Vec<search_core::VolumeSearchHit>> {
        match state.mode {
            SearchMode::Content => {
                let terms = content_terms(query);
                state.store.search_content(&terms, MAX_RESULTS)
            }
            SearchMode::Files | SearchMode::Folders => {
                let mut parsed = parse_search_query(query);
                parsed.filters.item_type = Some(match state.mode {
                    SearchMode::Files => ItemTypeFilter::File,
                    SearchMode::Folders => ItemTypeFilter::Directory,
                    _ => unreachable!(),
                });
                state.store.search_filtered(&parsed, MAX_RESULTS, 100_000)
            }
            SearchMode::All => {
                let parsed = parse_search_query(query);
                if !parsed.filters.is_empty() {
                    state.store.search_filtered(&parsed, MAX_RESULTS, 100_000)
                } else if relation_for_query(&parsed.text).is_some() {
                    state.store.search_related(&parsed.text, MAX_RESULTS)
                } else if should_route_natural(query) {
                    route_natural_query(state, query)
                } else {
                    state.store.search_ranked(&parsed.text, MAX_RESULTS)
                }
            }
        }
    }

    unsafe fn set_status(state: &State, value: &str) {
        let value = wide(value);
        set_window_text_w(state.status, value.as_ptr());
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
        let brush = if selected {
            state.accent_brush
        } else {
            state.surface_brush
        };
        fill_rect(draw.hdc, &draw.rc_item, brush);
        set_bk_mode(draw.hdc, TRANSPARENT);

        let old_font = select_object(draw.hdc, state.ui_font as Hgdiobj);
        let title_color = if selected {
            state.palette.selected_text
        } else {
            state.palette.text
        };
        set_text_color(draw.hdc, title_color.colorref());

        let icon = if row.is_directory { "▣" } else { "•" };
        let title = wide(&format!("{icon}  {}", row.name));
        let mut title_rect = Rect {
            left: draw.rc_item.left + 12,
            top: draw.rc_item.top + 7,
            right: draw.rc_item.right - 12,
            bottom: draw.rc_item.top + 31,
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
        let path = wide(&row.path);
        let mut path_rect = Rect {
            left: draw.rc_item.left + 34,
            top: draw.rc_item.top + 31,
            right: draw.rc_item.right - 12,
            bottom: draw.rc_item.bottom - 5,
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
            set_status(
                state,
                "Tema ayarları açıldı • değişiklikler sonraki açılışta uygulanır",
            );
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

    fn route_natural_query(
        state: &mut State,
        query: &str,
    ) -> io::Result<Vec<search_core::VolumeSearchHit>> {
        if state.intent_model.is_none() {
            match TinyIntentModel::load(&state.model_path) {
                Ok(model) => state.intent_model = Some(model),
                Err(_) => {
                    let subject = query_subject(query);
                    return state.store.search_ranked(&subject, MAX_RESULTS);
                }
            }
        }
        let subject = query_subject(query);
        let Some(model) = state.intent_model.as_ref() else {
            return state.store.search_ranked(&subject, MAX_RESULTS);
        };
        let prediction = model.classify(query);
        match prediction.intent {
            QueryIntent::ContentSearch => {
                let terms = content_terms(query);
                state.store.search_content(&terms, MAX_RESULTS)
            }
            QueryIntent::RelatedSearch => state.store.search_related(&subject, MAX_RESULTS),
            QueryIntent::FuzzySearch => state.store.search_fuzzy(&subject, 2, MAX_RESULTS),
            QueryIntent::ExactSearch | QueryIntent::Unknown => {
                state.store.search_ranked(&subject, MAX_RESULTS)
            }
            QueryIntent::CleanupAnalysis | QueryIntent::WebLookup | QueryIntent::Help => {
                state.store.search_ranked(&subject, MAX_RESULTS)
            }
        }
    }

    unsafe fn send_query_to_existing(hwnd: Hwnd, query: &str) {
        let payload = wide(query);
        let copy = CopyDataStruct {
            dw_data: 1,
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

    fn parse_search_uri(uri: &str) -> Option<String> {
        let rest = uri
            .strip_prefix("search:")
            .or_else(|| uri.strip_prefix("searchtool:"))?;
        let rest = rest.trim_start_matches('?');
        for pair in rest.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if key.eq_ignore_ascii_case("query") || key.eq_ignore_ascii_case("q") {
                let decoded = percent_decode(value);
                if !decoded.trim().is_empty() {
                    return Some(decoded);
                }
            }
        }
        if !rest.contains('=') {
            let decoded = percent_decode(rest);
            if !decoded.trim().is_empty() {
                return Some(decoded);
            }
        }
        None
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
                Some("hello world".to_string())
            );
        }

        #[test]
        fn parse_search_uri_accepts_private_protocol_and_plus_spaces() {
            assert_eq!(
                parse_search_uri("searchtool:q=report+2026"),
                Some("report 2026".to_string())
            );
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
