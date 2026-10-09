use crate::theme::{Theme, THEMES};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn draw_theme_picker(f: &mut Frame, selected_idx: usize, current_saved: &str, theme: &Theme) {
    let area = f.area();

    let width = 56.min(area.width.saturating_sub(4));
    let height = (THEMES.len() as u16 + 7).min(area.height.saturating_sub(2));

    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let picker_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, picker_area);

    let block = Block::default()
        .title(Span::styled(
            " \u{f1fc} select color theme ",
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
        let is_current = t_name.eq_ignore_ascii_case(current_saved);

        let preview_theme = crate::theme::by_name(t_name);

        let prefix = if is_selected { " ▸ " } else { "   " };
        let current_tag = if is_current { " (current)" } else { "" };

        let row_style = if is_selected {
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.fg_muted)
        };

        // Theme palette color preview chips
        let color_swatches = vec![
            Span::styled("■ ", Style::default().fg(preview_theme.accent)),
            Span::styled("■ ", Style::default().fg(preview_theme.success)),
            Span::styled("■ ", Style::default().fg(preview_theme.failure)),
            Span::styled("■ ", Style::default().fg(preview_theme.running)),
        ];

        let mut row_spans = vec![
            Span::styled(prefix, Style::default().fg(theme.accent)),
            Span::styled(format!("{:<14}", t_name), row_style),
        ];
        row_spans.extend(color_swatches);
        row_spans.push(Span::styled(
            format!(" {}", current_tag),
            Style::default().fg(theme.accent_dim),
        ));

        lines.push(Line::from(row_spans).style(row_style));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "↑/↓ / j/k: preview  ·  Enter: persist  ·  Esc: cancel",
            Style::default().fg(theme.fg_faint),
        ),
    ]));

    f.render_widget(Paragraph::new(lines).block(block), picker_area);
}
