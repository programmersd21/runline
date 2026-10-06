use super::Theme;
use ratatui::style::Color;

/// transparent theme — inherits the terminal's own palette.
/// works with catppuccin, gruvbox, nord, dracula, tokyo-night, etc.
/// backgrounds are Reset so terminal transparency/blur shows through.
pub fn glass() -> Theme {
    Theme {
        bg: Color::Reset,
        surface: Color::Reset,
        surface_raised: Color::Indexed(236),
        border: Color::Rgb(60, 65, 80),
        border_focus: Color::Rgb(137, 180, 250), // Catppuccin Mocha Blue
        fg: Color::Reset,
        fg_muted: Color::Rgb(166, 173, 200),   // Subtext
        fg_faint: Color::Rgb(108, 112, 134),   // Overlay
        accent: Color::Rgb(203, 166, 247),     // Mauve / Lavender
        accent_dim: Color::Rgb(116, 199, 236), // Sapphire
        success: Color::Rgb(166, 227, 161),    // Green
        failure: Color::Rgb(243, 139, 168),    // Red
        running: Color::Rgb(249, 226, 175),    // Yellow
        warning: Color::Rgb(250, 179, 135),    // Peach
        neutral: Color::Rgb(147, 153, 178),
        selection_bg: Color::Rgb(49, 50, 68),
        error_bg: Color::Rgb(69, 40, 48),
        match_bg: Color::Rgb(64, 60, 40),
    }
}

/// soft dark built-in with muted tones.
pub fn dusk() -> Theme {
    Theme {
        bg: Color::Rgb(22, 22, 30),
        surface: Color::Rgb(28, 28, 38),
        surface_raised: Color::Rgb(38, 38, 50),
        border: Color::Rgb(55, 55, 70),
        border_focus: Color::Rgb(120, 130, 220),
        fg: Color::Rgb(205, 205, 215),
        fg_muted: Color::Rgb(130, 130, 145),
        fg_faint: Color::Rgb(80, 80, 95),
        accent: Color::Rgb(120, 130, 220),
        accent_dim: Color::Rgb(60, 65, 110),
        success: Color::Rgb(100, 200, 130),
        failure: Color::Rgb(230, 90, 90),
        running: Color::Rgb(230, 180, 80),
        warning: Color::Rgb(230, 170, 60),
        neutral: Color::Rgb(120, 120, 130),
        selection_bg: Color::Rgb(38, 42, 68),
        error_bg: Color::Rgb(55, 25, 28),
        match_bg: Color::Rgb(70, 60, 25),
    }
}

pub fn sakura() -> Theme {
    Theme {
        bg: Color::Reset,
        surface: Color::Reset,
        surface_raised: Color::Indexed(236),
        border: Color::Rgb(244, 180, 200),       // Soft sakura pink
        border_focus: Color::Rgb(255, 126, 179), // Bright cherry blossom
        fg: Color::Reset,
        fg_muted: Color::Rgb(240, 200, 215), // Pastel pinkish white
        fg_faint: Color::Rgb(165, 130, 150),
        accent: Color::Rgb(255, 158, 204),     // Cute pink accent
        accent_dim: Color::Rgb(200, 160, 225), // Pastel lavender
        success: Color::Rgb(140, 230, 170),    // Mint green
        failure: Color::Rgb(255, 110, 140),    // Strawberry red
        running: Color::Rgb(255, 215, 130),    // Peach buttercup
        warning: Color::Rgb(255, 180, 120),    // Apricot
        neutral: Color::Rgb(180, 150, 170),
        selection_bg: Color::Rgb(70, 45, 60), // Rosy dark selection
        error_bg: Color::Rgb(80, 35, 50),
        match_bg: Color::Rgb(80, 70, 40),
    }
}

pub fn synthwave() -> Theme {
    Theme {
        bg: Color::Reset,
        surface: Color::Reset,
        surface_raised: Color::Indexed(235),
        border: Color::Rgb(100, 50, 140),
        border_focus: Color::Rgb(255, 42, 109), // Neon hot pink
        fg: Color::Reset,
        fg_muted: Color::Rgb(230, 210, 255),
        fg_faint: Color::Rgb(120, 95, 160),
        accent: Color::Rgb(5, 217, 232),      // Neon cyan
        accent_dim: Color::Rgb(180, 80, 255), // Electric violet
        success: Color::Rgb(0, 245, 160),     // Neon lime
        failure: Color::Rgb(255, 42, 109),    // Hot pink/red
        running: Color::Rgb(254, 222, 93),    // Cyber yellow
        warning: Color::Rgb(255, 140, 40),
        neutral: Color::Rgb(150, 130, 190),
        selection_bg: Color::Rgb(55, 30, 85),
        error_bg: Color::Rgb(80, 25, 55),
        match_bg: Color::Rgb(75, 65, 30),
    }
}

