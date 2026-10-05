use super::*;
use async_trait::async_trait;

pub struct FixtureBackend {
    pub repo: RepoRef,
    pub runs: Vec<Run>,
    pub run_details: std::collections::HashMap<RunId, RunDetail>,
    pub logs: std::collections::HashMap<JobId, RawLog>,
    pub rate_limit: RateLimit,
}

impl FixtureBackend {
    pub fn demo() -> Self {
        let repo = RepoRef::new("octocat", "runline");
        let now = Utc::now();

        let run_failed = Run {
            database_id: 1001,
            number: 101,
            name: "ci".to_string(),
            workflow_name: "ci".to_string(),
            display_title: "feat/parser: add log parser tests".to_string(),
            event: "push".to_string(),
            head_branch: "feat/parser".to_string(),
            head_sha: "a1b2c3d".to_string(),
            actor: "mira".to_string(),
            status: RunStatus::Completed,
            conclusion: Conclusion::Failure,
            health: Health::Failed,
            created_at: now - chrono::Duration::minutes(2),
            started_at: Some(now - chrono::Duration::minutes(2)),
            updated_at: now,
            url: "https://github.com/octocat/runline/actions/runs/1001".to_string(),
            attempt: 1,
        };

        let run_running = Run {
            database_id: 1002,
            number: 102,
            name: "release".to_string(),
            workflow_name: "release".to_string(),
            display_title: "release: v1.0.0".to_string(),
            event: "push".to_string(),
            head_branch: "main".to_string(),
            head_sha: "e5f6g7h".to_string(),
            actor: "jon".to_string(),
            status: RunStatus::InProgress,
            conclusion: Conclusion::None,
            health: Health::Running,
            created_at: now - chrono::Duration::seconds(14),
            started_at: Some(now - chrono::Duration::seconds(14)),
            updated_at: now,
            url: "https://github.com/octocat/runline/actions/runs/1002".to_string(),
            attempt: 1,
        };

        let run_passed = Run {
            database_id: 1003,
            number: 100,
            name: "ci".to_string(),
            workflow_name: "ci".to_string(),
            display_title: "fix: resolve theme contrast issue".to_string(),
            event: "push".to_string(),
            head_branch: "main".to_string(),
            head_sha: "9i8h7g6".to_string(),
            actor: "jon".to_string(),
            status: RunStatus::Completed,
            conclusion: Conclusion::Success,
            health: Health::Passed,
            created_at: now - chrono::Duration::hours(1),
            started_at: Some(now - chrono::Duration::hours(1)),
            updated_at: now - chrono::Duration::hours(1),
            url: "https://github.com/octocat/runline/actions/runs/1003".to_string(),
            attempt: 1,
        };

        let job_failed = Job {
            database_id: 5001,
            name: "test (ubuntu-latest)".to_string(),
            status: RunStatus::Completed,
            conclusion: Conclusion::Failure,
            started_at: Some(now - chrono::Duration::minutes(2)),
            completed_at: Some(now),
            runner_name: Some("GitHub Actions 2".to_string()),
            steps: vec![
                Step {
                    number: 1,
                    name: "set up rust".to_string(),
                    status: RunStatus::Completed,
                    conclusion: Conclusion::Success,
                    started_at: Some(now - chrono::Duration::minutes(2)),
                    completed_at: Some(
                        now - chrono::Duration::minutes(2) + chrono::Duration::seconds(10),
                    ),
                },
                Step {
                    number: 2,
                    name: "cargo test".to_string(),
                    status: RunStatus::Completed,
                    conclusion: Conclusion::Failure,
                    started_at: Some(
                        now - chrono::Duration::minutes(2) + chrono::Duration::seconds(10),
                    ),
                    completed_at: Some(now),
                },
            ],
        };

        let detail_failed = RunDetail {
            run: run_failed.clone(),
            jobs: vec![job_failed],
        };

        // Shaped exactly like `gh run view --job <id> --log`: two tab-separated
        // columns gh adds, then verbatim runner output whose first token is
        // the timestamp.
        let log_failed_text = "\
test (ubuntu-latest)\tUNKNOWN STEP\t\u{feff}2026-03-30T11:59:58.4825818Z Current runner version: '2.337.0'\n\
test (ubuntu-latest)\tSet up job\t2026-03-30T11:59:58.5000000Z ##[group]Operating System\n\
test (ubuntu-latest)\tSet up job\t2026-03-30T11:59:58.5001000Z Ubuntu 24.04.5 LTS\n\
test (ubuntu-latest)\tSet up job\t2026-03-30T11:59:58.5002000Z ##[endgroup]\n\
test (ubuntu-latest)\tset up rust\t2026-03-30T12:00:00.0000000Z ##[group]Run dtolnay/rust-toolchain@stable\n\
test (ubuntu-latest)\tset up rust\t2026-03-30T12:00:00.0001000Z [command]/usr/bin/rustc --version\n\
test (ubuntu-latest)\tset up rust\t2026-03-30T12:00:00.0002000Z rustc 1.89.0 (2948383e4 2025-08-04)\n\
test (ubuntu-latest)\tset up rust\t2026-03-30T12:00:00.0003000Z ##[endgroup]\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:00.0000000Z ##[group]Run cargo test\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:01.0000000Z running 12 tests\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:02.0000000Z test copy::tests::test_all_copy_strings_are_lowercase ... ok\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:03.0000000Z test parse::tests::test_log_parser ... FAILED\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:04.0000000Z ##[error]test parse::tests::test_log_parser ... FAILED\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:04.0001000Z ##[group]parse::tests::test_log_parser stdout\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:04.0002000Z assertion `left == right` failed\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:04.0003000Z   left: \"Failed\"\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:04.0004000Z  right: \"failed\"\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:05.0000000Z ##[endgroup]\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:05.1000000Z ::error file=src/gh/logs.rs,line=214,col=9::assertion `left == right` failed\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:05.2000000Z ::error file=src/gh/logs.rs,line=221,col=45::expected `failed`, found `Failed`\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:05.3000000Z ::warning file=src/gh/parse.rs,line=88,col=13::`parse_line` ignores the trailing column when the row is truncated\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:05.4000000Z ::warning file=src/ui/log.rs,line=347::wrap path measured in characters, not display columns\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:05.5000000Z ::notice::3 warnings emitted, 0 duplicate\n\
test (ubuntu-latest)\tcargo test\t2026-03-30T12:00:05.0001000Z ##[endgroup]\n\
test (ubuntu-latest)\tComplete job\t2026-03-30T12:00:06.0000000Z ##[error]Process completed with exit code 101.\n";

        let log_failed = logs::parse_log_output(5001, log_failed_text);

        let mut run_details = std::collections::HashMap::new();
        run_details.insert(1001, detail_failed);

        let mut logs_map = std::collections::HashMap::new();
        logs_map.insert(5001, log_failed);

        Self {
            repo,
            runs: vec![run_failed, run_running, run_passed],
            run_details,
            logs: logs_map,
            rate_limit: RateLimit {
                used: 188,
                limit: 5000,
                reset_at: now + chrono::Duration::hours(1),
            },
        }
    }
}

