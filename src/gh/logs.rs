use super::*;
use chrono::{DateTime, Utc};

/// Parse the output of `gh run view --job <id> --log [--log-failed]`.
///
/// # Wire format
///
/// `gh` does not invent a log format; it takes the raw step/job log files out
/// of the run's log ZIP archive and prefixes every line with two columns it
/// already knows (see `displayLogSegments` / `copyLogWithLinePrefix` in
/// `pkg/cmd/run/view/view.go` upstream):
///
/// ```text
/// JOB_NAME \t STEP_NAME \t MESSAGE \n
/// ```
///
/// There is **no timestamp column** — `MESSAGE` is verbatim runner output and
/// starts with the runner's own timestamp:
///
/// ```text
/// build \t Run cargo test \t 2024-09-28T11:25:29.6825818Z running 12 tests
/// ```
///
/// Three things about the real stream are easy to get wrong:
///
/// * `STEP_NAME` is the literal string `UNKNOWN STEP` when the platform could
///   not map a line to a step. That is the *only* placeholder; `Set up job` and
///   `Complete job` are real steps and must keep their names, because that is
///   the string the step-scoped view filters on.
/// * the first line of every job segment is prefixed with a UTF-8 BOM
///   (`\u{FEFF}`), which otherwise swallows the timestamp of that line;
/// * a truncated final line can arrive with fewer than three columns when a
///   stream is cut mid-write.
///
/// gh also pipes the log through an ASCII sanitiser, so escape sequences are
/// normally gone by the time we see them — but they are stripped here anyway,
/// because a byte that moves the cursor desyncs ratatui's cell bookkeeping for
/// the rest of the session.
pub fn parse_log_output(job_id: JobId, raw_text: &str) -> RawLog {
    let mut lines = Vec::new();

    for raw_line in raw_text.split('\n') {
        // Sanitize the line as a whole first, but keep colour: colours are
        // recovered per-message below, after the step/timestamp columns are
        // split off (colour never legitimately appears in those columns).
        let line = strip_non_ansi_line(raw_line);
        if line.trim().is_empty() {
            continue;
        }

        let (step_name, body) = split_columns(&line);
        let (timestamp, raw_message) = split_timestamp(body);

        // Recover the message's SGR colours first, while the codes are still
        // present, then sanitise whatever is left. Reversing this order would
        // strip the codes before they could be interpreted.
        let (plain, color_spans) = parse_ansi_colors(raw_message);
        let message = scrub_text(&plain).trim_end().to_string();
        let color_spans = if message.is_empty() {
            None
        } else {
            // `parse_ansi_colors` records byte ranges into its own output;
            // clamp them to the sanitised text they actually index.
            color_spans.map(|spans| {
                spans
                    .into_iter()
                    .filter_map(|mut sp| {
                        sp.start = sp.start.min(message.len());
                        sp.end = sp.end.min(message.len());
                        if sp.start < sp.end
                            && message.is_char_boundary(sp.start)
                            && message.is_char_boundary(sp.end)
                        {
                            Some(sp)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
        };
        let color_spans = match color_spans {
            Some(v) if !v.is_empty() => Some(v),
            _ => None,
        };

        // Capture the annotation from the message *before* the marker is
        // stripped below: the `::error file=…,line=…::` form carries its
        // properties in the head, which is what makes an annotation a
        // jump-to-source link rather than just coloured text.
        let annotation = extract_annotation(&message, step_name.as_deref());

        // Workflow-command markers are only markers when they *start* the
        // message. `contains` would paint every `dir::tests::errored ... ok`
        // red, which is exactly how a log viewer lies to you.
        let (level, is_group_start, is_group_end, message) = match parse_workflow_command(&message)
        {
            Some((cmd, payload)) => {
                let payload = payload.trim_end();
                let mut level = match cmd {
                    Command::Error => LogLevel::Error,
                    Command::Warning => LogLevel::Warning,
                    Command::Notice => LogLevel::Notice,
                    Command::Debug => LogLevel::Debug,
                    // `##[command]`, `##[add-matcher]`, `##[set-output]`, …
                    // are runner bookkeeping: drop the marker, keep the
                    // payload, do not colour it.
                    Command::Group | Command::EndGroup | Command::Other => LogLevel::Normal,
                };
                if level == LogLevel::Normal {
                    level = severity_from_text(payload.trim_start());
                }
                (
                    level,
                    cmd == Command::Group,
                    cmd == Command::EndGroup,
                    payload.to_string(),
                )
            }
            None => (
                severity_from_text(message.trim_start()),
                false,
                false,
                message.to_string(),
            ),
        };

        // The runner emits a timestamp-only line for blank output; keep it so
        // line numbers still line up 1:1 with what `gh` printed. Drop only
        // lines that are whitespace with nothing at all in them.
        if message.trim().is_empty() && timestamp.is_none() && !is_group_start && !is_group_end {
            continue;
        }

        lines.push(LogLine {
            job_id,
            step_name,
            timestamp,
            message,
            level,
            is_group_start,
            is_group_end,
            annotation,
            color_spans,
        });
    }

    RawLog { job_id, lines }
}

// ─────────────────────────────────────────────────────────────────────────────
// Line format parsing
// ─────────────────────────────────────────────────────────────────────────────

/// Split `JOB \t STEP \t MESSAGE` into the step name and the message.
///
/// The message is kept whole — CI output is full of tabs (`hint: \tgit …`),
/// and `splitn` puts them back where they were.
fn split_columns(line: &str) -> (Option<String>, &str) {
    let mut it = line.splitn(3, '\t');

    // first column: job name (always present in real output)
    let _job = it.next();

    let step = match it.next() {
        Some(s) => s,
        // no columns at all — a truncated write. Keep the text; inventing a
        // step for it would be a guess.
        None => return (None, line),
    };

    let message = it.next().unwrap_or("");

    // `UNKNOWN STEP` is gh's "could not map this line to a step" marker.
    // Anything else — including `Set up job` / `Complete job` — is a real step
    // name and must survive so step-scoped views can match it.
    let step_name = if step.trim().eq_ignore_ascii_case("UNKNOWN STEP") {
        None
    } else {
        let s = step.trim();
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    };

    (step_name, message)
}

/// Split the runner's leading `YYYY-MM-DDTHH:MM:SS[.fffffff]Z ` off a message.
///
/// Exactly one separating space is removed. Everything after it is kept
/// verbatim, including indentation — losing leading whitespace is what turns
/// a compiler diagnostic or a YAML dump into an unreadable smear.
///
/// The runner also writes lines whose entire content is a timestamp (blank
/// output still gets stamped), so a body that is *only* a timestamp counts.
fn split_timestamp(body: &str) -> (Option<DateTime<Utc>>, &str) {
    let parse = |s: &str| -> Option<DateTime<Utc>> {
        if !looks_like_timestamp(s) {
            return None;
        }
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    };

    match body.find(' ') {
        Some(space) => match parse(&body[..space]) {
            Some(ts) => (Some(ts), &body[space + 1..]),
            None => (None, body),
        },
        None => match parse(body) {
            Some(ts) => (Some(ts), ""),
            None => (None, body),
        },
    }
}

/// Cheap shape check before handing a token to the (much slower) RFC-3339
/// parser. The runner emits 100-ns ticks: `2024-09-28T11:25:29.6825818Z`.
fn looks_like_timestamp(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20 {
        return false;
    }
    b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
        && (b[10] == b'T' || b[10] == b' ')
        && b[11..13].iter().all(u8::is_ascii_digit)
        && b[13] == b':'
        && b[14..16].iter().all(u8::is_ascii_digit)
        && b[16] == b':'
}

// ─────────────────────────────────────────────────────────────────────────────
// Workflow commands
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Group,
    EndGroup,
    Error,
    Warning,
    Notice,
    Debug,
    Other,
}

/// Recognise a workflow command at the head of a message.
///
/// Both dialects appear in real logs:
///
/// * `##[group]Title`, `##[endgroup]`, `##[error]msg` — what the runner writes
///   into the log stream;
/// * `::group::Title`, `::error file=app.rs,line=12,col=3::msg` — the
///   documented workflow-command syntax, which also survives into logs.
///
/// Returns the command and the payload with the marker (and, for the `::` form,
/// the `key=value,` properties) removed.
fn parse_workflow_command(msg: &str) -> Option<(Command, &str)> {
    // ── ##[name]… ────────────────────────────────────────────────────────────
    if let Some(rest) = msg.strip_prefix("##[") {
        let end = rest.find(']')?;
        let cmd = match rest[..end].trim() {
            "group" => Command::Group,
            "endgroup" => Command::EndGroup,
            "error" => Command::Error,
            "warning" => Command::Warning,
            "notice" => Command::Notice,
            "debug" => Command::Debug,
            _ => Command::Other,
        };
        // `##[command]/usr/bin/git init` — no space after the bracket.
        return Some((cmd, rest[end + 1..].trim_start()));
    }

    // ── ::name[ props]::payload ───────────────────────────────────────────────
    if let Some(rest) = msg.strip_prefix("::") {
        let sep = rest.find("::")?;
        let head = &rest[..sep];
        // Properties look like `file=app.rs,line=12` or `title=Build failed`.
        let name = head
            .split(|c| c == ',' || c == ' ')
            .next()
            .unwrap_or(head)
            .trim();
        let cmd = match name {
            "group" => Command::Group,
            "endgroup" => Command::EndGroup,
            "error" => Command::Error,
            "warning" => Command::Warning,
            "notice" => Command::Notice,
            "debug" => Command::Debug,
            _ => Command::Other,
        };
        return Some((cmd, rest[sep + 2..].trim_start()));
    }

    None
}

/// Properties carried by `::name key=value,…::payload`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AnnotationProps {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub col: Option<u32>,
}

/// Pull `file=`, `line=` and `col=` out of a workflow-command head.
///
/// `::error file=src/main.rs,line=42,col=3::boom` — comma and space are both
/// legal separators between properties, and values are taken verbatim so a path
/// containing `,` is not mangled further.
fn annotation_props(head: &str) -> AnnotationProps {
    let mut props = AnnotationProps::default();
    for chunk in head.split(|c| c == ',' || c == ' ') {
        let Some((key, value)) = chunk.split_once('=') else {
            continue;
        };
        match key.trim() {
            "file" if !value.is_empty() => props.file = Some(value.to_string()),
            "line" => props.line = value.trim().parse().ok(),
            "col" => props.col = value.trim().parse().ok(),
            _ => {}
        }
    }
    props
}

/// Recognise and decode a workflow command, returning its annotation.
///
/// Only the four annotation commands produce anything; `##[group]`, `##[command]`
/// and friends are runner bookkeeping. The `::` form can carry `file`/`line`/
/// `col`, which is how an annotation becomes a jump-to-source link.
pub fn extract_annotation(msg: &str, step: Option<&str>) -> Option<Annotation> {
    let (cmd, payload) = parse_workflow_command(msg)?;

    let kind = match cmd {
        Command::Error => AnnotationKind::Error,
        Command::Warning => AnnotationKind::Warning,
        Command::Notice => AnnotationKind::Notice,
        Command::Debug => AnnotationKind::Debug,
        // group/endgroup/command/add-matcher/set-output are not annotations.
        Command::Group | Command::EndGroup | Command::Other => return None,
    };

    let payload = payload.trim();
    if payload.is_empty() {
        return None;
    }

    // `::`-form commands put their properties in the head; `##[error]` cannot.
    let props = if msg.starts_with("::") {
        // head is whatever sits between the leading `::` and the closing `::`
        let after = &msg[2..];
        let head_end = after.find("::").unwrap_or(after.len());
        annotation_props(&after[..head_end])
    } else {
        AnnotationProps::default()
    };

    Some(Annotation {
        kind,
        message: payload.to_string(),
        file: props.file,
        line: props.line,
        col: props.col,
        step: step.map(str::to_string),
    })
}

/// Severity of an ordinary line, from the prefixes toolchain output actually
/// uses. Prefix-only, and only when something follows the colon, so a bare
/// heading called `Errors:` is not repainted as four errors.
fn severity_from_text(msg: &str) -> LogLevel {
    for (prefix, level) in [
        ("error:", LogLevel::Error),
        ("fatal:", LogLevel::Error),
        ("warning:", LogLevel::Warning),
    ] {
        if starts_with_ci(msg, prefix) && msg.len() > prefix.len() {
            return level;
        }
    }
    LogLevel::Normal
}

// ─────────────────────────────────────────────────────────────────────────────
// Text cleaning
// ─────────────────────────────────────────────────────────────────────────────

/// Turn one raw byte line into a display-safe line.
///
/// * A carriage return overwrites in place on a real terminal, so a progress
///   line only *ends up* showing what follows its final `\r`.
/// * The first line of each job segment carries a UTF-8 BOM; left alone it
///   hides that line's timestamp.
/// * Remaining C0/C1 controls would move the cursor instead of drawing.
///   Tabs survive — CI output uses them for indentation.
/// Sanitise a line before column splitting: drop stray control characters,
/// but keep ESC and caret-notation escapes intact — they are interpreted
/// per-message by `parse_ansi_colors`. Stripping them here would delete the
/// colour information before it can be recovered.
fn strip_non_ansi_line(line: &str) -> String {
    // Progress-overwrite semantics: only the final segment survives.
    let line = match line.rfind('\r') {
        Some(pos) => &line[pos + 1..],
        None => line,
    };
    line.chars()
        .filter(|c| {
            *c == '\u{1b}' || *c == '^' || (!c.is_control() && *c != '\u{FEFF}' || *c == '\t')
        })
        .collect()
}

/// Sanitise a message that carries no colour: every escape sequence is
/// removed and only printable text survives.
fn scrub_text(s: &str) -> String {
    strip_ansi(s)
        .chars()
        .filter(|c| !c.is_control() && *c != '\u{FEFF}' || *c == '\t')
        .collect()
}

/// Recover colour runs from an ANSI-coloured line, and return the plain text
/// with every escape removed.
///
/// Compiler output is colourised on purpose — red for errors, yellow for
/// warnings — and that signal is worth keeping. Stripping the codes flattened
/// every line to one colour, so a reader had to parse `[0m` noise to find out
/// what actually failed.
///
/// Returns `(plain, spans)`. Spans are byte ranges into `plain`, and are `None`
/// when the line carried no colour at all.
///
/// Non-SGR sequences (cursor movement, OSC, DCS) are always removed and never
/// produce a span: they are not display attributes and replaying them would
/// move the cursor mid-frame.
fn parse_ansi_colors(s: &str) -> (String, Option<Vec<crate::gh::ColorSpan>>) {
    use crate::gh::{AnsiColor, ColorSpan};

    let mut plain = String::with_capacity(s.len());
    let mut spans: Vec<ColorSpan> = Vec::new();

    // current SGR state
    let mut fg: Option<AnsiColor> = None;
    let mut bold = false;
    let mut dim = false;
    let mut underline = false;
    let mut saw_style = false;
    // byte offset in `plain` where the current style began
    let mut style_start = 0usize;

    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        // Caret-notation escape: `^` + `[` stands in for ESC.
        let esc = ch == '\u{1b}' || (ch == '^' && matches!(chars.peek(), Some('[')));
        if !esc {
            plain.push(ch);
            continue;
        }
        if ch == '^' {
            chars.next(); // the '[' that stands in for ESC
        }

        // Need `[` to be a CSI; a bare two-char escape is dropped.
        if chars.peek() != Some(&'[') {
            // e.g. ESC ] OSC, ESC ( charset select — discard through the
            // sequence terminator so its body cannot leak as text.
            discard_escape(&mut chars);
            continue;
        }
        chars.next(); // '['

        // collect the parameter/intermediate bytes up to the final byte
        let mut body = String::new();
        for c in chars.by_ref() {
            if ('\x40'..='\x7e').contains(&c) {
                if c != 'm' {
                    // some other CSI final byte: not a style, drop it
                    body.clear();
                    fg = None;
                    bold = false;
                    dim = false;
                    underline = false;
                }
                break;
            }
            body.push(c);
        }

        // Close the run styled by the *previous* SGR before interpreting the
        // new one — including resets. Otherwise the text accumulated so far
        // is never attributed to any style.
        close_span(
            &mut spans,
            &mut style_start,
            &plain,
            fg,
            bold,
            dim,
            underline,
        );

        if !body.is_empty() {
            apply_sgr(
                &body,
                &mut fg,
                &mut bold,
                &mut dim,
                &mut underline,
                &mut saw_style,
            );
        } else {
            // bare `ESC [ m` == reset
            fg = None;
            bold = false;
            dim = false;
            underline = false;
        }
        style_start = plain.len();
    }

    if fg.is_some() || bold || dim || underline {
        close_span(
            &mut spans,
            &mut style_start,
            &plain,
            fg,
            bold,
            dim,
            underline,
        );
    }

    let spans = if saw_style && !spans.is_empty() {
        Some(spans)
    } else {
        None
    };
    (plain, spans)
}

/// Drop a non-CSI escape sequence's body so it cannot reach the screen.
fn discard_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match chars.peek() {
        // OSC runs to BEL or ST
        Some(']') => {
            chars.next();
            for c in chars.by_ref() {
                if c == '\u{7}' {
                    return;
                }
                if c == '\u{1b}' {
                    if chars.peek() == Some(&'\\') {
                        chars.next();
                    }
                    return;
                }
            }
        }
        // DCS/APC/PM are string sequences too
        Some('P') | Some('_') | Some('^') => {
            chars.next();
            while let Some(c) = chars.next() {
                if c == '\u{7}' {
                    return;
                }
                if c == '\u{1b}' {
                    if chars.peek() == Some(&'\\') {
                        chars.next();
                    }
                    return;
                }
            }
        }
        // everything else is a two-character escape
        Some(_) => {
            chars.next();
        }
        None => {}
    }
}

/// Close the current styled run, if any.
#[allow(clippy::too_many_arguments)]
fn close_span(
    spans: &mut Vec<crate::gh::ColorSpan>,
    style_start: &mut usize,
    plain: &str,
    fg: Option<crate::gh::AnsiColor>,
    bold: bool,
    dim: bool,
    underline: bool,
) {
    // A run with modifiers but no colour still matters (`\x1b[1m` around a
    // message) — it keeps the base foreground and adds the modifiers.
    if fg.is_none() && !bold && !dim && !underline {
        return;
    }
    if *style_start >= plain.len() {
        return;
    }
    spans.push(crate::gh::ColorSpan {
        start: *style_start,
        end: plain.len(),
        color: fg.unwrap_or(crate::gh::AnsiColor::Inherit),
        bold,
        dim,
        underline,
    });
}

/// Apply one SGR parameter list (the text between `ESC [` and the final `m`).
fn apply_sgr(
    body: &str,
    fg: &mut Option<crate::gh::AnsiColor>,
    bold: &mut bool,
    dim: &mut bool,
    underline: &mut bool,
    saw_style: &mut bool,
) {
    use crate::gh::AnsiColor;
    let parts: Vec<&str> = body.split(';').collect();
    let mut i = 0;
    while i < parts.len() {
        // an empty field means 0
        let n: u16 = parts[i].parse().unwrap_or(0);
        match n {
            0 => {
                *fg = None;
                *bold = false;
                *dim = false;
                *underline = false;
            }
            1 => {
                *bold = true;
                *saw_style = true;
            }
            2 => {
                *dim = true;
                *saw_style = true;
            }
            4 => {
                *underline = true;
                *saw_style = true;
            }
            22 => {
                *bold = false;
                *dim = false;
            }
            24 => *underline = false,
            // 30-37 basic, 90-97 bright basic
            30..=37 | 90..=97 => {
                let base = if n >= 90 { n - 90 + 8 } else { n - 30 };
                *fg = Some(AnsiColor::Basic(base as u8));
                *saw_style = true;
            }
            // 38;5;n (256) or 38;2;r;g;b (truecolor)
            38 => match parts.get(i + 1).map(|s| s.parse::<u16>().unwrap_or(0)) {
                Some(5) => {
                    if let Some(idx) = parts.get(i + 2).and_then(|s| s.parse::<u8>().ok()) {
                        *fg = Some(AnsiColor::Indexed(idx));
                        *saw_style = true;
                    }
                    i += 2;
                }
                Some(2) => {
                    let r = parts.get(i + 2).and_then(|s| s.parse::<u8>().ok());
                    let g = parts.get(i + 3).and_then(|s| s.parse::<u8>().ok());
                    let b = parts.get(i + 4).and_then(|s| s.parse::<u8>().ok());
                    if let (Some(r), Some(g), Some(b)) = (r, g, b) {
                        *fg = Some(AnsiColor::Rgb(r, g, b));
                        *saw_style = true;
                    }
                    i += 4;
                }
                _ => {}
            },
            // 39 = default foreground
            39 => *fg = None,
            // 40-47 / 100-107 background — background fills would fight the
            // transparent theme, so the foreground is left alone.
            _ => {}
        }
        i += 1;
    }
}

/// Strip ANSI escape sequences (colour codes, cursor movement, OSC title
/// set) that a runner or a workflow step may emit into its stdout.
///
/// Two encodings occur in practice and both must be handled:
///
/// * the raw control byte — `\x1b[31m`;
/// * the **caret notation** `\x5b` → `^[`, which is how `cat -v` and, more
///   importantly, GitHub's own log archive render ESC when it cannot store a
///   control byte. A log line full of compiler colour then arrives as literal
///   `^[[36;1m` text, which is what actually shows up in the viewer.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        // `^` that begins a caret-notation escape is not literal text.
        if ch == '^' && matches!(chars.peek(), Some('[')) {
            chars.next(); // the '[' that stands in for ESC
            // Caret notation spells ESC as `^` + `[`, so a CSI sequence is
            // `^` `[` `[` — a *second* `[` opens the control sequence, exactly
            // as the bare `[` does in the raw ESC form below. Without eating it
            // here, that `[` looks like a final byte and the parameters and the
            // final byte both leak through as text.
            if chars.peek() == Some(&'[') {
                chars.next();
            }
            for c in chars.by_ref() {
                if ('\x40'..='\x7e').contains(&c) {
                    break;
                }
            }
            continue;
        }
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next(); // consume '['
                // CSI: parameter bytes, then intermediates, then one final byte.
                for c in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&c) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next(); // consume ']'
                // OSC: terminated by BEL, or by ESC \ (ST).
                for c in chars.by_ref() {
                    if c == '\u{7}' {
                        break;
                    }
                    if c == '\u{1b}' {
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                }
            }
            Some(_) => {
                chars.next(); // two-character escape
            }
            None => {}
        }
    }
    out
}