pub fn tokyo_night() -> Theme {
    Theme {
        bg: Color::Reset,
        surface: Color::Reset,
        surface_raised: Color::Indexed(235),
        border: Color::Rgb(59, 66, 97),
        border_focus: Color::Rgb(122, 162, 247), // Tokyo Night Blue
        fg: Color::Reset,
        fg_muted: Color::Rgb(169, 177, 214),
        fg_faint: Color::Rgb(86, 95, 137),
        accent: Color::Rgb(187, 154, 247),     // Purple
        accent_dim: Color::Rgb(125, 207, 255), // Cyan
        success: Color::Rgb(158, 206, 106),    // Green
        failure: Color::Rgb(247, 118, 142),    // Red
        running: Color::Rgb(224, 175, 104),    // Yellow
        warning: Color::Rgb(255, 158, 100),    // Orange
        neutral: Color::Rgb(154, 165, 206),
        selection_bg: Color::Rgb(40, 52, 79),
        error_bg: Color::Rgb(70, 35, 45),
        match_bg: Color::Rgb(65, 55, 30),
    }
}

pub fn nord() -> Theme {
    Theme {
        bg: Color::Reset,
        surface: Color::Reset,
        surface_raised: Color::Indexed(236),
        border: Color::Rgb(76, 86, 106),
        border_focus: Color::Rgb(136, 192, 208), // Frost Frost
        fg: Color::Reset,
        fg_muted: Color::Rgb(216, 222, 233),
        fg_faint: Color::Rgb(94, 129, 172),
        accent: Color::Rgb(129, 161, 193),
        accent_dim: Color::Rgb(143, 188, 187),
        success: Color::Rgb(163, 190, 140),
        failure: Color::Rgb(191, 97, 106),
        running: Color::Rgb(235, 203, 139),
        warning: Color::Rgb(208, 135, 112),
        neutral: Color::Rgb(143, 150, 165),
        selection_bg: Color::Rgb(59, 66, 82),
        error_bg: Color::Rgb(75, 35, 40),
        match_bg: Color::Rgb(65, 60, 40),
    }
}

pub fn gruvbox() -> Theme {
    Theme {
        bg: Color::Reset,
        surface: Color::Reset,
        surface_raised: Color::Indexed(237),
        border: Color::Rgb(102, 92, 84),
        border_focus: Color::Rgb(250, 189, 47), // Gruvbox Yellow
        fg: Color::Reset,
        fg_muted: Color::Rgb(235, 219, 178),
        fg_faint: Color::Rgb(146, 131, 116),
        accent: Color::Rgb(254, 128, 25),     // Orange
        accent_dim: Color::Rgb(184, 187, 38), // Green
        success: Color::Rgb(184, 187, 38),
        failure: Color::Rgb(251, 73, 52),
        running: Color::Rgb(250, 189, 47),
        warning: Color::Rgb(254, 128, 25),
        neutral: Color::Rgb(168, 153, 132),
        selection_bg: Color::Rgb(60, 56, 54),
        error_bg: Color::Rgb(70, 30, 25),
        match_bg: Color::Rgb(65, 55, 25),
    }
}

pub fn dracula() -> Theme {
    Theme {
        bg: Color::Reset,
        surface: Color::Reset,
        surface_raised: Color::Indexed(236),
        border: Color::Rgb(98, 114, 164),
        border_focus: Color::Rgb(189, 147, 249), // Purple
        fg: Color::Reset,
        fg_muted: Color::Rgb(248, 248, 242),
        fg_faint: Color::Rgb(98, 114, 164),
        accent: Color::Rgb(255, 121, 198),     // Pink
        accent_dim: Color::Rgb(139, 233, 253), // Cyan
        success: Color::Rgb(80, 250, 123),     // Green
        failure: Color::Rgb(255, 85, 85),      // Red
        running: Color::Rgb(241, 250, 140),    // Yellow
        warning: Color::Rgb(255, 184, 108),    // Orange
        neutral: Color::Rgb(140, 150, 180),
        selection_bg: Color::Rgb(68, 71, 90),
        error_bg: Color::Rgb(75, 30, 45),
        match_bg: Color::Rgb(65, 60, 35),
    }
}