#[async_trait]
impl GhBackend for FixtureBackend {
    async fn version(&self) -> Result<GhVersion, GhError> {
        Ok(GhVersion {
            major: 2,
            minor: 40,
            patch: 0,
        })
    }

    async fn current_repo(&self) -> Result<RepoRef, GhError> {
        Ok(self.repo.clone())
    }

    async fn list_runs(&self, _repo: &RepoRef, _q: RunQuery) -> Result<Vec<Run>, GhError> {
        Ok(self.runs.clone())
    }

    async fn view_run(
        &self,
        _repo: &RepoRef,
        id: RunId,
        _attempt: Option<u32>,
    ) -> Result<RunDetail, GhError> {
        self.run_details
            .get(&id)
            .cloned()
            .ok_or_else(|| GhError::RepoNotFound(format!("run {}", id)))
    }

    async fn job_log(
        &self,
        _repo: &RepoRef,
        job: JobId,
        _failed_only: bool,
    ) -> Result<RawLog, GhError> {
        self.logs
            .get(&job)
            .cloned()
            .ok_or_else(|| GhError::Other("log not found".to_string()))
    }

    async fn rerun_failed(&self, _repo: &RepoRef, _id: RunId) -> Result<(), GhError> {
        Ok(())
    }

    async fn cancel(&self, _repo: &RepoRef, _id: RunId) -> Result<(), GhError> {
        Ok(())
    }

    async fn rate_limit(&self) -> Result<RateLimit, GhError> {
        Ok(self.rate_limit.clone())
    }
}
