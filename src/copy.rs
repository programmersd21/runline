/// all user-facing static strings.
/// rule: every static string must be strictly lowercase per section 5.

pub const APP_TITLE: &str = "runline";
pub const LIVE_INDICATOR: &str = "\u{f111} live";
pub const OFFLINE_INDICATOR: &str = "○ offline";
pub const QUOTA_LABEL: &str = "quota";

// column headers
pub const COL_STATUS: &str = "status";
pub const COL_WORKFLOW: &str = "workflow";
pub const COL_BRANCH: &str = "branch";
pub const COL_ACTOR: &str = "actor";
pub const COL_AGE: &str = "age";
pub const COL_DURATION: &str = "duration";

// status labels
pub const LABEL_PASSED: &str = "passed";
pub const LABEL_FAILED: &str = "failed";
pub const LABEL_RUNNING: &str = "running";
pub const LABEL_QUEUED: &str = "queued";
pub const LABEL_CANCELLED: &str = "cancelled";
pub const LABEL_SKIPPED: &str = "skipped";
pub const LABEL_TIMED_OUT: &str = "timed out";
pub const LABEL_NEEDS_ACTION: &str = "needs action";
pub const LABEL_UNKNOWN: &str = "unknown";

// footer hints (dashboard)
pub const HINT_DASHBOARD: &str =
    "j/k move   enter open   r rerun failed   c cancel   / filter   ? help   q quit";

// footer hints (detail)
pub const HINT_DETAIL: &str = "j/k move   enter open log   a attempt   r rerun   c cancel   o browser   shift+r reload   ? help   q back";

// footer hints (log)
pub const HINT_LOG: &str =
    "n next   e error   z fold   t timestamps   w wrap   y copy   / search   ? help   q back";

// dialog buttons and prompts
pub const PROMPT_RERUN_FAILED: &str = "rerun failed jobs?";
pub const PROMPT_CANCEL_RUN: &str = "cancel run?";
pub const BTN_CONFIRM: &str = "confirm";
pub const BTN_CANCEL: &str = "cancel";

// errors and states
pub const ERR_GH_NOT_FOUND: &str =
    "gh was not found on your path. install it from cli.github.com, then run runline again.";
pub const ERR_GH_TOO_OLD: &str =
    "runline needs gh 2.40.0 or newer. upgrade with your package manager or see cli.github.com.";
pub const ERR_GH_NOT_AUTH: &str = "gh is not signed in. run: gh auth login";
pub const ERR_GH_FINE_GRAINED: &str = "gh run view does not work with fine-grained tokens. sign in with a classic token or use gh auth login.";
pub const ERR_NOT_A_REPO: &str =
    "this folder is not a github repository. run runline inside one, or add repos to your config.";
pub const ERR_RATE_LIMITED: &str =
    "github rate limit reached. runline is waiting and will resume on its own.";
pub const ERR_NO_WORKFLOWS: &str =
    "this repo has no workflows. add one under .github/workflows to see runs here.";
pub const ERR_NO_MATCHES: &str = "no runs match your filters. press esc to clear them.";

pub fn format_quota(used: u32, limit: u32) -> String {
    format!("{} {}/{}", QUOTA_LABEL, used, limit)
}

/// Build an OSC 52 "copy to clipboard" escape sequence.
///
/// OSC 52 is written straight to the tty, *outside* ratatui's cell buffer, so
/// the frame scrub in `ui::sanitize` cannot protect it. Two things make that
/// safe here:
///
/// * the payload is base64, which is exactly why the OSC 52 spec defines it
///   that way — base64's alphabet contains no BEL, so the `;` and BEL framing
///   characters can never be forged from log content;
/// * the terminator is ST (`ESC \`) rather than BEL, so a stray 0x07 elsewhere
///   on the line cannot truncate the sequence early and leave the remainder
///   printed as literal text.
pub fn osc52_copy(payload: &str) -> String {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload.as_bytes());
    format!("\x1b]52;c;{encoded}\x1b\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc52_payload_is_base64_and_cannot_break_framing() {
        // hostile: BEL, ESC, the `;` separator, and the digits that name the
        // clipboard selection
        let hostile = "boom\u{7}\u{1b}]52;c;pwned\u{7}\u{1b}evil";

        let seq = osc52_copy(hostile);

        // exactly one terminator, and it is ST
        assert!(seq.starts_with("\x1b]52;c;"), "bad preamble: {seq:?}");
        assert!(seq.ends_with("\x1b\\"), "must end with ST: {seq:?}");
        assert_eq!(seq.matches('\u{7}').count(), 0, "BEL must not survive");

        // the payload is pure base64 — no BEL, no ESC, no ';'
        let payload = &seq["\x1b]52;c;".len()..seq.len() - 2];
        assert!(
            payload
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'='),
            "payload must be base64 only: {payload:?}"
        );

        // and it round-trips back to the original text
        use base64::Engine as _;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .expect("payload must decode");
        assert_eq!(String::from_utf8(decoded).unwrap(), hostile);
    }

    #[test]
    fn test_all_copy_strings_are_lowercase() {
        let strings = [
            APP_TITLE,
            LIVE_INDICATOR,
            OFFLINE_INDICATOR,
            QUOTA_LABEL,
            COL_STATUS,
            COL_WORKFLOW,
            COL_BRANCH,
            COL_ACTOR,
            COL_AGE,
            COL_DURATION,
            LABEL_PASSED,
            LABEL_FAILED,
            LABEL_RUNNING,
            LABEL_QUEUED,
            LABEL_CANCELLED,
            LABEL_SKIPPED,
            LABEL_TIMED_OUT,
            LABEL_NEEDS_ACTION,
            LABEL_UNKNOWN,
            HINT_DASHBOARD,
            HINT_DETAIL,
            HINT_LOG,
            PROMPT_RERUN_FAILED,
            PROMPT_CANCEL_RUN,
            BTN_CONFIRM,
            BTN_CANCEL,
            ERR_GH_NOT_FOUND,
            ERR_GH_TOO_OLD,
            ERR_GH_NOT_AUTH,
            ERR_GH_FINE_GRAINED,
            ERR_NOT_A_REPO,
            ERR_RATE_LIMITED,
            ERR_NO_WORKFLOWS,
            ERR_NO_MATCHES,
        ];

        for s in strings {
            assert_eq!(
                s,
                s.to_lowercase(),
                "copy string '{}' is not strictly lowercase",
                s
            );
        }
    }
}
