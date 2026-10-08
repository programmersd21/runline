use crate::app::AppState;
use crate::gh::{AnsiColor, ColorSpan, JobId, LogLevel, RunId};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// clean, god-tier log viewer.
pub fn draw_log(f: &mut Frame, state: &AppState, _run_id: RunId, _job_id: JobId) {
    let size = f.area();
    let theme = &state.theme;

    let lv = match state.log_viewer {
        Some(ref l) => l,
        None => {
            f.render_widget(Paragraph::new("no log viewer active"), size);
            return;
        }
    };

    // One pass over the log for the whole frame: the header counter, the
    // scroller and the renderer must agree on which rows exist, otherwise the
    // cursor lands on lines that were never drawn.
    let rows = crate::app::visible_rows(lv);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Length(1), // top hairline
            Constraint::Min(3),    // log body
            Constraint::Length(1), // bottom hairline
            Constraint::Length(1), // footer
        ])
        .split(size);

    let rule = "─".repeat(size.width as usize);

    draw_header(f, chunks[0], lv, theme, size.width as usize, &rows);
    f.render_widget(
        Paragraph::new(rule.clone()).style(Style::default().fg(theme.border)),
        chunks[1],
    );
    draw_body(f, chunks[2], lv, theme, &rows);
    f.render_widget(
        Paragraph::new(rule).style(Style::default().fg(theme.border)),
        chunks[3],
    );
    draw_footer(f, chunks[4], lv, state.last_export.as_deref(), theme);
}

// ─────────────────────────────────────────────────────────────────────────────
// Header
// ─────────────────────────────────────────────────────────────────────────────

fn draw_header(
    f: &mut Frame,
    area: Rect,
    lv: &crate::app::LogViewerState,
    theme: &Theme,
    width: usize,
    rows: &[crate::app::LogRow],
) {
    let total = rows.len();
    let cur = rows
        .iter()
        .position(|(i, _)| *i == lv.selected_line)
        .map(|p| p + 1)
        .unwrap_or(0);

    let mut spans: Vec<Span> = vec![
        Span::raw(" "),
        Span::styled(
            "\u{f15c} log",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}/{}", cur, total),
            Style::default().fg(theme.fg_faint),
        ),
    ];

    // step scope breadcrumb
    if let Some(ref sf) = lv.step_filter {
        spans.push(Span::styled("  ", Style::default()));
        spans.push(Span::styled("\u{e0b1} ", Style::default().fg(theme.border)));
        spans.push(Span::styled(
            truncate(sf, 30),
            Style::default().fg(theme.fg_muted),
        ));
        // `gh` labels every line `UNKNOWN STEP` whenever GitHub cannot map
        // logs to steps, and then the scoped view silently falls back to the
        // whole job log. Say so instead of quietly showing something else.
        let scoped_away = lv.log.as_ref().is_some_and(|log| {
            !log.lines
                .iter()
                .any(|l| l.step_name.as_deref() == Some(sf.as_str()))
        });
        if scoped_away {
            spans.push(Span::styled("  ", Style::default()));
            spans.push(Span::styled(
                "\u{f071} no step data, showing whole job",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::ITALIC),
            ));
        }
    }

    // search state
    if lv.search_active {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            "\u{f002}/",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!("{}\u{2588}", lv.search_query),
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ));
    } else if !lv.search_query.is_empty() {
        let n = lv.search_matches.len();
        let cm = if n > 0 { lv.current_match + 1 } else { 0 };
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("\u{f002} {}  ", lv.search_query),
            Style::default().fg(theme.fg_muted),
        ));
        spans.push(Span::styled(
            format!("{}/{}", cm, n),
            Style::default().fg(if n > 0 { theme.success } else { theme.failure }),
        ));
    }

    // retry error
    if let Some(ref e) = lv.load_error {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("\u{f071} {}  retrying\u{2026}", truncate(e, 35)),
            Style::default().fg(theme.warning),
        ));
    }

    // right-side flags (timestamps / wrap active)
    let flags: String = [
        if lv.show_timestamps {
            Some("\u{f017}")
        } else {
            None
        }, //
        if lv.wrap_lines {
            Some("\u{f0e9}")
        } else {
            None
        }, //
    ]
    .iter()
    .flatten()
    .cloned()
    .collect::<Vec<_>>()
    .join(" ");

    if !flags.is_empty() {
        // right-align
        let used: usize = spans
            .iter()
            .map(|s| crate::ui::widgets::display_width(&s.content))
            .sum();
        let pad = width.saturating_sub(used + crate::ui::widgets::display_width(&flags) + 1);
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled(flags, Style::default().fg(theme.fg_faint)));
        spans.push(Span::raw(" "));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Body
