pub mod annotations;
pub mod dashboard;
pub mod detail;
pub mod dialog;
pub mod help;
pub mod log;
pub mod picker;
pub mod sanitize;
pub mod widgets;

use crate::app::{AppState, Screen};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn draw(f: &mut Frame, state: &AppState) {
    match state.current_screen() {
        Screen::Dashboard => {
            dashboard::draw_dashboard(f, state);
        }
        Screen::Detail(run_id) => {
            detail::draw_detail(f, state, run_id);
        }
        Screen::Log(run_id, job_id) => {
            log::draw_log(f, state, run_id, job_id);
        }
    }

    if let Some(ref d) = state.dialog {
        dialog::draw_dialog(f, d, &state.theme);
    }

    if state.show_help {
        help::draw_help(f, &state.theme);
    }

    if state.show_theme_picker {
        picker::draw_theme_picker(f, state.theme_picker_idx, &state.theme);
    }

    // braille loader overlay — rendered last so it sits on top of everything
    if state.is_loading {
        draw_loading_overlay(f, state);
    }

    // scrub control characters out of the finished frame, after every screen
    // and overlay has drawn — see `sanitize` for why this has to happen here.
    sanitize::scrub(f);
}

/// A sleek bottom-right corner loading indicator.
/// Shows a spinning braille dot + "reloading…" so it's visible but not jarring.
fn draw_loading_overlay(f: &mut Frame, state: &AppState) {
    let theme = &state.theme;
    let area = f.area();

    // build the label: "⠹ reloading…"
    let spinner = state.anim.braille();
    let label = format!(" {} reloading\u{2026} ", spinner);
    let label_width = label.chars().count() as u16;

    // position: bottom-right, 1 row above the footer (row height-2), flush right
    let x = area.width.saturating_sub(label_width);
    let y = area.height.saturating_sub(2); // sits just above the footer hairline

    let badge_area = Rect {
        x,
        y,
        width: label_width,
        height: 1,
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", spinner),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("reloading\u{2026} ", Style::default().fg(theme.fg_muted)),
    ]);

    f.render_widget(Paragraph::new(line), badge_area);
}
