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
pub enum ThemePreset {
    Signature,
    Midnight,
    Graphite,
    Frost,
    Native,
}

impl ThemePreset {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Signature => "signature",
            Self::Midnight => "midnight",
            Self::Graphite => "graphite",
            Self::Frost => "frost",
            Self::Native => "native",
        }
    }

    fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "midnight" => Self::Midnight,
            "graphite" => Self::Graphite,
            "frost" => Self::Frost,
            "native" => Self::Native,
            _ => Self::Signature,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    Compact,
    Comfortable,
    Spacious,
}

impl Density {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Comfortable => "comfortable",
            Self::Spacious => "spacious",
        }
    }

    fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "compact" => Self::Compact,
            "spacious" => Self::Spacious,
            _ => Self::Comfortable,
        }
    }

    pub const fn row_height(self) -> i32 {
        match self {
            Self::Compact => 48,
            Self::Comfortable => 62,
            Self::Spacious => 76,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundFit {
    Fill,
    Fit,
    Stretch,
}

impl BackgroundFit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Fit => "fit",
            Self::Stretch => "stretch",
        }
    }

    fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "fit" => Self::Fit,
            "stretch" => Self::Stretch,
            _ => Self::Fill,
        }
    }
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
    pub preset: ThemePreset,
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
    pub density: Density,
    pub background_image: Option<PathBuf>,
    pub background_fit: BackgroundFit,
    pub background_image_opacity: u8,
}