// ─────────────────────────────────────────────────────────────────────────────

fn draw_body(
    f: &mut Frame,
    area: Rect,
    lv: &crate::app::LogViewerState,
    theme: &Theme,
    rows: &[crate::app::LogRow],
) {
    let width = area.width as usize;

    if let Some(ref raw_log) = lv.log {
        let vh = area.height as usize;

        if rows.is_empty() {
            // Distinguish "this step really produced nothing" from "there is no
            // log to read at all" — the first is an answer, the second is
            // usually a run whose logs have not landed yet.
            let (title, hint) = if lv.step_filter.is_some() {
                (
                    "\u{f05a}  no output from this step",
                    Some("the step ran, but its job log has no lines for it."),
                )
            } else {
                (
                    "\u{f110}  no log output",
                    Some(
                        "the job produced no log lines. it may still be running, \
                         or logs are not available yet \u{2014} they will appear once they are.",
                    ),
                )
            };

            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        title,
                        Style::default()
                            .fg(theme.fg_muted)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ];
            if let Some(hint) = hint {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(hint, Style::default().fg(theme.fg_faint)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("press ", Style::default().fg(theme.fg_muted)),
                    Span::styled(
                        "[R]",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to retry, ", Style::default().fg(theme.fg_muted)),
                    Span::styled(
                        "[q]",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to go back", Style::default().fg(theme.fg_muted)),
                ]));
            }

            f.render_widget(Paragraph::new(lines), area);
            return;
        }

        // Keep the cursor on screen: scroll so it sits roughly centred, but
        // never past the end of the row list.
        let sel_pos = rows
            .iter()
            .position(|(i, _)| *i == lv.selected_line)
            .unwrap_or(0);
        let start = sel_pos
            .saturating_sub(vh / 2)
            .min(rows.len().saturating_sub(vh));

        // prefix width: 1 (cursor) + 5 (linenum) + 1 (space) + 2 (gutter) = 9
        // + optional timestamp 9 chars "HH:MM:SS "
        let ts_cols: usize = if lv.show_timestamps { 9 } else { 0 };
        let prefix_cols = 9 + ts_cols;
        // Never exceed the space actually left in the area: a line wider than
        // the terminal wraps, scrolls the screen and desyncs ratatui's buffer
        // from reality, which shows up as garbage on every later screen.
        let avail_msg = width.saturating_sub(prefix_cols).max(1);

        let mut lines: Vec<Line> = Vec::new();

        for (idx, hidden) in rows.iter().skip(start).take(vh) {
            let line = &raw_log.lines[*idx];
            let is_sel = *idx == lv.selected_line;
            let is_cur_match = lv
                .search_matches
                .get(lv.current_match)
                .map(|&m| m == *idx)
                .unwrap_or(false);
            let is_match = lv.search_matches.contains(idx);

            let (glyph, glyph_style) = level_glyph(line.level, theme);
            let msg_style = level_style(line.level, theme);
            let msg_style = if is_cur_match {
                msg_style.bg(theme.match_bg).add_modifier(Modifier::BOLD)
            } else if is_match {
                msg_style.add_modifier(Modifier::UNDERLINED)
            } else {
                msg_style
            };

            let row_style = if is_sel {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // build spans left-to-right
            let cursor = if is_sel {
                Span::styled("\u{258c}", Style::default().fg(theme.accent))
            } else {
                Span::raw(" ")
            };

            let linenum = Span::styled(
                format!("{:>5} ", idx + 1),
                Style::default().fg(if is_sel {
                    theme.fg_muted
                } else {
                    theme.fg_faint
                }),
            );

            let gutter_span = Span::styled(
                format!("{} ", glyph),
                if is_sel {
                    glyph_style.add_modifier(Modifier::BOLD)
                } else {
                    glyph_style
                },
            );

            let mut spans: Vec<Span> = vec![cursor, linenum, gutter_span];

            if lv.show_timestamps {
                let ts_str = if let Some(ref ts) = line.timestamp {
                    format!("{} ", ts.format("%H:%M:%S"))
                } else {
                    "         ".to_string() // 9 spaces to keep alignment
                };
                spans.push(Span::styled(ts_str, Style::default().fg(theme.fg_faint)));
            }

            // fold indicator instead of message for folded group headers
            if line.is_group_start && lv.folded_groups.contains(idx) {
                spans.push(Span::styled(
                    format!("\u{f0da} {} lines  ", hidden),
                    Style::default()
                        .fg(theme.accent_dim)
                        .add_modifier(Modifier::BOLD),
                ));
                // strip ##[group] from the label
                let label = strip_group_prefix(&line.message);
                spans.push(Span::styled(
                    truncate(label, avail_msg.saturating_sub(16)),
                    Style::default().fg(theme.fg_muted),
                ));
                lines.push(Line::from(spans).style(row_style));
                continue;
            }

            // group-open header: ▼ + message
            if line.is_group_start {
                spans.push(Span::styled("\u{f0d7} ", Style::default().fg(theme.border)));
                let label = strip_group_prefix(&line.message);
                spans.push(Span::styled(
                    truncate(label, avail_msg.saturating_sub(2)),
                    Style::default()
                        .fg(theme.fg_muted)
                        .add_modifier(Modifier::ITALIC),
                ));
                lines.push(Line::from(spans).style(row_style));
                continue;
            }

            // group-end: render as faint separator, skip message
            if line.is_group_end {
                spans.push(Span::styled(
                    "─".repeat(avail_msg.min(20)),
                    Style::default().fg(theme.border),
                ));
                lines.push(Line::from(spans).style(Style::default()));
                continue;
            }

            // normal message line. When the runner colourised it, repaint the
            // recovered colour runs; otherwise fall back to the theme style.
            let msg = &line.message;

            if let Some(colored) = colored_spans(line, msg, msg_style, theme) {
                spans.extend(colored);
                lines.push(Line::from(spans).style(row_style));
            } else if lv.wrap_lines && crate::ui::widgets::display_width(msg) > avail_msg {
                // wrap by display width so wide chars never overflow
                // the area and desync the terminal.
                let mut first = true;
                for s in crate::ui::widgets::wrap_width(msg, avail_msg) {
                    if first {
                        spans.push(Span::styled(s, msg_style));
                        lines.push(Line::from(std::mem::take(&mut spans)).style(row_style));
                        first = false;
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw(" ".repeat(prefix_cols)),
                            Span::styled(s, msg_style),
                        ]));
                    }
                }
            } else {
                // no wrap: truncate by display width + ellipsis if too long
                let msg_display = crate::ui::widgets::truncate_width(msg, avail_msg);
                spans.push(Span::styled(msg_display, msg_style));
                lines.push(Line::from(spans).style(row_style));
            }
        }

        f.render_widget(Paragraph::new(lines), area);
    } else if lv.load_error.is_some() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "\u{f071}  log unavailable",
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "retrying automatically \u{2026}  press ",
                        Style::default().fg(theme.fg_muted),
                    ),
                    Span::styled(
                        "[R]",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to force-reload", Style::default().fg(theme.fg_muted)),
                ]),
            ]),
            area,
        );
    } else {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "\u{f110}  fetching log \u{2026}",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ]),
            area,
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Footer
// ─────────────────────────────────────────────────────────────────────────────

