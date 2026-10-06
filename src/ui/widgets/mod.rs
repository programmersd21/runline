pub mod duration;
pub mod status;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// fully transparent widgets: foreground only, never any background fill,
/// so compositor blur and terminal transparency show through.

pub fn status_glyph(h: &crate::gh::Health, ascii: bool) -> &'static str {
    if ascii {
        match h {
            crate::gh::Health::Passed => "+",
            crate::gh::Health::Failed => "x",
            crate::gh::Health::Running => "*",
            crate::gh::Health::Queued => ".",
            crate::gh::Health::Cancelled => "-",
            crate::gh::Health::Skipped => "-",
            _ => "?",
        }
    } else {
        match h {
            crate::gh::Health::Passed => "\u{f00c}",
            crate::gh::Health::Failed => "\u{f00d}",
            crate::gh::Health::Running => "\u{f110}",
            crate::gh::Health::Queued => "\u{f10c}",
            crate::gh::Health::Cancelled => "\u{f05e}",
            crate::gh::Health::Skipped => "\u{f068}",
            _ => "\u{f059}",
        }
    }
}

/// eighth-block bar cells for a fraction, minimum one sliver so short
/// durations never vanish.
pub fn bar_cells(frac: f64, width: usize) -> Vec<u8> {
    const CELLS: u8 = 8;
    if width == 0 {
        return Vec::new();
    }
    if frac <= 0.0 {
        return vec![1];
    }
    let total = (frac.clamp(0.0, 1.0) * width as f64 * CELLS as f64).round() as usize;
    let mut cells = vec![0u8; width];
    let mut rest = total;
    for c in cells.iter_mut() {
        let take = rest.min(CELLS as usize);
        *c = take as u8;
        rest -= take;
        if rest == 0 {
            break;
        }
    }
    cells
}

const PART: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

pub fn duration_bar(frac: f64, width: usize) -> String {
    bar_cells(frac, width)
        .iter()
        .map(|&c| PART[c as usize])
        .collect()
}

/// per-cell gradient spans from `dim` to `full` along the filled portion.
/// empty cells emit nothing so the terminal shows through.
pub fn gradient_bar(frac: f64, width: usize, dim: Color, full: Color) -> Vec<Span<'static>> {
    let cells = bar_cells(frac, width);
    let filled = cells.iter().filter(|&&c| c > 0).count().max(1);
    let mut i = 0;
    cells
        .into_iter()
        .filter(|&c| c > 0)
        .map(|c| {
            let t = i as f32 / filled as f32;
            i += 1;
            Span::styled(
                PART[c as usize].to_string(),
                Style::default().fg(mix(dim, full, t)),
            )
        })
        .collect()
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let m = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
            Color::Rgb(m(ar, br), m(ag, bg), m(ab, bb))
        }
        _ => b,
    }
}

pub fn fmt_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else {
        format!("{}m {:02}s", (secs / 60), secs % 60)
    }
}

pub fn sparkline(vals: &[f64]) -> String {
    const CH: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if vals.is_empty() {
        return String::new();
    }
    let (mn, mx) = vals
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    vals.iter()
        .map(|&v| {
            let t = if mx > mn { (v - mn) / (mx - mn) } else { 0.5 };
            CH[(t * 7.0).round() as usize]
        })
        .collect()
}

/// quiet key hints: keys faint-bold, descriptions fainter. no bar, no fill.
pub fn hint_bar<'a>(th: &crate::theme::Theme, hints: &[(&'a str, &'a str)]) -> Line<'a> {
    let mut spans = Vec::new();
    for (i, (k, d)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::default().fg(th.fg_faint)));
        }
        spans.push(Span::styled(
            (*k).to_string(),
            Style::default()
                .fg(th.fg_muted)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", d),
            Style::default().fg(th.fg_faint),
        ));
    }
    Line::from(spans)
}

/// hairline rule for section separation. the only chrome allowed.
pub fn rule(th: &crate::theme::Theme, width: usize) -> Line<'static> {
    Line::from(Span::styled(
        "─".repeat(width),
        Style::default().fg(th.border),
    ))
}

/// display width in terminal cells (wide chars count 2).
pub fn display_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

/// split into chunks of at most `width` display columns.
pub fn wrap_width(s: &str, width: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthChar;
    let width = width.max(1);
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut cols = 0;
    for c in s.chars() {
        let w = c.width().unwrap_or(0).max(1);
        if cols + w > width && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
            cols = 0;
        }
        cur.push(c);
        cols += w;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// truncate to `width` display columns with an ellipsis.
pub fn truncate_width(s: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    if display_width(s) <= width {
        return s.to_string();
    }
    let mut out = String::new();
    let mut cols = 0;
    for c in s.chars() {
        let w = c.width().unwrap_or(0).max(1);
        if cols + w + 1 > width {
            break;
        }
        out.push(c);
        cols += w;
    }
    out.push('\u{2026}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bar() {
        assert!(!duration_bar(0.5, 8).is_empty());
        assert!(!duration_bar(0.0, 4).is_empty());
        assert_eq!(bar_cells(0.5, 4), vec![8, 8, 0, 0]);
    }
    #[test]
    fn gradient_len() {
        let spans = gradient_bar(0.5, 4, Color::DarkGray, Color::White);
        assert_eq!(spans.len(), 2);
    }
}