impl Default for UiTheme {
    fn default() -> Self {
        let mut theme = Self {
            preset: ThemePreset::Signature,
            mode: ThemeMode::Dark,
            backdrop: Backdrop::Acrylic,
            accent: Rgb::new(68, 214, 255),
            background: Some(Rgb::new(10, 15, 28)),
            surface: Some(Rgb::new(20, 28, 46)),
            text: Some(Rgb::new(245, 249, 255)),
            muted: Some(Rgb::new(142, 160, 183)),
            opacity_percent: 96,
            width: 900,
            height: 640,
            density: Density::Comfortable,
            background_image: None,
            background_fit: BackgroundFit::Fill,
            background_image_opacity: 34,
        };
        theme.apply_preset(ThemePreset::Signature);
        theme
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

    pub fn apply_preset(&mut self, preset: ThemePreset) {
        self.preset = preset;
        match preset {
            ThemePreset::Signature => {
                self.mode = ThemeMode::Dark;
                self.backdrop = Backdrop::Acrylic;
                self.accent = Rgb::new(68, 214, 255);
                self.background = Some(Rgb::new(10, 15, 28));
                self.surface = Some(Rgb::new(20, 28, 46));
                self.text = Some(Rgb::new(245, 249, 255));
                self.muted = Some(Rgb::new(142, 160, 183));
                self.opacity_percent = 96;
            }
            ThemePreset::Midnight => {
                self.mode = ThemeMode::Dark;
                self.backdrop = Backdrop::None;
                self.accent = Rgb::new(149, 117, 255);
                self.background = Some(Rgb::new(7, 8, 18));
                self.surface = Some(Rgb::new(16, 18, 34));
                self.text = Some(Rgb::new(248, 247, 255));
                self.muted = Some(Rgb::new(155, 151, 181));
                self.opacity_percent = 100;
            }
            ThemePreset::Graphite => {
                self.mode = ThemeMode::Dark;
                self.backdrop = Backdrop::Mica;
                self.accent = Rgb::new(84, 225, 171);
                self.background = Some(Rgb::new(24, 26, 29));
                self.surface = Some(Rgb::new(35, 38, 42));
                self.text = Some(Rgb::new(245, 247, 248));
                self.muted = Some(Rgb::new(163, 169, 174));
                self.opacity_percent = 100;
            }
            ThemePreset::Frost => {
                self.mode = ThemeMode::Light;
                self.backdrop = Backdrop::Mica;
                self.accent = Rgb::new(42, 115, 230);
                self.background = Some(Rgb::new(235, 243, 252));
                self.surface = Some(Rgb::new(250, 252, 255));
                self.text = Some(Rgb::new(18, 33, 51));
                self.muted = Some(Rgb::new(88, 108, 128));
                self.opacity_percent = 98;
            }
            ThemePreset::Native => {
                self.mode = ThemeMode::System;
                self.backdrop = Backdrop::Auto;
                self.accent = Rgb::new(0, 120, 212);
                self.background = None;
                self.surface = None;
                self.text = None;
                self.muted = None;
                self.opacity_percent = 100;
            }
        }
    }

    pub fn reset_palette_overrides(&mut self) {
        self.background = None;
        self.surface = None;
        self.text = None;
        self.muted = None;
    }

    pub fn result_row_height(&self) -> i32 {
        self.density.row_height()
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
            "# Search Tool UI\npreset={}\ntheme={mode}\nbackdrop={backdrop}\naccent={}\nopacity={}\nwidth={}\nheight={}\ndensity={}\nbackground_fit={}\nbackground_image_opacity={}\n",
            self.preset.as_str(),
            rgb_hex(self.accent),
            self.opacity_percent,
            self.width,
            self.height,
            self.density.as_str(),
            self.background_fit.as_str(),
            self.background_image_opacity,
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
        if let Some(path) = self.background_image.as_ref() {
            out.push_str("background_image=");
            out.push_str(&path.to_string_lossy());
            out.push('\n');
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
# preset = signature | midnight | graphite | frost | native\n\
preset=signature\n\
# theme = system | dark | light\n\
theme=dark\n\
# backdrop = acrylic | mica | none | auto\n\
backdrop=acrylic\n\
# accent and optional palette overrides use #RRGGBB\n\
accent=#44D6FF\n\
background=#0A0F1C\n\
surface=#141C2E\n\
text=#F5F9FF\n\
muted=#8EA0B7\n\
# whole-window opacity, 55..100\n\
opacity=96\n\
width=900\n\
height=640\n\
# density = compact | comfortable | spacious\n\
density=comfortable\n\
# optional image: background_image=C:\\Pictures\\search-bg.png\n\
# background_fit = fill | fit | stretch\n\
background_fit=fill\n\
# reserved for image-overlay rendering, 0..100\n\
background_image_opacity=34\n"
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
            "preset" => theme.apply_preset(ThemePreset::parse(value)),
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
                    theme.width = value.clamp(620, 1800);
                }
            }
            "height" => {
                if let Ok(value) = value.parse::<i32>() {
                    theme.height = value.clamp(420, 1300);
                }
            }
            "density" => theme.density = Density::parse(value),
            "background_image" => {
                theme.background_image = if value.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(value))
                };
            }
            "background_fit" => theme.background_fit = BackgroundFit::parse(value),
            "background_image_opacity" => {
                if let Ok(value) = value.parse::<u8>() {
                    theme.background_image_opacity = value.min(100);
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
    fn signature_is_distinctive_default() {
        let theme = UiTheme::default();
        assert_eq!(theme.preset, ThemePreset::Signature);
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.accent, Rgb::new(68, 214, 255));
        assert_eq!(theme.background, Some(Rgb::new(10, 15, 28)));
        assert_eq!(theme.width, 900);
        assert_eq!(theme.height, 640);
    }

    #[test]
    fn parses_theme_palette_density_background_and_bounds() {
        let mut theme = UiTheme::default();
        apply_config(
            &mut theme,
            "preset=midnight\ntheme=dark\nbackdrop=mica\naccent=#112233\nopacity=20\nwidth=99\nheight=9999\ndensity=spacious\nbackground_image=C:\\\\wall.png\nbackground_fit=fit\nbackground_image_opacity=140\n",
        );
        assert_eq!(theme.preset, ThemePreset::Midnight);
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.backdrop, Backdrop::Mica);
        assert_eq!(theme.accent, Rgb::new(0x11, 0x22, 0x33));
        assert_eq!(theme.opacity_percent, 55);
        assert_eq!(theme.width, 620);
        assert_eq!(theme.height, 1300);
        assert_eq!(theme.density, Density::Spacious);
        assert_eq!(theme.background_fit, BackgroundFit::Fit);
        assert_eq!(theme.background_image_opacity, 100);
        assert!(theme.background_image.is_some());
    }

    #[test]
    fn density_controls_result_card_height() {
        assert_eq!(Density::Compact.row_height(), 48);
        assert_eq!(Density::Comfortable.row_height(), 62);
        assert_eq!(Density::Spacious.row_height(), 76);
    }

    #[test]
    fn preset_switch_changes_identity_palette() {
        let mut theme = UiTheme::default();
        theme.apply_preset(ThemePreset::Frost);
        assert_eq!(theme.mode, ThemeMode::Light);
        assert_eq!(theme.backdrop, Backdrop::Mica);
        assert_eq!(theme.accent, Rgb::new(42, 115, 230));
        theme.apply_preset(ThemePreset::Native);
        assert_eq!(theme.mode, ThemeMode::System);
        assert_eq!(theme.background, None);
        assert_eq!(theme.surface, None);
    }

    #[test]
    fn colorref_uses_win32_byte_order() {
        assert_eq!(Rgb::new(0x11, 0x22, 0x33).colorref(), 0x0033_2211);
    }

    #[test]
    fn serialized_theme_keeps_distinctive_settings() {
        let theme = UiTheme {
            preset: ThemePreset::Graphite,
            mode: ThemeMode::Dark,
            backdrop: Backdrop::Mica,
            accent: Rgb::new(0x11, 0x22, 0x33),
            opacity_percent: 75,
            width: 960,
            height: 680,
            density: Density::Compact,
            background_image: Some(PathBuf::from(r"C:\wall.png")),
            background_fit: BackgroundFit::Stretch,
            background_image_opacity: 25,
            ..UiTheme::default()
        };
        let text = theme.to_config_text();
        assert!(text.contains("preset=graphite"));
        assert!(text.contains("theme=dark"));
        assert!(text.contains("backdrop=mica"));
        assert!(text.contains("accent=#112233"));
        assert!(text.contains("opacity=75"));
        assert!(text.contains("density=compact"));
        assert!(text.contains("background_image=C:\\wall.png"));
        assert!(text.contains("background_fit=stretch"));
    }
}
