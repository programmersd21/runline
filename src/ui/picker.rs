use crate::theme::{THEMES, Theme};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn draw_theme_picker(f: &mut Frame, selected_idx: usize, theme: &Theme) {
    let area = f.area();

    let width = 44.min(area.width.saturating_sub(4));
    let height = (THEMES.len() as u16 + 4).min(area.height.saturating_sub(2));

    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let picker_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, picker_area);

    let block = Block::default()
        .title(Span::styled(
            " \u{f1fc} select theme ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focus));

    let mut lines = Vec::new();
    lines.push(Line::from(""));

    for (idx, &t_name) in THEMES.iter().enumerate() {
        let is_selected = idx == selected_idx;
        let prefix = if is_selected { " ▌ " } else { "   " };
        let style = if is_selected {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg)
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(theme.accent)),
            Span::styled(t_name, style),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().fg(theme.fg)),
        picker_area,
    );
}
