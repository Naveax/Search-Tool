use std::{env, fs, io, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    Auto,
    Mica,
    Acrylic,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn colorref(self) -> u32 {
        self.r as u32 | ((self.g as u32) << 8) | ((self.b as u32) << 16)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiTheme {
    pub mode: ThemeMode,
    pub backdrop: Backdrop,
    pub accent: Rgb,
    pub background: Option<Rgb>,
    pub surface: Option<Rgb>,
    pub text: Option<Rgb>,
    pub muted: Option<Rgb>,
    pub opacity_percent: u8,
    pub width: i32,
    pub height: i32,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            mode: ThemeMode::System,
            backdrop: Backdrop::Acrylic,
            accent: Rgb::new(96, 205, 255),
            background: None,
            surface: None,
            text: None,
            muted: None,
            opacity_percent: 100,
            width: 820,
            height: 590,
        }
    }
}

impl UiTheme {
    pub fn load() -> Self {
        let mut theme = Self::default();
        let path = config_path();
        let Ok(content) = fs::read_to_string(path) else {
            return theme;
        };
        apply_config(&mut theme, &content);
        theme
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_config_text())
    }

    pub fn to_config_text(&self) -> String {
        let mode = match self.mode {
            ThemeMode::System => "system",
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        };
        let backdrop = match self.backdrop {
            Backdrop::Auto => "auto",
            Backdrop::Mica => "mica",
            Backdrop::Acrylic => "acrylic",
            Backdrop::None => "none",
        };
        let mut out = format!(
            "# Search Tool UI\ntheme={mode}\nbackdrop={backdrop}\naccent={}\nopacity={}\nwidth={}\nheight={}\n",
            rgb_hex(self.accent),
            self.opacity_percent,
            self.width,
            self.height,
        );
        for (key, value) in [
            ("background", self.background),
            ("surface", self.surface),
            ("text", self.text),
            ("muted", self.muted),
        ] {
            if let Some(value) = value {
                out.push_str(key);
                out.push('=');
                out.push_str(&rgb_hex(value));
                out.push('\n');
            }
        }
        out
    }

    pub fn palette(&self, dark: bool) -> Palette {
        let defaults = if dark {
            Palette {
                background: Rgb::new(32, 32, 32),
                surface: Rgb::new(43, 43, 43),
                text: Rgb::new(250, 250, 250),
                muted: Rgb::new(176, 176, 176),
                accent: self.accent,
                selected_text: Rgb::new(255, 255, 255),
            }
        } else {
            Palette {
                background: Rgb::new(243, 243, 243),
                surface: Rgb::new(255, 255, 255),
                text: Rgb::new(24, 24, 24),
                muted: Rgb::new(96, 96, 96),
                accent: self.accent,
                selected_text: Rgb::new(255, 255, 255),
            }
        };
        Palette {
            background: self.background.unwrap_or(defaults.background),
            surface: self.surface.unwrap_or(defaults.surface),
            text: self.text.unwrap_or(defaults.text),
            muted: self.muted.unwrap_or(defaults.muted),
            ..defaults
        }
    }

    pub fn alpha(&self) -> u8 {
        ((self.opacity_percent as u16 * 255 + 50) / 100) as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub background: Rgb,
    pub surface: Rgb,
    pub text: Rgb,
    pub muted: Rgb,
    pub accent: Rgb,
    pub selected_text: Rgb,
}

pub fn config_path() -> PathBuf {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SearchTool")
        .join("ui.conf")
}

pub fn ensure_default_config() {
    let path = config_path();
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let _ = fs::write(path, default_config_text());
}

pub fn default_config_text() -> &'static str {
    "# Search Tool UI\n\
# theme = system | dark | light\n\
theme=system\n\
# backdrop = acrylic | mica | none | auto\n\
backdrop=acrylic\n\
# accent and optional palette overrides use #RRGGBB\n\
accent=#60CDFF\n\
# background=#202020\n\
# surface=#2B2B2B\n\
# text=#FAFAFA\n\
# muted=#B0B0B0\n\
# whole-window opacity, 55..100\n\
opacity=100\n\
width=820\n\
height=590\n"
}

fn apply_config(theme: &mut UiTheme, content: &str) {
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "theme" => {
                theme.mode = match value.to_ascii_lowercase().as_str() {
                    "dark" => ThemeMode::Dark,
                    "light" => ThemeMode::Light,
                    _ => ThemeMode::System,
                };
            }
            "backdrop" => {
                theme.backdrop = match value.to_ascii_lowercase().as_str() {
                    "mica" => Backdrop::Mica,
                    "acrylic" => Backdrop::Acrylic,
                    "none" => Backdrop::None,
                    _ => Backdrop::Auto,
                };
            }
            "accent" => {
                if let Some(color) = parse_rgb(value) {
                    theme.accent = color;
                }
            }
            "background" => theme.background = parse_rgb(value),
            "surface" => theme.surface = parse_rgb(value),
            "text" => theme.text = parse_rgb(value),
            "muted" => theme.muted = parse_rgb(value),
            "opacity" => {
                if let Ok(value) = value.parse::<u8>() {
                    theme.opacity_percent = value.clamp(55, 100);
                }
            }
            "width" => {
                if let Ok(value) = value.parse::<i32>() {
                    theme.width = value.clamp(560, 1600);
                }
            }
            "height" => {
                if let Ok(value) = value.parse::<i32>() {
                    theme.height = value.clamp(360, 1200);
                }
            }
            _ => {}
        }
    }
}

fn rgb_hex(value: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", value.r, value.g, value.b)
}

fn parse_rgb(value: &str) -> Option<Rgb> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 {
        return None;
    }
    let raw = u32::from_str_radix(value, 16).ok()?;
    Some(Rgb::new(
        ((raw >> 16) & 0xff) as u8,
        ((raw >> 8) & 0xff) as u8,
        (raw & 0xff) as u8,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_theme_palette_and_bounds() {
        let mut theme = UiTheme::default();
        apply_config(
            &mut theme,
            "theme=dark\nbackdrop=mica\naccent=#112233\nopacity=20\nwidth=99\nheight=9999\n",
        );
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.backdrop, Backdrop::Mica);
        assert_eq!(theme.accent, Rgb::new(0x11, 0x22, 0x33));
        assert_eq!(theme.opacity_percent, 55);
        assert_eq!(theme.width, 560);
        assert_eq!(theme.height, 1200);
    }

    #[test]
    fn colorref_uses_win32_byte_order() {
        assert_eq!(Rgb::new(0x11, 0x22, 0x33).colorref(), 0x0033_2211);
    }

    #[test]
    fn serialized_theme_keeps_native_settings() {
        let theme = UiTheme {
            mode: ThemeMode::Dark,
            backdrop: Backdrop::Mica,
            accent: Rgb::new(0x11, 0x22, 0x33),
            opacity_percent: 75,
            width: 900,
            height: 640,
            ..UiTheme::default()
        };
        let text = theme.to_config_text();
        assert!(text.contains("theme=dark"));
        assert!(text.contains("backdrop=mica"));
        assert!(text.contains("accent=#112233"));
        assert!(text.contains("opacity=75"));
    }
}
