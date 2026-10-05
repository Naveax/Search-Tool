use std::{io, ptr::null_mut};

type Hkey = *mut core::ffi::c_void;
type Hwnd = *mut core::ffi::c_void;

const HKEY_CURRENT_USER: Hkey = 0x8000_0001usize as Hkey;
const RRF_RT_REG_DWORD: u32 = 0x0000_0018;
const REG_DWORD: u32 = 4;
const ERROR_FILE_NOT_FOUND: i32 = 2;

const HWND_BROADCAST: Hwnd = 0xffffusize as Hwnd;
const WM_SETTINGCHANGE: u32 = 0x001A;
const SMTO_ABORTIFHUNG: u32 = 0x0002;

const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const DWM: &str = r"Software\Microsoft\Windows\DWM";
const EXPLORER_ACCENT: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent";
const DESKTOP: &str = r"Control Panel\Desktop";

#[link(name = "advapi32")]
extern "system" {
    #[link_name = "RegGetValueW"]
    fn reg_get_value_w(
        hkey: Hkey,
        sub_key: *const u16,
        value: *const u16,
        flags: u32,
        value_type: *mut u32,
        data: *mut core::ffi::c_void,
        data_size: *mut u32,
    ) -> i32;

    #[link_name = "RegSetKeyValueW"]
    fn reg_set_key_value_w(
        hkey: Hkey,
        sub_key: *const u16,
        value: *const u16,
        value_type: u32,
        data: *const core::ffi::c_void,
        data_size: u32,
    ) -> i32;
}

#[link(name = "user32")]
extern "system" {
    #[link_name = "SendMessageTimeoutW"]
    fn send_message_timeout_w(
        hwnd: Hwnd,
        msg: u32,
        w_param: usize,
        l_param: isize,
        flags: u32,
        timeout_ms: u32,
        result: *mut usize,
    ) -> isize;
}

