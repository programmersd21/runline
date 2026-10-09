/// Last line of defence before text reaches the terminal.
///
/// ratatui writes cell symbols to the tty verbatim. A stray control byte is
/// therefore not *displayed*, it is *executed*: a carriage return rewinds the
/// cursor so every cell after it lands one position out, and an escape
/// sequence can move the cursor anywhere. Once that happens the terminal and
/// ratatui's buffer disagree about what is on screen, and because ratatui only
/// repaints cells that differ from its own model, the damage is permanent —
/// it shows up as stray text bleeding across every later screen.
///
/// Commit titles, branch names and CI output are all attacker-influenced, so
/// this runs over the finished frame rather than trusting each call site.
use ratatui::Frame;

/// Characters a terminal acts on instead of drawing.
fn is_unsafe(c: char) -> bool {
    matches!(c,
        // C0 controls (incl. \r \n \t \x1b) and DEL
        '\u{0}'..='\u{1F}' | '\u{7F}'
        // C1 controls
        | '\u{80}'..='\u{9F}'
        // zero-width / bidi formatting: invisible, but they shift the column
        // accounting ratatui and the terminal both rely on
        | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{206F}'
        | '\u{FEFF}'
    )
}

/// Replace unsafe characters in `s` with a single space.
///
/// Emptying the cell entirely would leave a stale wide-glyph behind, and a
/// space keeps column accounting honest.
pub fn safe(s: &str) -> String {
    if !s.chars().any(is_unsafe) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut replaced = false;
    for c in s.chars() {
        if is_unsafe(c) {
            if !replaced {
                out.push(' ');
                replaced = true;
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Scrub every rendered cell of `f`.
///
/// Called once at the end of the frame, after all screens and overlays have
/// drawn, so no widget can bypass it.
pub fn scrub(f: &mut Frame) {
    for cell in f.buffer_mut().content.iter_mut() {
        let symbol = cell.symbol();
        if symbol.chars().any(is_unsafe) {
            let cleaned = safe(symbol);
            // A symbol made only of control characters collapses to one
            // space; keep the cell exactly one column wide so ratatui's width
            // bookkeeping and the terminal still agree.
            let cleaned = if cleaned.is_empty() {
                " ".to_string()
            } else {
                cleaned
            };
            cell.set_symbol(&cleaned);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_carriage_return_and_escape() {
        assert_eq!(safe("downloading 10%\r100%"), "downloading 10% 100%");
        // each run of unsafe bytes collapses to a single space, so the cell
        // keeps a stable width; only the ESC itself is removed, the printable
        // remainder is left alone
        assert_eq!(safe("\u{1b}[31mred\u{1b}[0m"), " [31mred[0m");
        assert_eq!(safe("plain\u{7}text"), "plain text");
    }

    #[test]
    fn strips_bidi_and_zero_width() {
        // "trojan source": renders as one word, stores as two
        assert_eq!(safe("fi\u{202E}le"), "fi le");
        assert_eq!(safe("a\u{200B}b"), "a b");
    }

    #[test]
    fn leaves_ordinary_text_untouched() {
        for s in ["", "cargo test", "错误: 构建失败 🔥", "✓ passed"] {
            assert_eq!(safe(s), s);
        }
    }

    #[test]
    fn never_returns_a_multibyte_truncated_string() {
        // must stay valid utf-8 regardless of input
        assert!(safe("\u{1b}]0;title\u{7}ok").is_char_boundary(0));
    }
}
