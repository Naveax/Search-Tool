#[cfg(windows)]
mod filter;
#[cfg(windows)]
mod ntfs;
#[cfg(windows)]
mod probe;
#[cfg(windows)]
mod scheduling;
#[cfg(windows)]
mod service;
#[cfg(windows)]
mod sync;
#[cfg(windows)]
mod theme;
#[cfg(windows)]
mod web;

#[cfg(windows)]
pub use filter::extract_filter_text;
#[cfg(windows)]
pub use ntfs::{
    EnumerationStats, JournalCheckpoint, JournalInfo, JournalReadStats, NtfsVolume, UsnRecordView,
};
#[cfg(windows)]
pub use probe::WindowsResourceProbe;
#[cfg(windows)]
pub use scheduling::enter_process_background_mode;
#[cfg(windows)]
pub use service::is_service_running;
#[cfg(windows)]
pub use sync::{
    rebuild_index, sync_index_bounded, sync_index_default, sync_index_once,
    usn_reconciliation_required, IndexSyncStats, InitialIndexStats,
};
#[cfg(windows)]
pub use theme::{
    native_theme_state, set_native_accent_auto, set_native_accent_rgb,
    set_native_accent_visibility, set_native_theme_mode, set_native_transparency, NativeThemeMode,
    NativeThemeState,
};
#[cfg(windows)]
pub use web::google_custom_search_json;

#[cfg(not(windows))]
#[derive(Debug, Default)]
pub struct WindowsResourceProbe;

#[cfg(not(windows))]
impl WindowsResourceProbe {
    pub fn new() -> Self {
        Self
    }

    pub fn sample(&mut self) -> search_core::ResourceSample {
        search_core::ResourceSample::default()
    }
}

#[cfg(not(windows))]
pub fn is_service_running(_service_name: &str) -> std::io::Result<bool> {
    Ok(false)
}

#[cfg(not(windows))]
pub fn enter_process_background_mode() -> std::io::Result<()> {
    Ok(())
}
