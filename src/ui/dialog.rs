use crate::app::{DialogState, DialogType};
use crate::copy::*;
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn draw_dialog(f: &mut Frame, dialog: &DialogState, theme: &Theme) {
    let area = f.area();

    // Centered modal popup area
    let width = 50.min(area.width.saturating_sub(4));
    let height = 8.min(area.height.saturating_sub(2));

    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, dialog_area);

    let (title, detail) = match &dialog.dialog_type {
        DialogType::RerunFailed(_id, wf, branch, count) => (
            PROMPT_RERUN_FAILED,
            format!("workflow: {}\nbranch: {} ({} jobs)", wf, branch, count),
        ),
        DialogType::CancelRun(_id, wf, branch) => (
            PROMPT_CANCEL_RUN,
            format!("workflow: {}\nbranch: {}", wf, branch),
        ),
    };

    let block = Block::default()
        .title(Span::styled(
            format!(" \u{f0e7} {} ", title),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focus));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(2), Constraint::Length(1)])
        .split(dialog_area);

    f.render_widget(block, dialog_area);

    let body_text = if let Some(ref err) = dialog.error {
        format!("error: {}", err)
    } else if dialog.is_running {
        "running action...".to_string()
    } else {
        detail
    };

    f.render_widget(
        Paragraph::new(body_text).style(Style::default().fg(theme.fg)),
        chunks[0],
    );

    // Buttons: Cancel (default) vs Confirm
    let cancel_style = if !dialog.focus_confirm {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(theme.fg_muted)
    };

    let confirm_style = if dialog.focus_confirm {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(theme.fg_muted)
    };

    let buttons = Line::from(vec![
        Span::raw("        "),
        Span::styled(format!(" [ {} ] ", BTN_CANCEL), cancel_style),
        Span::raw("   "),
        Span::styled(format!(" [ {} ] ", BTN_CONFIRM), confirm_style),
    ]);

    f.render_widget(Paragraph::new(buttons), chunks[1]);
}
