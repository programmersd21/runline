use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn draw_help(f: &mut Frame, theme: &Theme) {
    let area = f.area();

    let width = 64.min(area.width.saturating_sub(4));
    let height = 22.min(area.height.saturating_sub(2));

    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let help_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, help_area);

    let block = Block::default()
        .title(Span::styled(
            " \u{f11c} keyboard shortcuts ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focus));

    let lines = vec![
        Line::from(Span::styled(
            "global",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?            toggle help overlay"),
        Line::from("  shift+t      open interactive theme picker"),
        Line::from("  q            back, or quit from dashboard"),
        Line::from("  ctrl+c       quit immediately"),
        Line::from("  ctrl+r       force refresh runs"),
        Line::from(""),
        Line::from(Span::styled(
            "dashboard",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / k        move selection up/down"),
        Line::from("  enter        open run details & steps"),
        Line::from("  /            filter runs by keyword"),
        Line::from("  b            toggle local branch scope"),
        Line::from("  tab          switch repository tab"),
        Line::from("  r            rerun failed jobs"),
        Line::from("  c            cancel active run"),
        Line::from(""),
        Line::from(Span::styled(
            "log viewer",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  /            search in log"),
        Line::from("  n            next search match (shift+n previous)"),
        Line::from("  e            next error (shift+e previous)"),
        Line::from("  t            toggle timestamps"),
        Line::from("  w            toggle line wrapping"),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().fg(theme.fg)),
        help_area,
    );
}
