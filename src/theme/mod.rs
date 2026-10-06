pub mod builtin;
pub mod color;
pub mod derive;
use ratatui::style::Color;
#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub border: Color,
    pub border_focus: Color,
    pub fg: Color,
    pub fg_muted: Color,
    pub fg_faint: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub success: Color,
    pub failure: Color,
    pub running: Color,
    pub warning: Color,
    pub neutral: Color,
    pub selection_bg: Color,
    pub error_bg: Color,
    pub match_bg: Color,
}
impl Default for Theme {
    fn default() -> Self {
        builtin::dusk()
    }
}
pub const THEMES: &[&str] = &[
    "glass",
    "sakura",
    "tokyo-night",
    "synthwave",
    "nord",
    "gruvbox",
    "dracula",
    "dusk",
];

pub fn by_name(name: &str) -> Theme {
    match name.to_lowercase().as_str() {
        "glass" | "terminal" => builtin::glass(),
        "sakura" | "cherry" | "cute" | "paper" => builtin::sakura(),
        "synthwave" | "cyberpunk" => builtin::synthwave(),
        "tokyo-night" | "tokyonight" => builtin::tokyo_night(),
        "nord" => builtin::nord(),
        "gruvbox" => builtin::gruvbox(),
        "dracula" => builtin::dracula(),
        "dusk" => builtin::dusk(),
        _ => builtin::glass(),
    }
}
fn rgb_of(c: Color) -> Option<(u8, u8, u8)> {
    match c {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::White => Some((255, 255, 255)),
        Color::Black => Some((0, 0, 0)),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::builtin;
    use super::color::contrast_ratio;
    #[test]
    fn builtin_contrast_aa() {
        for t in [builtin::dusk()] {
            let bg = super::rgb_of(t.bg).unwrap();
            for fg in [t.fg, t.fg_muted, t.accent, t.success, t.failure] {
                if let Some(f) = super::rgb_of(fg) {
                    assert!(contrast_ratio(f, bg) >= 2.0, "low contrast {:?}", f);
                }
            }
        }
    }
}
