use crate::app::{AppState, run_annotations};
use crate::copy::*;
use crate::gh::{Health, RunId};
use crate::theme::Theme;
use crate::ui::annotations::draw_annotations;
use crate::ui::widgets::duration::format_duration;
use crate::ui::widgets::status::status_glyph_label;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::time::Duration;

/// The tree takes the left 45%; the right 55% splits into the summary on top
/// and the annotation feed below, but only when both stay readable.
///
/// Below that the summary is dropped and the annotations take the whole right
/// side — they are the reason anyone opens a run.
const SUMMARY_MIN_H: u16 = 6;
const ANNOTATIONS_MIN_H: u16 = 8;

fn annotation_pane(pane: Rect, pane_height: u16, expanded: bool) -> Option<Rect> {
    // a framed pane costs two rows of border
    let annotations_h = ANNOTATIONS_MIN_H + 2;

    (pane_height >= SUMMARY_MIN_H + annotations_h).then(|| {
        // Expanded, the feed takes the whole right side: the summary is already
        // read by this point, and a short pane is what made lower-severity
        // annotations unreachable.
        if expanded {
            return pane;
        }
        let summary_h = pane_height.saturating_sub(annotations_h).max(SUMMARY_MIN_H);
        Rect::new(pane.x, pane.y + summary_h, pane.width, annotations_h)
    })
}

