use crate::copy::*;
use crate::gh::{Conclusion, Health, RunStatus};
use crate::theme::Theme;
use ratatui::style::Style;

pub fn status_glyph_label<'a>(
    health: Health,
    spinner_frame: &'a str,
    ascii: bool,
    theme: &Theme,
) -> (&'a str, &'static str, Style) {
    if ascii {
        match health {
            Health::Passed => ("+", LABEL_PASSED, Style::default().fg(theme.success)),
            Health::Failed => ("x", LABEL_FAILED, Style::default().fg(theme.failure)),
            Health::Running => ("*", LABEL_RUNNING, Style::default().fg(theme.running)),
            Health::Queued => (".", LABEL_QUEUED, Style::default().fg(theme.fg_muted)),
            Health::Cancelled => ("-", LABEL_CANCELLED, Style::default().fg(theme.neutral)),
            Health::Skipped => ("-", LABEL_SKIPPED, Style::default().fg(theme.fg_faint)),
            Health::Unknown => (
                "\u{f059}",
                LABEL_UNKNOWN,
                Style::default().fg(theme.neutral),
            ),
        }
    } else {
        match health {
            Health::Passed => ("\u{f00c}", LABEL_PASSED, Style::default().fg(theme.success)),
            Health::Failed => ("\u{f00d}", LABEL_FAILED, Style::default().fg(theme.failure)),
            Health::Running => (
                spinner_frame,
                LABEL_RUNNING,
                Style::default().fg(theme.running),
            ),
            Health::Queued => (
                "\u{f10c}",
                LABEL_QUEUED,
                Style::default().fg(theme.fg_muted),
            ),
            Health::Cancelled => (
                "\u{f05e}",
                LABEL_CANCELLED,
                Style::default().fg(theme.neutral),
            ),
            Health::Skipped => (
                "\u{f068}",
                LABEL_SKIPPED,
                Style::default().fg(theme.fg_faint),
            ),
            Health::Unknown => (
                "\u{f059}",
                LABEL_UNKNOWN,
                Style::default().fg(theme.neutral),
            ),
        }
    }
}
