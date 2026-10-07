use crate::app::AppState;
use crate::copy::*;
use crate::theme::Theme;
use crate::ui::widgets::duration::{format_duration, render_duration_bar};
use crate::ui::widgets::status::status_glyph_label;
use chrono::Utc;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::time::Duration;

pub fn draw_dashboard(f: &mut Frame, state: &AppState) {
    let size = f.area();
    let theme = &state.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top header
            Constraint::Length(1), // Hairline rule
            Constraint::Min(3),    // Main table or empty state
            Constraint::Length(1), // Filter bar (if active)
            Constraint::Length(1), // Bottom hairline rule
            Constraint::Length(1), // Footer shortcuts
        ])
        .split(size);

    // 1. Header
    let repo_title = match state.current_repo() {
        Some(r) => r.name_with_owner(),
        None => "no repository".to_string(),
    };

    let quota_str = match state.rate_limit {
        Some(ref q) => format_quota(q.used, q.limit),
        None => String::new(),
    };

    let live_str = if state.error_banner.is_some() {
        Span::styled(OFFLINE_INDICATOR, Style::default().fg(theme.fg_muted))
    } else {
        Span::styled(LIVE_INDICATOR, Style::default().fg(theme.accent))
    };

    let header_line = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            APP_TITLE,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            repo_title,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(size.width.saturating_sub(55) as usize)),
        live_str,
        Span::raw("   "),
        Span::styled(quota_str, Style::default().fg(theme.fg_faint)),
    ]);
    f.render_widget(Paragraph::new(header_line), chunks[0]);

    // 2. Hairline rule
    let rule_char = "─";
    let rule = Paragraph::new(rule_char.repeat(size.width as usize))
        .style(Style::default().fg(theme.border));
    f.render_widget(rule, chunks[1]);

    // 3. Main content
    let runs = state.filtered_runs();
    if runs.is_empty() {
        let empty_msg = if !state.filter_text.is_empty() {
            ERR_NO_MATCHES
        } else if let Some(ref err) = state.error_banner {
            err.as_str()
        } else {
            "no runs found. push a commit or check your filters."
        };
        let empty_para = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(empty_msg, Style::default().fg(theme.fg_muted)),
            ]),
        ]);
        f.render_widget(empty_para, chunks[2]);
    } else {
        let max_duration = runs
            .iter()
            .map(|r| {
                let end = r.updated_at;
                let start = r.started_at.unwrap_or(r.created_at);
                let diff = end.signed_duration_since(start).num_seconds().max(0) as u64;
                Duration::from_secs(diff)
            })
            .max()
            .unwrap_or_else(|| Duration::from_secs(1));

        let mut lines = Vec::new();

        // Table Header
        let col_header = Line::from(vec![
            Span::raw("   "),
            Span::styled(
                format!("{:<12}", COL_STATUS),
                Style::default()
                    .fg(theme.fg_faint)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<16}", COL_WORKFLOW),
                Style::default()
                    .fg(theme.fg_faint)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<20}", COL_BRANCH),
                Style::default()
                    .fg(theme.fg_faint)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<12}", COL_ACTOR),
                Style::default()
                    .fg(theme.fg_faint)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<10}", COL_AGE),
                Style::default()
                    .fg(theme.fg_faint)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                COL_DURATION,
                Style::default()
                    .fg(theme.fg_faint)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        lines.push(col_header);

        let spinner = state.anim.spinner_glyph();
        let now = Utc::now();

        for (idx, r) in runs.iter().enumerate() {
            let is_selected = idx == state.selected_run_idx;
            let (glyph, label, glyph_style) =
                status_glyph_label(r.health, spinner, state.config.ui.ascii, theme);

            let status_cell = format!("{} {}", glyph, label);

            // calculate age
            let age_secs = now.signed_duration_since(r.created_at).num_seconds().max(0);
            let age_str = if age_secs < 60 {
                format!("{}s", age_secs)
            } else if age_secs < 3600 {
                format!("{}m", age_secs / 60)
            } else if age_secs < 86400 {
                format!("{}h", age_secs / 3600)
            } else {
                format!("{}d", age_secs / 86400)
            };

            // calculate duration
            let end = r.updated_at;
            let start = r.started_at.unwrap_or(r.created_at);
            let dur_secs = end.signed_duration_since(start).num_seconds().max(0) as u64;
            let run_dur = Duration::from_secs(dur_secs);
            let dur_str = format_duration(run_dur);

            let bar_width = 12;
            let (bar_str, _) = render_duration_bar(run_dur, max_duration, bar_width);

            let prefix = if is_selected {
                Span::styled("▌ ", Style::default().fg(theme.accent))
            } else {
                Span::raw("  ")
            };

            let row_style = if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let actor_display = if r.actor.is_empty() || r.actor == "unknown" {
                if !r.event.is_empty() { &r.event } else { "ci" }
            } else {
                &r.actor
            };

            let row = Line::from(vec![
                prefix,
                Span::styled(format!("{:<13}", status_cell), glyph_style),
                Span::styled(
                    format!("{:<16}", truncate_str(&r.workflow_name, 14)),
                    Style::default().fg(theme.fg),
                ),
                Span::styled(
                    format!("{:<20}", truncate_str(&r.head_branch, 18)),
                    Style::default().fg(theme.fg_muted),
                ),
                Span::styled(
                    format!("{:<12}", truncate_str(actor_display, 10)),
                    Style::default().fg(theme.fg_faint),
                ),
                Span::styled(
                    format!("{:<10}", age_str),
                    Style::default().fg(theme.fg_faint),
                ),
                Span::styled(
                    format!("{:<14}", bar_str),
                    if r.health == crate::gh::Health::Failed {
                        Style::default().fg(theme.failure)
                    } else {
                        Style::default().fg(theme.accent_dim)
                    },
                ),
                Span::styled(dur_str, Style::default().fg(theme.fg_muted)),
            ])
            .style(row_style);

            lines.push(row);
        }

        f.render_widget(Paragraph::new(lines), chunks[2]);
    }

    // 4. Filter input (if active)
    if state.filter_active {
        let filter_line = Line::from(vec![
            Span::styled("/ ", Style::default().fg(theme.accent)),
            Span::styled(&state.filter_text, Style::default().fg(theme.fg)),
            Span::styled("█", Style::default().fg(theme.accent)),
        ]);
        f.render_widget(Paragraph::new(filter_line), chunks[3]);
    } else {
        f.render_widget(Paragraph::new(""), chunks[3]);
    }

    // 5. Bottom hairline rule
    let bottom_rule = Paragraph::new(rule_char.repeat(size.width as usize))
        .style(Style::default().fg(theme.border));
    f.render_widget(bottom_rule, chunks[4]);

    // 6. Footer (Linear / Raycast pill shortcuts)
    let footer_line = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            " [j/k] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("move", Style::default().fg(theme.fg_muted)),
        Span::raw("  "),
        Span::styled(
            " [↵] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("inspect", Style::default().fg(theme.fg_muted)),
        Span::raw("  "),
        Span::styled(
            " [/] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("filter", Style::default().fg(theme.fg_muted)),
        Span::raw("  "),
        Span::styled(
            " [r] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("rerun", Style::default().fg(theme.fg_muted)),
        Span::raw("  "),
        Span::styled(
            " [c] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("cancel", Style::default().fg(theme.fg_muted)),
        Span::raw("  "),
        Span::styled(
            " [T] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("theme", Style::default().fg(theme.fg_muted)),
        Span::raw("  "),
        Span::styled(
            " [?] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("help", Style::default().fg(theme.fg_muted)),
        Span::raw("  "),
        Span::styled(
            " [q] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("quit", Style::default().fg(theme.fg_muted)),
        Span::raw("  "),
        Span::styled(
            " [shift+r] ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("reload", Style::default().fg(theme.fg_muted)),
    ]);
    f.render_widget(Paragraph::new(footer_line), chunks[5]);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let mut truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        truncated.push('…');
        truncated
    }
}
