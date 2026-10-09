//! The log screen, rendered the way a user actually sees it.
//!
//! Every fixture here is the verbatim `gh run view --job <id> --log` wire
//! format — `JOB \t STEP \t MESSAGE` — so a parser regression shows up as a
//! wrong frame rather than as a passing unit test.

use ratatui::{Terminal, backend::TestBackend};
use runline::app::{
    AppState, LogViewerState, Screen, clamp_log_selection, group_start_at, move_log_selection,
    visible_rows,
};
use runline::config::Config;
use runline::gh::LogLevel;
use runline::gh::logs::parse_log_output;

const RAW: &str = "\
build\tUNKNOWN STEP\t\u{feff}2024-09-28T11:25:29.6825818Z Current runner version: '2.337.0'\n\
build\tSet up job\t2024-09-28T11:25:29.7000000Z ##[group]Operating System\n\
build\tSet up job\t2024-09-28T11:25:29.7001000Z Ubuntu 24.04.5 LTS\n\
build\tSet up job\t2024-09-28T11:25:29.7002000Z ##[endgroup]\n\
build\tRun actions/checkout@v4\t2024-09-28T11:25:31.0000000Z ##[group]Run actions/checkout@v4\n\
build\tRun actions/checkout@v4\t2024-09-28T11:25:31.0001000Z [command]/usr/bin/git --version\n\
build\tRun actions/checkout@v4\t2024-09-28T11:25:31.4001000Z hint: \tgit branch -m <name>\n\
build\tRun actions/checkout@v4\t2024-09-28T11:25:31.6000000Z ##[warning]Node.js 20 is deprecated.\n\
build\tRun actions/checkout@v4\t2024-09-28T11:25:31.7000000Z ##[endgroup]\n\
build\tRun cargo test\t2024-09-28T11:25:33.0000000Z ##[group]Run cargo test\n\
build\tRun cargo test\t2024-09-28T11:25:33.1000000Z test dir::tests::errored ... ok\n\
build\tRun cargo test\t2024-09-28T11:25:33.2000000Z     Compiling runline v1.0.0\n\
build\tRun cargo test\t2024-09-28T11:25:33.5000000Z ##[error]test parse::tests::test_log_parser ... FAILED\n\
build\tRun cargo test\t2024-09-28T11:25:33.6001000Z   left: \"Failed\"\n\
build\tRun cargo test\t2024-09-28T11:25:33.8000000Z ##[endgroup]\n\
build\tComplete job\t2024-09-28T11:25:35.2000000Z ##[error]Process completed with exit code 101.\n";

fn viewer(step_filter: Option<&str>) -> AppState {
    let mut s = AppState::new(Config::default());
    s.theme = runline::theme::builtin::dusk();
    s.repos = vec![runline::gh::RepoRef::new("owner", "repo")];
    s.screen_stack = vec![Screen::Log(1, 2)];
    s.log_viewer = Some(LogViewerState {
        job_id: 2,
        step_filter: step_filter.map(str::to_string),
        log: Some(parse_log_output(2, RAW)),
        load_error: None,
        selected_line: 0,
        search_query: String::new(),
        search_active: false,
        search_matches: Vec::new(),
        current_match: 0,
        folded_groups: std::collections::HashSet::new(),
        show_timestamps: false,
        wrap_lines: false,
    });
    s
}