/// ASCII-case-insensitive `starts_with` without allocating.
fn starts_with_ci(s: &str, prefix: &str) -> bool {
    let b = s.as_bytes();
    let p = prefix.as_bytes();
    b.len() >= p.len() && b[..p.len()].eq_ignore_ascii_case(p)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim shape of `gh run view --job <id> --log`, captured from
    /// `gh 2.102.0`. Note: no timestamp column — the timestamp is part of
    /// the message and separated from it by a single space.
    const REAL: &str = "\
wasm\tUNKNOWN STEP\t\u{feff}2026-10-11T01:34:14.0522012Z Current runner version: '2.337.0'
wasm\tRun cargo test\t2026-10-11T01:34:15.0360355Z ##[group]Run cargo test
wasm\tRun cargo test\t2026-10-11T01:34:15.0365000Z \u{feff}hint: \tgit branch -m <name>
wasm\tRun cargo test\t2026-10-11T01:34:15.0369000Z test dir::tests::errored ... ok
wasm\tRun cargo test\t2026-10-11T01:34:15.0370000Z ##[warning]Node.js 20 is deprecated.
wasm\tRun cargo test\t2026-10-11T01:34:15.0375000Z ##[error]test parser failed
wasm\tRun cargo test\t2026-10-11T01:34:15.0379000Z ##[endgroup]
wasm\tComplete job\t2026-10-11T01:34:16.0000000Z ##[group]Operating System
wasm\tComplete job\t2026-10-11T01:34:16.0001000Z ##[endgroup]
";

    #[test]
    fn parses_the_real_three_column_format() {
        let log = parse_log_output(42, REAL);
        assert_eq!(log.lines.len(), 9);
        assert_eq!(log.lines[0].step_name, None, "UNKNOWN STEP → no step");
        assert_eq!(
            log.lines[0].message, "Current runner version: '2.337.0'",
            "BOM must not eat the timestamp's message"
        );
        assert!(
            log.lines[0].timestamp.is_some(),
            "BOM must not eat the timestamp itself"
        );
        assert_eq!(log.lines[1].step_name.as_deref(), Some("Run cargo test"));
        assert_eq!(log.lines[1].message, "Run cargo test");
        assert!(log.lines[1].is_group_start);
        assert_eq!(log.lines[7].message, "Operating System");
        assert!(log.lines[7].is_group_start);
        assert!(log.lines[8].is_group_end);
    }

    #[test]
    fn every_real_line_gets_a_step_and_a_timestamp() {
        let log = parse_log_output(42, REAL);
        for l in &log.lines {
            assert!(l.timestamp.is_some(), "missing timestamp: {:?}", l.message);
        }
        assert_eq!(
            log.lines.iter().filter(|l| l.step_name.is_some()).count(),
            8,
            "every line except the UNKNOWN STEP one carries its step"
        );
    }

    #[test]
    fn tabs_inside_the_message_survive() {
        let log = parse_log_output(
            7,
            "j\tStep\t2024-09-28T11:25:29.6825818Z hint: \tgit config\n",
        );
        assert_eq!(log.lines[0].message, "hint: \tgit config");
        assert_eq!(log.lines[0].step_name.as_deref(), Some("Step"));
    }

    #[test]
    fn indentation_is_preserved() {
        let raw = "j\tS\t2024-09-28T11:25:29Z       left: \"Failed\"\n\
                   j\tS\t2024-09-28T11:25:29Z  right: \"failed\"\n";
        let log = parse_log_output(1, raw);
        assert_eq!(log.lines[0].message, "      left: \"Failed\"");
        assert_eq!(log.lines[1].message, " right: \"failed\"");
    }

    /// GitHub's log archive stores ESC as `^[` caret notation, so a compiler
    /// colour arrives as literal text. It must be stripped, not displayed.
    #[test]
    fn strips_caret_notation_ansi() {
        let raw = concat!(
            "j\tstep\t2026-01-01T00:00:00Z ^[[36;1mcargo clippy^[[0m\n",
            "j\tstep\t2026-01-01T00:00:01Z ^[[1m^[[91merror^[[0m: too many arguments\n",
            "j\tstep\t2026-01-01T00:00:02Z   ^[[1m^[[94m819^[[0m ^[[1m^[[94m |\n",
            "j\tstep\t2026-01-01T00:00:03Z plain text stays\n",
        );
        let log = parse_log_output(1, raw);
        for l in &log.lines {
            assert!(
                !l.message.contains("^["),
                "caret-notation ANSI survived: {:?}",
                l.message
            );
        }
        // the text around the escapes must survive intact
        assert_eq!(log.lines[0].message, "cargo clippy");
        assert_eq!(log.lines[1].message, "error: too many arguments");
        assert_eq!(log.lines[3].message, "plain text stays");
    }

    /// A bare `^` and a `^[` that is not a complete sequence are real text
    /// and must not be eaten.
    #[test]
    fn keeps_caret_that_is_not_an_escape() {
        let raw = concat!(
            "j\tstep\t2026-01-01T00:00:00Z 2 ^ 3\n",
            "j\tstep\t2026-01-01T00:00:01Z arr[^0] and a[0] literal\n",
            "j\tstep\t2026-01-01T00:00:02Z 100% done^\n",
        );
        let log = parse_log_output(1, raw);
        assert_eq!(log.lines[0].message, "2 ^ 3");
        assert_eq!(log.lines[1].message, "arr[^0] and a[0] literal");
        assert_eq!(log.lines[2].message, "100% done^");
    }

    /// The real ESC encoding must keep working — the caret form is additive,
    /// not a replacement.
    #[test]
    fn still_strips_raw_escape() {
        let raw = concat!(
            "j\tstep\t2026-01-01T00:00:00Z \u{1b}[31mred\u{1b}[0m tail\n",
            "j\tstep\t2026-01-01T00:00:01Z \u{1b}[2J\u{1b}[9;1Hmoved\n",
        );
        let log = parse_log_output(1, raw);
        assert_eq!(log.lines[0].message, "red tail");
        assert_eq!(log.lines[1].message, "moved");
    }

    #[test]
    fn only_real_annotations_are_errors() {
        // `contains("::error")` used to paint all of these red.
        for msg in [
            "test dir::tests::errored ... ok",
            "test flags::config::tests::error ... ok",
            "error handling is documented",
        ] {
            let raw = format!("j\tS\t2024-09-28T11:25:29Z {}\n", msg);
            let log = parse_log_output(1, &raw);
            assert_eq!(
                log.lines[0].level,
                LogLevel::Normal,
                "false positive: {:?}",
                msg
            );
        }
    }

    #[test]
    fn annotation_levels() {
        let cases = [
            ("##[error]boom", LogLevel::Error),
            ("##[warning]careful", LogLevel::Warning),
            ("##[notice]fyi", LogLevel::Notice),
            ("##[debug]noisy", LogLevel::Debug),
            ("::error::boom", LogLevel::Error),
            ("::warning file=a,line=1::boom", LogLevel::Warning),
            ("::error file=a,line=1::boom", LogLevel::Error),
            ("error: compile failed", LogLevel::Error),
            ("nothing to see", LogLevel::Normal),
        ];
        for (msg, want) in cases {
            let raw = format!("j\tS\t2024-09-28T11:25:29Z {}\n", msg);
            assert_eq!(parse_log_output(1, &raw).lines[0].level, want, "{}", msg);
        }
    }

    #[test]
    fn workflow_command_properties_are_stripped() {
        let raw =
            "j\tS\t2024-09-28T11:25:29Z ::error file=src/main.rs,line=9,col=2::mismatched types\n";
        assert_eq!(
            parse_log_output(1, raw).lines[0].message,
            "mismatched types"
        );
    }

    #[test]
    fn runner_bookkeeping_is_neutral() {
        // the `##[…]` dialect: marker dropped, payload kept
        let raw = "j\tS\t2024-09-28T11:25:29Z ##[command]/usr/bin/git init\n";
        let l = &parse_log_output(1, raw).lines[0];
        assert_eq!(l.level, LogLevel::Normal);
        assert_eq!(l.message, "/usr/bin/git init");

        // what the runner actually writes: a bare `[command]` tag, which is
        // informative on its own and must survive untouched
        let raw = "j\tS\t2024-09-28T11:25:29Z [command]/usr/bin/git init\n";
        let l = &parse_log_output(1, raw).lines[0];
        assert_eq!(l.level, LogLevel::Normal);
        assert_eq!(l.message, "[command]/usr/bin/git init");
    }

    #[test]
    fn groups_nest_and_close() {
        let raw = "j\tS\t2024-01-01T00:00:00Z ##[group]Outer\n\
                   j\tS\t2024-01-01T00:00:01Z ##[group]Inner\n\
                   j\tS\t2024-01-01T00:00:02Z body\n\
                   j\tS\t2024-01-01T00:00:03Z ##[endgroup]\n\
                   j\tS\t2024-01-01T00:00:04Z ##[endgroup]\n";
        let log = parse_log_output(1, raw);
        assert!(log.lines[0].is_group_start);
        assert!(log.lines[1].is_group_start);
        assert!(!log.lines[2].is_group_start && !log.lines[2].is_group_end);
        assert!(log.lines[3].is_group_end);
        assert!(log.lines[4].is_group_end);
        assert_eq!(log.lines[4].message, "", "endgroup carries no payload");
    }

    #[test]
    fn carriage_returns_use_overwrite_semantics() {
        let raw = "j\tS\t2024-09-28T11:25:29Z downloading 10%\rdownloading 100%\n\
                   j\tS\t2024-09-28T11:25:30Z plain\u{7}bell\n";
        let log = parse_log_output(1, raw);
        assert_eq!(log.lines.len(), 2);
        assert_eq!(log.lines[0].message, "downloading 100%");
        assert_eq!(log.lines[1].message, "plainbell");
    }

    #[test]
    fn nanosecond_timestamps_parse() {
        let raw = "j\tS\t2024-09-28T11:25:29.6825818Z tick\n";
        let ts = parse_log_output(1, raw).lines[0].timestamp.unwrap();
        assert_eq!(ts.timestamp_subsec_micros(), 682581);
    }

    #[test]
    fn truncated_tail_line_is_kept_as_text() {
        let raw = "j\tS\t2024-09-28T11:25:29Z complete\ntest (win-gnu, nightly";
        let log = parse_log_output(1, raw);
        assert_eq!(log.lines.len(), 2);
        assert_eq!(log.lines[1].step_name, None);
        assert_eq!(log.lines[1].message, "test (win-gnu, nightly");
    }

    #[test]
    fn escape_sequences_are_stripped() {
        let raw = "j\tS\t2024-09-28T11:25:29Z \u{1b}[32mhello\u{1b}[0m \u{1b}]0;hijack\u{7}\n";
        assert_eq!(parse_log_output(1, raw).lines[0].message, "hello");
    }

    #[test]
    fn blank_lines_are_dropped() {
        let raw = "j\tS\t2024-09-28T11:25:29Z a\n\n\nj\tS\t2024-09-28T11:25:30Z b\n";
        assert_eq!(parse_log_output(1, raw).lines.len(), 2);
    }
}
