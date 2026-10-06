use ratatui::style::{Color, Style};
use std::time::Duration;

const EIGHTHS: &[char] = &[' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

pub fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    if mins >= 60 {
        let hours = mins / 60;
        let rem_mins = mins % 60;
        format!("{}h {:02}m", hours, rem_mins)
    } else {
        format!("{}m {:02}s", mins, secs)
    }
}

pub fn render_duration_bar(
    duration: Duration,
    max_duration: Duration,
    width: usize,
) -> (String, usize) {
    if width == 0 || max_duration.as_millis() == 0 {
        return (String::new(), 0);
    }

    let dur_ms = duration.as_millis() as f64;
    let max_ms = max_duration.as_millis() as f64;
    let ratio = (dur_ms / max_ms).clamp(0.0, 1.0);

    let total_eighths = (ratio * (width as f64) * 8.0).round() as usize;
    let full_cells = (total_eighths / 8).min(width);
    let rem_eighth = total_eighths % 8;

    let mut bar = String::with_capacity(width);
    for _ in 0..full_cells {
        bar.push('█');
    }

    if full_cells < width {
        if rem_eighth > 0 {
            bar.push(EIGHTHS[rem_eighth]);
        } else if full_cells == 0 {
            // ensure minimum visible bar is one eighth block
            bar.push('▏');
        }
    }

    let rendered_len = bar.chars().count();
    (bar, rendered_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(14)), "0m 14s");
        assert_eq!(format_duration(Duration::from_secs(252)), "4m 12s");
        assert_eq!(format_duration(Duration::from_secs(3660)), "1h 01m");
    }

    #[test]
    fn test_render_duration_bar() {
        let (bar, len) = render_duration_bar(Duration::from_secs(50), Duration::from_secs(100), 10);
        assert_eq!(len, 5);
        assert_eq!(bar, "█████");
    }
}
