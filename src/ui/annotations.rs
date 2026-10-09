use crate::gh::{Annotation, AnnotationKind};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Counts per severity, used for the summary chips.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AnnotationCounts {
    pub errors: usize,
    pub warnings: usize,
    pub notices: usize,
    pub debugs: usize,
}

impl AnnotationCounts {
    pub fn of(items: &[&Annotation]) -> Self {
        let mut c = Self::default();
        for a in items {
            match a.kind {
                AnnotationKind::Error => c.errors += 1,
                AnnotationKind::Warning => c.warnings += 1,
                AnnotationKind::Notice => c.notices += 1,
                AnnotationKind::Debug => c.debugs += 1,
            }
        }
        c
    }

    pub fn total(self) -> usize {
        self.errors + self.warnings + self.notices + self.debugs
    }
}

/// `src/gh/logs.rs:214:9` — the bit that makes an annotation actionable.
pub fn location(a: &Annotation) -> Option<String> {
    let file = a.file.as_ref()?;
    let file = file.rsplit('/').next().unwrap_or(file);
    Some(match (a.line, a.col) {
        (Some(l), Some(c)) => format!("{file}:{l}:{c}"),
        (Some(l), None) => format!("{file}:{l}"),
        _ => file.to_string(),
    })
}

fn kind_glyph(kind: AnnotationKind) -> &'static str {
    match kind {
        AnnotationKind::Error => "\u{f00d}",
        AnnotationKind::Warning => "\u{f071}",
        AnnotationKind::Notice => "\u{f05a}",
        AnnotationKind::Debug => "\u{f188}",
    }
}

fn kind_style(kind: AnnotationKind, th: &Theme) -> Style {
    let color = match kind {
        AnnotationKind::Error => th.failure,
        AnnotationKind::Warning => th.warning,
        AnnotationKind::Notice => th.accent,
        AnnotationKind::Debug => th.fg_faint,
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("{n} {one}")
    } else {
        format!("{n} {many}")
    }
}

