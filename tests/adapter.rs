use runline::gh::LogLevel;
use runline::gh::logs::parse_log_output;
use runline::gh::parse;

#[test]
fn fixtures_parse() {
    let txt = std::fs::read_to_string("tests/fixtures/runs.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    let runs = parse::parse_runs_json(&v).unwrap();
    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0].health, runline::gh::Health::Failed);
    let d = std::fs::read_to_string("tests/fixtures/run_detail.json").unwrap();
    let dv: serde_json::Value = serde_json::from_str(&d).unwrap();
    let det = parse::parse_run_detail_json(&dv).unwrap();
    assert_eq!(det.jobs.len(), 2);
}

#[test]
fn job_log_fixture_parses_like_the_real_thing() {
    let raw = std::fs::read_to_string("tests/fixtures/job.log").unwrap();
    let log = parse_log_output(1, &raw);

    assert_eq!(log.lines.len(), 39, "every line survives parsing");
    assert!(
        log.lines.iter().all(|l| l.timestamp.is_some()),
        "every line is stamped"
    );

    // the step column is what the step-scoped view filters on
    let steps: std::collections::BTreeSet<_> = log
        .lines
        .iter()
        .filter_map(|l| l.step_name.clone())
        .collect();
    assert_eq!(
        steps.into_iter().collect::<Vec<_>>(),
        vec![
            "Complete job",
            "Run actions/checkout@v4",
            "Run cargo test",
            "Set up job",
        ]
    );

    // two `##[error]` markers plus one `error:` tool line; the `error: ` prefix
    // is part of cargo's message, not a marker, so it stays in the text
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
            "error: test failed, to rerun pass `-p runline --lib parse::tests::test_log_parser`",
            "Process completed with exit code 101.",
        ]
    );

    // `dir::tests::errored` must not be mistaken for an annotation
    assert!(
        !log.lines
            .iter()
            .any(|l| l.level == LogLevel::Error && l.message.contains("errored"))
    );

    // indentation and tabs survive
    assert_eq!(
        log.lines
            .iter()
            .find(|l| l.message.contains("left:"))
            .map(|l| l.message.as_str()),
        Some("  left: \"Failed\"")
    );
    assert!(
        log.lines
            .iter()
            .any(|l| l.message == "hint: \tgit config --global init.defaultBranch <name>")
    );

    // groups are balanced
    assert_eq!(log.lines.iter().filter(|l| l.is_group_start).count(), 6);
    assert_eq!(
        log.lines.iter().filter(|l| l.is_group_end).count(),
        log.lines.iter().filter(|l| l.is_group_start).count()
    );
}