#[link(name = "dwmapi")]
extern "system" {
    #[link_name = "DwmGetColorizationColor"]
    fn dwm_get_colorization_color(color: *mut u32, opaque_blend: *mut i32) -> i32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeThemeState {
    pub apps_light: Option<bool>,
    pub system_light: Option<bool>,
    pub transparency: Option<bool>,
    pub accent_on_start_taskbar: Option<bool>,
    pub accent_on_titlebars: Option<bool>,
    pub colorization_color: Option<u32>,
    pub active_dwm_color: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeThemeMode {
    Light,
    Dark,
    SystemDarkAppsLight,
    SystemLightAppsDark,
}

pub fn native_theme_state() -> io::Result<NativeThemeState> {
    Ok(NativeThemeState {
        apps_light: read_dword(PERSONALIZE, "AppsUseLightTheme")?.map(|value| value != 0),
        system_light: read_dword(PERSONALIZE, "SystemUsesLightTheme")?.map(|value| value != 0),
        transparency: read_dword(PERSONALIZE, "EnableTransparency")?.map(|value| value != 0),
        accent_on_start_taskbar: read_dword(PERSONALIZE, "ColorPrevalance")?
            .map(|value| value != 0),
        accent_on_titlebars: read_dword(DWM, "ColorPrevalence")?.map(|value| value != 0),
        colorization_color: read_dword(DWM, "ColorizationColor")?,
        active_dwm_color: active_dwm_color(),
    })
}

pub fn set_native_theme_mode(mode: NativeThemeMode) -> io::Result<()> {
    let (system_light, apps_light) = match mode {
        NativeThemeMode::Light => (true, true),
        NativeThemeMode::Dark => (false, false),
        NativeThemeMode::SystemDarkAppsLight => (false, true),
        NativeThemeMode::SystemLightAppsDark => (true, false),
    };
    write_bool(PERSONALIZE, "SystemUsesLightTheme", system_light)?;
    write_bool(PERSONALIZE, "AppsUseLightTheme", apps_light)?;
    broadcast_theme_change();
    Ok(())
}

pub fn set_native_transparency(enabled: bool) -> io::Result<()> {
    write_bool(PERSONALIZE, "EnableTransparency", enabled)?;
    broadcast_theme_change();
    Ok(())
}

pub fn set_native_accent_visibility(start_taskbar: bool, titlebars: bool) -> io::Result<()> {
    write_bool(PERSONALIZE, "ColorPrevalance", start_taskbar)?;
    write_bool(DWM, "ColorPrevalence", titlebars)?;
    broadcast_theme_change();
    Ok(())
}

pub fn set_native_accent_auto(enabled: bool) -> io::Result<()> {
    write_bool(DESKTOP, "AutoColorization", enabled)?;
    broadcast_theme_change();
    Ok(())
}

pub fn set_native_accent_rgb(rgb: u32) -> io::Result<()> {
    let rgb = rgb & 0x00ff_ffff;
    let alpha = read_dword(DWM, "ColorizationColor")?
        .map(|value| value >> 24)
        .filter(|value| *value != 0)
        .unwrap_or(0xc4);
    let colorization = colorization_with_alpha(rgb, alpha);
    let abgr = rgb_to_abgr(rgb);

    write_bool(DESKTOP, "AutoColorization", false)?;
    write_dword(DWM, "ColorizationColor", colorization)?;
    write_dword(DWM, "AccentColor", abgr)?;
    write_dword(EXPLORER_ACCENT, "AccentColorMenu", abgr)?;
    broadcast_theme_change();
    Ok(())
}

fn colorization_with_alpha(rgb: u32, alpha: u32) -> u32 {
    ((alpha & 0xff) << 24) | (rgb & 0x00ff_ffff)
}

fn rgb_to_abgr(rgb: u32) -> u32 {
    let rgb = rgb & 0x00ff_ffff;
    let red = (rgb >> 16) & 0xff;
    let green = (rgb >> 8) & 0xff;
    let blue = rgb & 0xff;
    0xff00_0000 | (blue << 16) | (green << 8) | red
}

fn active_dwm_color() -> Option<u32> {
    let mut color = 0_u32;
    let mut opaque = 0_i32;
    let hr = unsafe { dwm_get_colorization_color(&mut color, &mut opaque) };
    (hr >= 0).then_some(color)
}

fn read_dword(sub_key: &str, value_name: &str) -> io::Result<Option<u32>> {
    let sub_key = wide(sub_key);
    let value_name = wide(value_name);
    let mut value = 0_u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        reg_get_value_w(
            HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&mut value as *mut u32).cast(),
            &mut size,
        )
    };
    match status {
        0 => Ok(Some(value)),
        ERROR_FILE_NOT_FOUND => Ok(None),
        code => Err(io::Error::from_raw_os_error(code)),
    }
}

fn write_bool(sub_key: &str, value_name: &str, value: bool) -> io::Result<()> {
    write_dword(sub_key, value_name, u32::from(value))
}

fn write_dword(sub_key: &str, value_name: &str, value: u32) -> io::Result<()> {
    let sub_key = wide(sub_key);
    let value_name = wide(value_name);
    let status = unsafe {
        reg_set_key_value_w(
            HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            value_name.as_ptr(),
            REG_DWORD,
            (&value as *const u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status))
    }
}

fn broadcast_theme_change() {
    for setting in ["ImmersiveColorSet", "WindowsThemeElement"] {
        let setting = wide(setting);
        let mut result = 0_usize;
        unsafe {
            let _ = send_message_timeout_w(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                setting.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                2_000,
                &mut result,
            );
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accent_encoding_matches_windows_formats() {
        assert_eq!(colorization_with_alpha(0x0078D7, 0xC4), 0xC40078D7);
        assert_eq!(rgb_to_abgr(0x0078D7), 0xFFD77800);
        assert_eq!(rgb_to_abgr(0x6A5ACD), 0xFFCD5A6A);
    }

    #[test]
    fn colorization_masks_out_of_range_bits() {
        assert_eq!(colorization_with_alpha(0xAB12_3456, 0x1C4), 0xC4123456);
        assert_eq!(rgb_to_abgr(0xFF12_3456), 0xFF563412);
    }
}