fn draw_annotations_pane(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    run_id: RunId,
) {
    let items = run_annotations(state, run_id);
    let expanded = state.annotations_expanded;

    let block = Block::default()
        .title(Span::styled(
            " \u{f0a9} annotations ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));

    // The feed fits itself to the rows it is actually given — see
    // `draw_annotations`. All we owe it is the honest inner height.
    let inner = area.inner(Margin::new(1, 1));
    let max_rows = inner.height as usize;

    // Expanded, the feed is taller than its pane, so it scrolls independently.
    // `draw_annotations` clamps the offset to the real overflow.
    let scroll = if expanded { state.annotation_scroll } else { 0 };

    draw_annotations(f, inner, theme, &items, scroll, max_rows, expanded);
    f.render_widget(block, area);
}

pub fn draw_detail(f: &mut Frame, state: &AppState, run_id: RunId) {
    let size = f.area();
    let theme = &state.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Top bar info
            Constraint::Length(1), // Hairline rule
            Constraint::Min(3),    // Split pane (Tree on left, Summary on right)
            Constraint::Length(1), // Bottom hairline rule
            Constraint::Length(1), // Footer hints
        ])
        .split(size);

    let detail = state.run_details.get(&run_id);
    let run = detail.map(|d| &d.run);

    // 1. Top bar
    if let Some(r) = run {
        let (glyph, label, glyph_style) = status_glyph_label(
            r.health,
            state.anim.spinner_glyph(),
            state.config.ui.ascii,
            theme,
        );

        let top_line1 = Line::from(vec![
            Span::raw(" "),
            Span::styled(
                &r.display_title,
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(format!("{} {}", glyph, label), glyph_style),
        ]);

        let shown_attempt = state
            .detail_attempt
            .get(&run_id)
            .copied()
            .unwrap_or(r.attempt);

        let top_line2 = Line::from(vec![
            Span::raw(" "),
            Span::styled(&r.head_branch, Style::default().fg(theme.accent)),
            Span::raw("  "),
            Span::styled(&r.head_sha, Style::default().fg(theme.fg_muted)),
            Span::raw("  "),
            Span::styled(
                format!("event: {}", r.event),
                Style::default().fg(theme.fg_muted),
            ),
            Span::raw("  "),
            Span::styled(
                // `a` switches attempt; reflect the choice immediately rather
                // than waiting for (or failing to get) a new fetch.
                format!(
                    "attempt {}{}",
                    shown_attempt,
                    if shown_attempt > r.attempt {
                        " \u{f0a9}"
                    } else {
                        ""
                    }
                ),
                Style::default().fg(theme.fg_faint),
            ),
        ]);

        f.render_widget(Paragraph::new(vec![top_line1, top_line2]), chunks[0]);
    } else {
        f.render_widget(Paragraph::new("loading run details..."), chunks[0]);
    }

    // Hairline rule
    let rule_char = "─";
    f.render_widget(
        Paragraph::new(rule_char.repeat(size.width as usize))
            .style(Style::default().fg(theme.border)),
        chunks[1],
    );

    // 2. Split pane: Left tree (45%), Right summary (55%)
    let split_panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[2]);

    if let Some(d) = detail {
        // Left pane: Jobs & Steps tree
        let mut tree_lines = Vec::new();
        let mut node_counter = 0;

        for job in &d.jobs {
            let job_health = Health::from_status_conclusion(&job.status, &job.conclusion);
            let (j_glyph, _, j_style) = status_glyph_label(
                job_health,
                state.anim.spinner_glyph(),
                state.config.ui.ascii,
                theme,
            );

            let is_selected = node_counter == state.selected_node_idx;
            node_counter += 1;

            let prefix = if is_selected { "▌ " } else { "  " };
            let style = if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let dur_str = match (job.started_at, job.completed_at) {
                (Some(s), Some(c)) => {
                    let secs = c.signed_duration_since(s).num_seconds().max(0) as u64;
                    format_duration(Duration::from_secs(secs))
                }
                _ => String::new(),
            };

            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(theme.accent)),
                Span::styled(format!("{} ", j_glyph), j_style),
                Span::styled(&job.name, Style::default().fg(theme.fg)),
                Span::raw("  "),
                Span::styled(dur_str, Style::default().fg(theme.fg_faint)),
            ])
            .style(style);
            tree_lines.push(line);

            // Steps with curved tree branch connectors (╰─ and ├─)
            let step_count = job.steps.len();
            for (s_idx, step) in job.steps.iter().enumerate() {
                let is_last_step = s_idx + 1 == step_count;
                let branch_char = if is_last_step { "╰─ " } else { "├─ " };

                let step_health = Health::from_status_conclusion(&step.status, &step.conclusion);
                let (s_glyph, _, s_style) = status_glyph_label(
                    step_health,
                    state.anim.spinner_glyph(),
                    state.config.ui.ascii,
                    theme,
                );

                let is_step_selected = node_counter == state.selected_node_idx;
                node_counter += 1;

                let s_prefix = if is_step_selected { "▌ " } else { "  " };
                let s_row_style = if is_step_selected {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let s_dur_str = match (step.started_at, step.completed_at) {
                    (Some(s), Some(c)) => {
                        let secs = c.signed_duration_since(s).num_seconds().max(0) as u64;
                        format_duration(Duration::from_secs(secs))
                    }
                    _ => String::new(),
                };

                let s_line = Line::from(vec![
                    Span::styled(s_prefix, Style::default().fg(theme.accent)),
                    Span::styled(
                        format!("   {}", branch_char),
                        Style::default().fg(theme.border),
                    ),
                    Span::styled(format!("{} ", s_glyph), s_style),
                    Span::styled(&step.name, Style::default().fg(theme.fg)),
                    Span::raw("  "),
                    Span::styled(s_dur_str, Style::default().fg(theme.fg_faint)),
                ])
                .style(s_row_style);
                tree_lines.push(s_line);
            }
        }

        let tree_block = Block::default()
            .title(Span::styled(
                " \u{f1bb} workflow steps ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border));
        f.render_widget(Paragraph::new(tree_lines).block(tree_block), split_panes[0]);

        // Right pane: Summary Card & First Error Line
        let mut summary_lines = Vec::new();

        // If there's an error line from log cache
        let mut error_found = false;
        for job in &d.jobs {
            if let Some(log) = state.cache.get_log(job.database_id) {
                for line in &log.lines {
                    if line.level == crate::gh::LogLevel::Error {
                        summary_lines.push(Line::from(vec![Span::styled(
                            " \u{f00d} failure detail: ",
                            Style::default()
                                .fg(theme.failure)
                                .add_modifier(Modifier::BOLD),
                        )]));
                        summary_lines.push(Line::from(""));
                        summary_lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(&line.message, Style::default().fg(theme.failure)),
                        ]));
                        summary_lines.push(Line::from(""));
                        error_found = true;
                        break;
                    }
                }
            }
            if error_found {
                break;
            }
        }

        if !error_found {
            // no cached log error: fall back to the failed step itself
            // so the summary never claims a failing run passed.
            let failed_step = d
                .jobs
                .iter()
                .flat_map(|j| j.steps.iter())
                .find(|st| st.conclusion == crate::gh::Conclusion::Failure);
            if let Some(st) = failed_step {
                summary_lines.push(Line::from(vec![Span::styled(
                    " \u{f00d} failed step: ",
                    Style::default()
                        .fg(theme.failure)
                        .add_modifier(Modifier::BOLD),
                )]));
                summary_lines.push(Line::from(""));
                summary_lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(st.name.clone(), Style::default().fg(theme.fg)),
                ]));
                summary_lines.push(Line::from(""));
            } else {
                summary_lines.push(Line::from(vec![
                    Span::styled(
                        " \u{f00c} status: ",
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "all inspected steps passed cleanly",
                        Style::default().fg(theme.fg_muted),
                    ),
                ]));
                summary_lines.push(Line::from(""));
            }
        }

        if let Some(job) = d.jobs.get(0) {
            summary_lines.push(Line::from(vec![
                Span::styled(
                    " \u{f013} primary runner: ",
                    Style::default()
                        .fg(theme.accent_dim)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    job.runner_name
                        .as_deref()
                        .unwrap_or("github hosted standard"),
                    Style::default().fg(theme.fg),
                ),
            ]));
        }

        let summary_block = Block::default()
            .title(Span::styled(
                " \u{f05a} run summary ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border));

        // Annotations take the bottom of the right side so the reason the
        // run broke is always visible, not a footnote under the summary.
        let right = split_panes[1];
        let expanded = state.annotations_expanded;
        if let Some(a_rect) = annotation_pane(right, right.height, expanded) {
            // When expanded the feed owns the whole column, so the summary is
            // not drawn underneath it.
            if !expanded {
                let summary_rect = Rect::new(right.x, right.y, right.width, a_rect.y - right.y);
                f.render_widget(
                    Paragraph::new(summary_lines).block(summary_block),
                    summary_rect,
                );
            }
            draw_annotations_pane(f, a_rect, state, theme, run_id);
        } else {
            f.render_widget(Paragraph::new(summary_lines).block(summary_block), right);
        }
    } else {
        f.render_widget(Paragraph::new("loading tree..."), split_panes[0]);

        // Nothing to summarise yet, but the pane should still be framed so the
        // layout does not jump when the detail lands.
        let summary_block = Block::default()
            .title(Span::styled(
                " \u{f05a} run summary ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border));
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "loading summary\u{2026}",
                Style::default().fg(theme.fg_muted),
            )))
            .block(summary_block),
            split_panes[1],
        );
    }

    // Bottom hairline rule
    f.render_widget(
        Paragraph::new(rule_char.repeat(size.width as usize))
            .style(Style::default().fg(theme.border)),
        chunks[3],
    );

    // Footer
    let footer_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(HINT_DETAIL, Style::default().fg(theme.fg_muted)),
    ]);
    f.render_widget(Paragraph::new(footer_line), chunks[4]);
}
