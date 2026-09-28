#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("search-tool-gui is only available on Windows");
}

#[cfg(windows)]
mod windows_app {
    use search_core::{
        content_terms, parse_search_query, query_subject, relation_for_query, MultiLiveSearchStore,
        QueryIntent, TinyIntentModel,
    };
    use std::{env, ffi::c_void, io, path::PathBuf, ptr::null_mut};

    type Hwnd = *mut c_void;
    type Hinstance = *mut c_void;
    type Hicon = *mut c_void;
    type Hcursor = *mut c_void;
    type Hbrush = *mut c_void;
    type Lparam = isize;
    type Wparam = usize;
    type Lresult = isize;

    const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;
    const WS_CHILD: u32 = 0x4000_0000;
    const WS_TABSTOP: u32 = 0x0001_0000;
    const WS_VSCROLL: u32 = 0x0020_0000;
    const ES_AUTOHSCROLL: u32 = 0x0080;
    const LBS_NOTIFY: u32 = 0x0001;
    const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
    const CW_USEDEFAULT: i32 = i32::MIN;
    const SW_HIDE: i32 = 0;
    const SW_SHOW: i32 = 5;
    const SW_RESTORE: i32 = 9;
    const WM_NCCREATE: u32 = 0x0081;
    const WM_NCDESTROY: u32 = 0x0082;
    const WM_CREATE: u32 = 0x0001;
    const WM_DESTROY: u32 = 0x0002;
    const WM_SIZE: u32 = 0x0005;
    const WM_COMMAND: u32 = 0x0111;
    const WM_CLOSE: u32 = 0x0010;
    const WM_HOTKEY: u32 = 0x0312;
    const GWLP_USERDATA: i32 = -21;
    const EN_CHANGE: usize = 0x0300;
    const LB_ADDSTRING: u32 = 0x0180;
    const LB_RESETCONTENT: u32 = 0x0184;
    const COLOR_WINDOW: isize = 5;
    const IDC_ARROW: usize = 32512;
    const EDIT_HEIGHT: i32 = 36;
    const MARGIN: i32 = 10;
    const MAX_QUERY_U16: i32 = 512;
    const MAX_RESULTS: usize = 64;
    const HOTKEY_ID: i32 = 0x5345;
    const MOD_ALT: u32 = 0x0001;
    const MOD_CONTROL: u32 = 0x0002;
    const MOD_NOREPEAT: u32 = 0x4000;
    const VK_SPACE: u32 = 0x20;
    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
    const ERROR_ALREADY_EXISTS: u32 = 183;

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
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
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
        #[link_name = "SendMessageW"]
        fn send_message_w(hwnd: Hwnd, msg: u32, w_param: Wparam, l_param: Lparam) -> Lresult;
        #[link_name = "SetFocus"]
        fn set_focus(hwnd: Hwnd) -> Hwnd;
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
    }

    struct State {
        store: MultiLiveSearchStore,
        edit: Hwnd,
        list: Hwnd,
        resident: bool,
        hotkey_registered: bool,
        intent_model: Option<TinyIntentModel>,
        model_path: PathBuf,
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
        // Best effort on Windows 10+: keep Win32 controls crisp and correctly sized
        // when the window moves between monitors with different scaling factors.
        unsafe {
            let _ = set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let mut resident = false;
        let mut smoke = false;
        let mut index_source = None;
        for arg in env::args().skip(1) {
            if arg == "--resident" {
                resident = true;
            } else if arg == "--smoke" {
                smoke = true;
            } else if index_source.is_none() {
                index_source = Some(PathBuf::from(arg));
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
        let mut state = Box::new(State {
            store,
            edit: null_mut(),
            list: null_mut(),
            resident,
            hotkey_registered: false,
            intent_model: None,
            model_path: default_model_path(),
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
            background: (COLOR_WINDOW + 1) as Hbrush,
            menu_name: null_mut(),
            class_name: class_name.as_ptr(),
            icon_small: null_mut(),
        };
        if unsafe { register_class_ex_w(&class) } == 0 {
            return Err(io::Error::last_os_error());
        }

        let title = wide("Search Tool");
        let raw_state: *mut State = &mut *state;
        let hwnd = unsafe {
            create_window_ex_w(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                860,
                620,
                null_mut(),
                null_mut(),
                instance,
                raw_state.cast(),
            )
        };
        if hwnd.is_null() {
            return Err(io::Error::last_os_error());
        }

        // The window owns State after WM_NCCREATE. Avoid Box dropping it here.
        std::mem::forget(state);
        unsafe {
            if smoke {
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
                let instance = get_module_handle_w(null_mut());
                let edit_class = wide("EDIT");
                let list_class = wide("LISTBOX");
                let empty = wide("");
                state.edit = create_window_ex_w(
                    0,
                    edit_class.as_ptr(),
                    empty.as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL,
                    MARGIN,
                    MARGIN,
                    800,
                    EDIT_HEIGHT,
                    hwnd,
                    std::ptr::with_exposed_provenance_mut::<c_void>(1),
                    instance,
                    null_mut(),
                );
                state.list = create_window_ex_w(
                    0,
                    list_class.as_ptr(),
                    empty.as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_VSCROLL | LBS_NOTIFY | LBS_NOINTEGRALHEIGHT,
                    MARGIN,
                    MARGIN + EDIT_HEIGHT + MARGIN,
                    800,
                    520,
                    hwnd,
                    std::ptr::with_exposed_provenance_mut::<c_void>(2),
                    instance,
                    null_mut(),
                );
                if state.edit.is_null() || state.list.is_null() {
                    return -1;
                }
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
                }
                0
            }
            WM_HOTKEY if !state_ptr.is_null() && w_param as i32 == HOTKEY_ID => {
                let state = &mut *state_ptr;
                show_window(hwnd, SW_RESTORE);
                set_foreground_window(hwnd);
                // Refresh only when the user summons the resident window. This keeps
                // idle mode event-driven while ensuring an unchanged query does not
                // display a pre-USN/compaction result set indefinitely.
                refresh_results(state);
                if !state.edit.is_null() {
                    set_focus(state.edit);
                }
                0
            }
            WM_CLOSE if !state_ptr.is_null() && (*state_ptr).resident => {
                let state = &mut *state_ptr;
                // The tiny model is useful only while interacting with natural-language
                // queries. Release it when the resident UI is hidden so idle RAM stays low.
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
        let list_y = MARGIN + EDIT_HEIGHT + MARGIN;
        let list_height = (rect.bottom - list_y - MARGIN).max(1);
        move_window(state.edit, MARGIN, MARGIN, width, EDIT_HEIGHT, 1);
        move_window(state.list, MARGIN, list_y, width, list_height, 1);
    }

    unsafe fn refresh_results(state: &mut State) {
        send_message_w(state.list, LB_RESETCONTENT, 0, 0);
        let len = get_window_text_length_w(state.edit).clamp(0, MAX_QUERY_U16);
        if len == 0 {
            return;
        }
        let mut buffer = vec![0_u16; len as usize + 1];
        let copied = get_window_text_w(state.edit, buffer.as_mut_ptr(), len + 1);
        if copied <= 0 {
            return;
        }
        let query = String::from_utf16_lossy(&buffer[..copied as usize]);
        let query = query.trim();
        let parsed = parse_search_query(query);
        let hits = if !parsed.filters.is_empty() {
            state.store.search_filtered(&parsed, MAX_RESULTS, 100_000)
        } else if relation_for_query(&parsed.text).is_some() {
            state.store.search_related(&parsed.text, MAX_RESULTS)
        } else if should_route_natural(query) {
            route_natural_query(state, query)
        } else {
            state.store.search_ranked(&parsed.text, MAX_RESULTS)
        };
        let Ok(hits) = hits else {
            return;
        };
        for hit in hits {
            let label = state
                .store
                .reconstruct_path(&hit, 256)
                .unwrap_or_else(|_| format!("{}:\\{}", hit.volume, hit.hit.name));
            let wide_label = wide(&label);
            send_message_w(state.list, LB_ADDSTRING, 0, wide_label.as_ptr() as Lparam);
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
            // Cleanup and web lookup require explicit user actions. Typing in the search
            // box must never delete files or unexpectedly send a network request.
            QueryIntent::CleanupAnalysis | QueryIntent::WebLookup | QueryIntent::Help => {
                state.store.search_ranked(&subject, MAX_RESULTS)
            }
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
}

#[cfg(windows)]
fn main() {
    if let Err(error) = windows_app::run() {
        windows_app::show_error(&error.to_string());
        std::process::exit(1);
    }
}