/// Render the annotation feed for a run.
///
/// Ordered worst-first by the caller. `Scroll` lets the panel be taller than
/// its viewport without losing the tail.
pub fn draw_annotations(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    items: &[&Annotation],
    scroll: u16,
    max_rows: usize,
    expanded: bool,
) {
    let th = theme;
    let counts = AnnotationCounts::of(items);
    let width = area.width as usize;

    let mut lines: Vec<Line> = Vec::new();

    // ── summary chips ───────────────────────────────────────────────────────
    let mut chips: Vec<Span> = Vec::new();
    let mut chip = |n: usize, label: &str, kind: AnnotationKind, th: &Theme| {
        if n == 0 {
            return;
        }
        if !chips.is_empty() {
            chips.push(Span::styled("  ·  ", Style::default().fg(th.border)));
        }
        chips.push(Span::styled(
            format!("{} {}", kind_glyph(kind), n),
            kind_style(kind, th),
        ));
        chips.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(th.fg_muted),
        ));
    };
    chip(counts.errors, "errors", AnnotationKind::Error, th);
    chip(counts.warnings, "warnings", AnnotationKind::Warning, th);
    chip(counts.notices, "notices", AnnotationKind::Notice, th);
    if counts.debugs > 0 {
        chip(counts.debugs, "debug", AnnotationKind::Debug, th);
    }
    if chips.is_empty() {
        chips.push(Span::styled(
            "none",
            Style::default()
                .fg(th.fg_muted)
                .add_modifier(Modifier::BOLD),
        ));
    }

    lines.push(Line::from(chips));
    lines.push(Line::from(""));

    if items.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "\u{f2713} no annotations",
                Style::default().fg(th.success).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "this run reported no workflow errors or warnings.",
                Style::default().fg(th.fg_faint),
            ),
        ]));
        f.render_widget(Paragraph::new(lines), area);
        return;
    }

    // ── the feed ────────────────────────────────────────────────────────────
    // Build each entry as its own group (heading + message + meta), then fit
    // whole groups into the rows we actually have. A group is never split: a
    // heading without its message is exactly the cut-off this feed must not
    // produce. Entries arrive worst-first, so the most severe entry always
    // wins the space; the tail is summarised into "+ N more".
    let mut groups: Vec<Vec<Line>> = Vec::new();
    let mut last_kind: Option<AnnotationKind> = None;

    for a in items.iter() {
        let mut group: Vec<Line> = Vec::new();

        // One heading per severity run, so a wall of warnings reads as a block.
        if last_kind != Some(a.kind) {
            let label = match a.kind {
                AnnotationKind::Error => plural(counts.errors, "error", "errors"),
                AnnotationKind::Warning => plural(counts.warnings, "warning", "warnings"),
                AnnotationKind::Notice => plural(counts.notices, "notice", "notices"),
                AnnotationKind::Debug => plural(counts.debugs, "debug message", "debug messages"),
            };
            group.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(label, kind_style(a.kind, th)),
            ]));
            group.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "\u{2500}".repeat((width.saturating_sub(4)).min(72)),
                    Style::default().fg(th.border),
                ),
            ]));
            last_kind = Some(a.kind);
        }

        let mut spans = vec![
            Span::raw("  "),
            Span::styled(format!("{} ", kind_glyph(a.kind)), kind_style(a.kind, th)),
        ];

        // Reserve room for the step/file gutter so the messages stay aligned.
        let loc = location(a);
        let loc_w = loc
            .as_ref()
            .map(|l| crate::ui::widgets::display_width(l) + 1);
        let indent = loc_w.map(|w| w + 8).unwrap_or(8);

        let msg_w = width.saturating_sub(indent).max(8);
        spans.push(Span::styled(
            crate::ui::widgets::truncate_width(&a.message, msg_w),
            Style::default().fg(th.fg),
        ));
        group.push(Line::from(spans));

        // The detail line: file:line, and the step it came from.
        let mut meta: Vec<Span> = vec![Span::raw(" ".repeat(4))];
        if let Some(loc) = &loc {
            meta.push(Span::styled(
                loc.clone(),
                Style::default().fg(th.accent_dim),
            ));
        }
        if let Some(step) = &a.step {
            if !step.is_empty() {
                if !meta.len().eq(&1) {
                    meta.push(Span::styled("  ·  ", Style::default().fg(th.border)));
                }
                meta.push(Span::styled(
                    crate::ui::widgets::truncate_width(step, msg_w),
                    Style::default().fg(th.fg_faint),
                ));
            }
        }
        if meta.len() > 1 {
            group.push(Line::from(meta));
        }

        groups.push(group);
    }

    // Collapsed: fit whole entries into the rows we have, worst first.
    // Expanded: show every entry and let the caller scroll it.
    let header_rows = 2;
    let tail_rows = 2;

    let mut shown = 0usize;
    if expanded {
        for group in &groups {
            lines.extend(group.iter().cloned());
            shown += 1;
        }
    } else {
        let mut budget = max_rows.saturating_sub(header_rows);
        // Fit greedily from the most severe entry down. Peek ahead: if this
        // entry fits but a later one will not, the toggle still needs its rows.
        for (i, group) in groups.iter().enumerate() {
            let rest_cost: usize = groups[i..].iter().map(|g| g.len()).sum();
            let reserve = if rest_cost > budget { tail_rows } else { 0 };
            if group.len() + reserve <= budget {
                lines.extend(group.iter().cloned());
                budget -= group.len();
                shown += 1;
            } else {
                break;
            }
        }
    }

    if shown < groups.len() {
        // The tail was reserved above, so these two rows are guaranteed
        // visible — the control can never be pushed off-screen.
        let hidden = groups.len() - shown;
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                // A pill, so it reads as something you can press rather than
                // a caption describing a limit.
                format!(" \u{f078} {hidden} more "),
                Style::default()
                    .fg(th.accent)
                    .add_modifier(Modifier::BOLD)
                    .bg(th.match_bg),
            ),
            Span::raw(" "),
            Span::styled("press A to expand", Style::default().fg(th.fg_faint)),
        ]));
    } else if expanded && groups.len() > 1 {
        // Offer a way back down once the user has opened it up.
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                " \u{f203} collapse ",
                Style::default()
                    .fg(th.accent)
                    .add_modifier(Modifier::BOLD)
                    .bg(th.match_bg),
            ),
            Span::raw(" "),
            Span::styled("press A to collapse", Style::default().fg(th.fg_faint)),
        ]));
    }

    // Clamp to the feed's real overflow. Without this, scrolling past the last
    // entry scrolls into empty space and the tail of the feed disappears —
    // which is exactly how a warning goes missing.
    let content_rows: usize = groups.iter().map(|g| g.len()).sum::<usize>() + header_rows;
    let max_scroll = content_rows.saturating_sub(max_rows);
    let scroll = (scroll as usize).min(max_scroll) as u16;

    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ann(kind: AnnotationKind, msg: &str, file: Option<&str>, line: Option<u32>) -> Annotation {
        Annotation {
            kind,
            message: msg.into(),
            file: file.map(str::to_string),
            line,
            col: None,
            step: Some("cargo test".into()),
        }
    }

    #[test]
    fn counts_split_by_severity() {
        let items = vec![
            ann(AnnotationKind::Error, "a", None, None),
            ann(AnnotationKind::Error, "b", None, None),
            ann(AnnotationKind::Warning, "c", None, None),
            ann(AnnotationKind::Notice, "d", None, None),
        ];
        let refs: Vec<&Annotation> = items.iter().collect();
        let c = AnnotationCounts::of(&refs);
        assert_eq!(c.errors, 2);
        assert_eq!(c.warnings, 1);
        assert_eq!(c.notices, 1);
        assert_eq!(c.total(), 4);
    }

    #[test]
    fn location_shortens_path_and_keeps_line() {
        let a = ann(
            AnnotationKind::Error,
            "x",
            Some("src/gh/logs.rs"),
            Some(214),
        );
        assert_eq!(location(&a).as_deref(), Some("logs.rs:214"));
        let b = ann(AnnotationKind::Warning, "x", None, None);
        assert_eq!(location(&b), None);
    }

    #[test]
    fn severity_orders_worst_first() {
        assert!(AnnotationKind::Error.rank() < AnnotationKind::Warning.rank());
        assert!(AnnotationKind::Warning.rank() < AnnotationKind::Notice.rank());
        assert!(AnnotationKind::Notice.rank() < AnnotationKind::Debug.rank());
    }

    /// The exact cut-off from the bug report: a pane with room for the error
    /// and the warning *heading* but not the warning body must show the error
    /// plus "+ N more" — never a stranded heading.
    #[test]
    fn tight_pane_never_strands_a_heading() {
        use crate::theme::builtin::dusk;
        use ratatui::{Terminal, backend::TestBackend};

        let items = vec![
            ann(AnnotationKind::Error, "boom", Some("a.rs"), Some(1)),
            ann(AnnotationKind::Warning, "careful", Some("b.rs"), Some(2)),
        ];
        let refs: Vec<&Annotation> = items.iter().collect();
        let th = dusk();

        // inner height 8: header(2) + error group(4) + tail(2). The warning
        // heading + body (4) cannot fit, so it must be summarised, not cut.
        let backend = TestBackend::new(100, 8);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            draw_annotations(f, f.area(), &th, &refs, 0, 8, false);
        })
        .unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(text.contains("boom"), "the error itself must be visible");
        assert!(
            text.contains("1 more") && text.contains("press A to expand"),
            "the cut warning must be counted behind an expand control, got:\n{text}"
        );
        // "warning" appears once (chips). A stranded heading would make it
        // twice; the body "careful" must be absent in that case too.
        assert_eq!(
            text.matches("warning").count(),
            1,
            "warning heading stranded without its body, got:\n{text}"
        );
        assert!(!text.contains("careful"));
    }

    /// With room for everything, no tail is rendered and both bodies show.
    #[test]
    fn roomy_pane_shows_everything_without_tail() {
        use crate::theme::builtin::dusk;
        use ratatui::{Terminal, backend::TestBackend};

        let items = vec![
            ann(AnnotationKind::Error, "boom", Some("a.rs"), Some(1)),
            ann(AnnotationKind::Warning, "careful", Some("b.rs"), Some(2)),
        ];
        let refs: Vec<&Annotation> = items.iter().collect();
        let th = dusk();

        let backend = TestBackend::new(100, 20);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            draw_annotations(f, f.area(), &th, &refs, 0, 20, false);
        })
        .unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(text.contains("boom") && text.contains("careful"));
        assert!(
            !text.contains("more") && !text.contains("collapse"),
            "nothing was cut, so no expand control should be drawn, got:\n{text}"
        );
    }
}