fn draw_footer(
    f: &mut Frame,
    area: Rect,
    lv: &crate::app::LogViewerState,
    last_export: Option<&str>,
    theme: &Theme,
) {
    let hints: &[(&str, &str)] = if lv.search_active {
        &[("↵", "confirm"), ("esc", "cancel search")]
    } else {
        &[
            ("j/k", "move"),
            ("/", "find"),
            ("n", "next"),
            ("e", "err"),
            ("y", "copy"),
            ("S", "save"),
            ("z", "fold"),
            ("t", "time"),
            ("w", "wrap"),
            ("q", "back"),
        ]
    };

    let mut spans = vec![Span::raw(" ")];
    for (key, desc) in hints {
        spans.push(Span::styled(
            format!("[{}]", key),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}  ", desc),
            Style::default().fg(theme.fg_faint),
        ));
    }

    // Confirmation of the last export, so `S` gets an answer.
    if let Some(name) = last_export {
        spans.push(Span::styled(
            format!("\u{f0c7} saved {name}"),
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn level_glyph(level: LogLevel, theme: &Theme) -> (&'static str, Style) {
    match level {
        LogLevel::Error => ("\u{f00d}", Style::default().fg(theme.failure)),
        LogLevel::Warning => ("\u{f071}", Style::default().fg(theme.warning)),
        LogLevel::Notice => ("\u{f05a}", Style::default().fg(theme.accent_dim)),
        LogLevel::Debug => ("\u{f188}", Style::default().fg(theme.fg_faint)),
        LogLevel::Normal => (" ", Style::default().fg(theme.fg_faint)),
    }
}

fn level_style(level: LogLevel, theme: &Theme) -> Style {
    match level {
        LogLevel::Error => Style::default()
            .fg(theme.failure)
            .add_modifier(Modifier::BOLD),
        LogLevel::Warning => Style::default().fg(theme.warning),
        LogLevel::Notice => Style::default().fg(theme.accent_dim),
        LogLevel::Debug => Style::default().fg(theme.fg_faint),
        LogLevel::Normal => Style::default().fg(theme.fg),
    }
}

fn truncate(s: &str, max: usize) -> String {
    crate::ui::widgets::truncate_width(s, max)
}

/// strip ##[group] / ::group:: prefix from a group header message.
/// Repaint a line with the colours the runner originally gave it.
///
/// Returns `None` when the line carries no colour, so the caller falls back
/// to the plain theme-styled path.
///
/// Basic 0-7 map onto the theme's own palette (so a red error reads as *this*
/// theme's red, not a hardcoded terminal red); 256-palette and RGB pass
/// through untouched. When the message was truncated, spans past the cut are
/// dropped and one straddling it is clipped.
fn colored_spans(
    line: &crate::gh::LogLine,
    msg: &str,
    base: ratatui::style::Style,
    theme: &Theme,
) -> Option<Vec<Span<'static>>> {
    let spans = line.color_spans.as_ref()?;
    if spans.is_empty() {
        return None;
    }

    // Copy into owned Strings: `msg` is borrowed from state, but the spans
    // outlive this frame's borrow.
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut cursor = 0usize;
    for sp in spans {
        let (start, end) = (sp.start.min(msg.len()), sp.end.min(msg.len()));
        if start >= end || !msg.is_char_boundary(start) || !msg.is_char_boundary(end) {
            continue;
        }
        if start > cursor {
            out.push(Span::styled(msg[cursor..start].to_string(), base));
        }
        out.push(Span::styled(
            msg[start..end].to_string(),
            ansi_style(sp, base, theme),
        ));
        cursor = end;
    }
    if cursor < msg.len() {
        out.push(Span::styled(msg[cursor..].to_string(), base));
    }
    if out.is_empty() { None } else { Some(out) }
}

/// One recovered SGR run as a ratatui style.
fn ansi_style(sp: &ColorSpan, base: ratatui::style::Style, theme: &Theme) -> ratatui::style::Style {
    use ratatui::style::{Color, Modifier};
    let mut style = base;
    style.fg = Some(match sp.color {
        // Basic colours resolve through the theme so "red" means this theme's
        // failure red, not whatever the user's terminal calls red.
        AnsiColor::Basic(0) => theme.neutral,
        AnsiColor::Basic(1) => theme.failure,
        AnsiColor::Basic(2) => theme.success,
        AnsiColor::Basic(3) => theme.warning,
        AnsiColor::Basic(4) => theme.accent,
        AnsiColor::Basic(5) => theme.accent_dim,
        AnsiColor::Basic(6) => theme.accent_dim,
        AnsiColor::Basic(7) => theme.fg_muted,
        AnsiColor::Basic(8) => theme.fg_faint,
        AnsiColor::Basic(9) => theme.failure,
        AnsiColor::Basic(10) => theme.success,
        AnsiColor::Basic(11) => theme.warning,
        AnsiColor::Basic(12) => theme.accent,
        AnsiColor::Basic(13) => theme.accent_dim,
        AnsiColor::Basic(14) => theme.accent_dim,
        AnsiColor::Basic(_) => theme.fg,
        // 256-palette and truecolor pass through verbatim.
        AnsiColor::Indexed(i) => Color::Indexed(i),
        AnsiColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
        // No colour change: the base foreground (theme level style) stands.
        AnsiColor::Inherit => base.fg.unwrap_or(theme.fg),
    });
    if sp.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if sp.dim {
        style = style.add_modifier(Modifier::DIM);
    }
    if sp.underline {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    style
}

fn strip_group_prefix(msg: &str) -> &str {
    for prefix in &["##[group]", "::group::"] {
        if let Some(rest) = msg.strip_prefix(prefix) {
            return rest.trim();
        }
    }
    msg.trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::builtin::dusk;

    fn line_with_spans(msg: &str, spans: Vec<crate::gh::ColorSpan>) -> crate::gh::LogLine {
        crate::gh::LogLine {
            job_id: 1,
            step_name: None,
            timestamp: None,
            message: msg.to_string(),
            level: LogLevel::Normal,
            is_group_start: false,
            is_group_end: false,
            annotation: None,
            color_spans: Some(spans),
        }
    }

    #[test]
    fn basic_colors_resolve_through_the_theme() {
        use crate::gh::{AnsiColor, ColorSpan};
        let th = dusk();
        let base = level_style(LogLevel::Normal, &th);
        let line = line_with_spans(
            "error: boom",
            vec![ColorSpan {
                start: 0,
                end: 5,
                color: AnsiColor::Basic(9),
                bold: true,
                dim: false,
                underline: false,
            }],
        );
        let out = colored_spans(&line, &line.message, base, &th).expect("must paint");
        // "error" in theme failure red + bold, ": boom" in the base style
        assert_eq!(out.len(), 2, "colored run + plain tail, got {out:?}");
        assert_eq!(out[0].content, "error");
        assert_eq!(out[0].style.fg, Some(th.failure));
        assert!(out[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(out[1].content, ": boom");
        assert_eq!(out[1].style.fg, base.fg);
    }

    #[test]
    fn palette_and_truecolor_pass_through() {
        use crate::gh::{AnsiColor, ColorSpan};
        use ratatui::style::Color;
        let th = dusk();
        let base = level_style(LogLevel::Normal, &th);
        let line = line_with_spans(
            "ab",
            vec![
                ColorSpan {
                    start: 0,
                    end: 1,
                    color: AnsiColor::Indexed(208),
                    bold: false,
                    dim: false,
                    underline: false,
                },
                ColorSpan {
                    start: 1,
                    end: 2,
                    color: AnsiColor::Rgb(10, 20, 30),
                    bold: false,
                    dim: false,
                    underline: false,
                },
            ],
        );
        let out = colored_spans(&line, &line.message, base, &th).expect("must paint");
        assert_eq!(out[0].style.fg, Some(Color::Indexed(208)));
        assert_eq!(out[1].style.fg, Some(Color::Rgb(10, 20, 30)));
    }

    #[test]
    fn no_spans_falls_back_to_plain() {
        let th = dusk();
        let base = level_style(LogLevel::Normal, &th);
        let mut line = line_with_spans("plain", vec![]);
        line.color_spans = None;
        assert!(colored_spans(&line, &line.message, base, &th).is_none());
    }
}