fn frame(state: &AppState) -> String {
    let mut term = Terminal::new(TestBackend::new(100, 24)).unwrap();
    term.draw(|f| runline::ui::draw(f, state)).unwrap();
    term.backend()
        .buffer()
        .content()
        .chunks(term.backend().buffer().area.width as usize)
        .map(|r| r.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

// ── the whole job ────────────────────────────────────────────────────────────

#[test]
fn every_line_is_stamped_and_its_step_known() {
    let log = parse_log_output(2, RAW);
    assert_eq!(log.lines.len(), 16);
    assert!(
        log.lines.iter().all(|l| l.timestamp.is_some()),
        "a line without a timestamp renders in a blank gutter column forever"
    );
    let errs: Vec<&str> = log
        .lines
        .iter()
        .filter(|l| l.level == LogLevel::Error)
        .map(|l| l.message.as_str())
        .collect();
    assert_eq!(
        errs,
        vec![
            "test parse::tests::test_log_parser ... FAILED",
            "Process completed with exit code 101.",
        ],
        "`dir::tests::errored ... ok` is not an error"
    );
}

#[test]
fn full_job_frame_shows_steps_tabs_and_indentation() {
    let out = frame(&viewer(None));
    assert!(out.contains("1/16"), "header counts rows:\n{}", out);
    assert!(out.contains("Current runner version"), "{}", out);
    // the tab inside the message survives and does not eat the gutter
    assert!(out.contains("git branch -m <name>"), "{}", out);
    // indentation inside cargo's output is preserved
    assert!(out.contains("    Compiling runline"), "{}", out);
    // annotation markers are not shown raw
    assert!(!out.contains("##["), "{}", out);
}

#[test]
fn timestamps_toggle_adds_the_gutter() {
    let mut s = viewer(None);
    s.log_viewer.as_mut().unwrap().show_timestamps = true;
    let out = frame(&s);
    assert!(out.contains("11:25:29"), "{}", out);
}

// ── scoped to a step ─────────────────────────────────────────────────────────

#[test]
fn step_scope_hides_every_other_step() {
    let mut s = viewer(Some("Run cargo test"));
    clamp_log_selection(&mut s);
    let rows = visible_rows(s.log_viewer.as_ref().unwrap());
    let log = s.log_viewer.as_ref().unwrap().log.as_ref().unwrap();
    assert_eq!(rows.len(), 6, "exactly the cargo test lines");
    for &(i, _) in &rows {
        assert_eq!(log.lines[i].step_name.as_deref(), Some("Run cargo test"));
    }

    let out = frame(&s);
    assert!(
        out.contains("1/6"),
        "counter shows the scoped row count:\n{out}"
    );
    assert!(out.contains("Run cargo test"), "breadcrumb:\n{out}");
    assert!(out.contains("test parse::tests::test_log_parser"), "{out}");
    assert!(
        !out.contains("git branch -m"),
        "another step's output leaked in:\n{out}"
    );
    assert!(
        !out.contains("Operating System"),
        "another step's group header leaked in:\n{out}"
    );
}

#[test]
fn step_scope_falls_back_when_nothing_matches() {
    // gh labels every line UNKNOWN STEP when the platform cannot map logs to
    // steps; a scoped view must not become an empty screen.
    let mut s = viewer(Some("Run cargo test"));
    s.log_viewer.as_mut().unwrap().log = Some(parse_log_output(
        2,
        "build\tUNKNOWN STEP\t2024-09-28T11:25:29.6825818Z hello\n",
    ));
    clamp_log_selection(&mut s);
    let rows = visible_rows(s.log_viewer.as_ref().unwrap());
    assert_eq!(rows.len(), 1);
    let out = frame(&s);
    assert!(out.contains("hello"), "{out}");
    // ...and it has to say why it is not showing the step
    assert!(out.contains("no step data"), "{out}");
}

#[test]
fn navigation_walks_visible_rows_not_raw_indices() {
    let mut s = viewer(Some("Run cargo test"));
    // opening the viewer on a step puts the cursor on that step's first row,
    // not on line 0 of the raw log
    clamp_log_selection(&mut s);
    let rows = visible_rows(s.log_viewer.as_ref().unwrap());
    assert_eq!(s.log_viewer.as_ref().unwrap().selected_line, rows[0].0);

    for _ in 0..3 {
        move_log_selection(&mut s, 1);
    }
    assert_eq!(s.log_viewer.as_ref().unwrap().selected_line, rows[3].0);

    // clamp at the end instead of running off into hidden rows
    for _ in 0..50 {
        move_log_selection(&mut s, 1);
    }
    assert_eq!(
        s.log_viewer.as_ref().unwrap().selected_line,
        rows.last().unwrap().0
    );
    move_log_selection(&mut s, -1);
    assert_eq!(s.log_viewer.as_ref().unwrap().selected_line, rows[4].0);
}

// ── folding ──────────────────────────────────────────────────────────────────

#[test]
fn folding_a_group_collapses_its_body() {
    let mut s = viewer(None);
    let log = s.log_viewer.as_ref().unwrap().log.clone().unwrap();
    let start = group_start_at(&log.lines, 10).unwrap();

    s.log_viewer.as_mut().unwrap().folded_groups.insert(start);
    let rows = visible_rows(s.log_viewer.as_ref().unwrap());
    let folded = *rows
        .iter()
        .find(|r| r.0 == start)
        .expect("header row still visible");
    assert_eq!(folded.1, 4, "the body reports how much it hides");

    let out = frame(&s);
    assert!(out.contains("lines"), "{}", out);
    assert!(!out.contains("Compiling runline"), "{}", out);
}
