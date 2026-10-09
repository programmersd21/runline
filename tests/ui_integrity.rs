//! Guards the two invariants that keep the tui from corrupting the terminal.
//!
//! 1. No cell may contain a byte the terminal would *execute* rather than
//!    draw. Commit titles, branch names and CI output are attacker-influenced,
//!    and one stray escape is enough to desync the screen permanently.
//! 2. Truncation must be measured in display columns, not characters, or a
//!    line ends up wider than its area, wraps, and scrolls the screen out of
//!    sync with ratatui's buffer.

use ratatui::{Terminal, backend::TestBackend};
use runline::{
    app::{AppState, LogViewerState, Screen},
    config::Config,
    gh::{Conclusion, Health, RawLog, RepoRef, Run, RunStatus, parse::parse_run_detail_json},
    ui::widgets::{display_width, truncate_width, wrap_width},
};
use std::collections::HashSet;

/// Bytes a terminal acts on rather than displays.
fn is_unsafe(c: char) -> bool {
    matches!(c,
        '\u{0}'..='\u{1F}' | '\u{7F}' | '\u{80}'..='\u{9F}'
        | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}'
        | '\u{2060}'..='\u{206F}' | '\u{FEFF}')
}

fn run_at(now: chrono::DateTime<chrono::Utc>, index: u64) -> Run {
    // hostile-but-plausible: a commit title and branch carrying terminal
    // control bytes, an escape sequence and a bidi override.
    Run {
        database_id: 1000 + index,
        number: 100 + index,
        name: "ci".into(),
        workflow_name: "ci\r\n[31m".into(),
        display_title: "fix: \u{1b}[2J\u{1b}[10;1Hwreck\u{202E}evil\u{7}".into(),
        event: "push".into(),
        head_branch: "feat/\u{1b}[31mred\u{7}/x".into(),
        head_sha: "a1b2c3d".into(),
        actor: "mira".into(),
        status: RunStatus::Completed,
        conclusion: Conclusion::Failure,
        health: Health::Failed,
        created_at: now - chrono::Duration::minutes(index as i64 + 1),
        started_at: Some(now - chrono::Duration::minutes(index as i64 + 1)),
        updated_at: now,
        url: "https://github.com/o/r/actions/runs/1".into(),
        attempt: 1,
    }
}

fn seeded() -> AppState {
    let mut s = AppState::new(Config::default());
    s.theme = runline::theme::builtin::dusk();
    s.repos = vec![RepoRef::new("owner", "repo")];
    let now = chrono::Utc::now();
    let runs: Vec<Run> = (0..6).map(|i| run_at(now, i)).collect();
    s.runs.insert("owner/repo".into(), runs);

    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("tests/fixtures/run_detail.json").unwrap())
            .unwrap();
    let d = parse_run_detail_json(&v).unwrap();
    let job_id = d.jobs[0].database_id;
    s.run_details.insert(d.run.database_id, d);

    // a log carrying the same hostile bytes
    s.log_viewer = Some(LogViewerState {
        job_id,
        step_filter: None,
        log: Some(RawLog {
            job_id,
            lines: vec![runline::gh::LogLine {
                job_id,
                step_name: Some("test\u{1b}[31m".into()),
                timestamp: None,
                message: "boom \u{1b}[2J\u{1b}[5;5Hhere\u{7} done \u{202E}reversed".into(),
                level: runline::gh::LogLevel::Error,
                is_group_start: false,
                is_group_end: false,
                annotation: None,
                color_spans: None,
            }],
        }),
        load_error: None,
        selected_line: 0,
        search_query: String::new(),
        search_active: false,
        search_matches: Vec::new(),
        current_match: 0,
        folded_groups: HashSet::new(),
        show_timestamps: true,
        wrap_lines: true,
    });
    s
}

fn assert_clean(state: &AppState, w: u16, h: u16, what: &str) {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| runline::ui::draw(f, state)).unwrap();
    let buf = term.backend().buffer();
    for (i, cell) in buf.content.iter().enumerate() {
        let bad: Vec<char> = cell.symbol().chars().filter(|c| is_unsafe(*c)).collect();
        assert!(
            bad.is_empty(),
            "{what} at {w}x{h} left terminal control chars {bad:?} in cell ({},{})",
            i as u16 % w,
            i as u16 / w,
        );
    }
}

#[test]
fn no_screen_emits_terminal_control_characters() {
    let s = seeded();
    let run_id = s.filtered_runs()[0].database_id;
    let job_id = s.log_viewer.as_ref().unwrap().job_id;

    let widths = [(80u16, 24u16), (120, 40), (190, 48), (60, 16)];

    for (w, h) in widths {
        // dashboard
        let mut dash = seeded();
        dash.screen_stack = vec![Screen::Dashboard];
        dash.log_viewer = None;
        assert_clean(&dash, w, h, "dashboard");

        // detail
        let mut detail = seeded();
        detail.screen_stack = vec![Screen::Detail(run_id)];
        assert_clean(&detail, w, h, "detail");

        // log, with timestamps + wrap on (the widest possible lines)
        let mut log = seeded();
        log.screen_stack = vec![Screen::Log(run_id, job_id)];
        assert_clean(&log, w, h, "log");

        // overlays
        let mut help = seeded();
        help.show_help = true;
        assert_clean(&help, w, h, "help");

        let mut picker = seeded();
        picker.show_theme_picker = true;
        assert_clean(&picker, w, h, "theme picker");
    }
}

#[test]
fn truncation_is_measured_in_display_columns() {
    // 10 CJK chars = 20 columns
    let cjk = "错误构建失败日志输出";
    assert_eq!(display_width(cjk), 20);

    let cut = truncate_width(cjk, 8);
    assert!(
        display_width(&cut) <= 8,
        "truncate_width returned {} columns for max 8",
        display_width(&cut)
    );

    // the naive char-based truncation this replaces would have returned
    // 8 characters = 16 columns, twice the budget
    let naive: String = cjk.chars().take(8).collect();
    assert_eq!(display_width(&naive), 16);
}

#[test]
fn wrapping_never_exceeds_the_requested_columns() {
    let mixed = "错误: 构建失败 🚀 launching \u{1f680} \u{1f41b} done";
    for width in [4usize, 8, 12, 20, 40] {
        for line in wrap_width(mixed, width) {
            assert!(
                display_width(&line) <= width,
                "wrapped line is {} columns, budget was {width}",
                display_width(&line)
            );
        }
    }
}
