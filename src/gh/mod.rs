pub mod error;
pub mod fixture;
pub mod logs;
pub mod parse;
pub mod process;
pub mod version;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use error::GhError;
pub use version::GhVersion;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoRef {
    pub owner: String,
    pub repo: String,
}

impl RepoRef {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
        }
    }

    /// Parse a repository from whatever shape the user throws at it:
    ///
    /// * `owner/repo` — the slug;
    /// * `https://github.com/owner/repo` (trailing slash and `.git` tolerated);
    /// * `git@github.com:owner/repo.git` — the SSH remote form;
    /// * `gh:owner/repo` — the `gh` CLI shorthand.
    ///
    /// Anything else is an error naming what was expected, so `--repo oops`
    /// tells the user what to type instead of silently doing nothing.
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim().trim_end_matches('/');

        // gh CLI shorthand
        let slug = input.strip_prefix("gh:").unwrap_or(input);

        // SSH remote: git@github.com:owner/repo(.git)?
        if let Some(path) = slug.strip_prefix("git@github.com:") {
            return Self::from_slug(path.trim_end_matches(".git"));
        }

        // full URL: scheme://host/owner/repo(.git)?
        if let Some(after_scheme) = slug.split_once("://").map(|(_, rest)| rest) {
            let path = after_scheme.split_once('/').map(|(_, p)| p).unwrap_or("");
            // reject bare hosts and single-segment paths early with a useful error
            if path.is_empty() || !path.contains('/') {
                return Err(format!(
                    "cannot find owner/repo in url '{input}' — expected \
                     https://github.com/<owner>/<repo>"
                ));
            }
            return Self::from_slug(path.trim_end_matches(".git"));
        }

        Self::from_slug(slug)
    }

    fn from_slug(slug: &str) -> Result<Self, String> {
        match slug.split_once('/') {
            Some((owner, repo))
                if !owner.is_empty()
                    && !repo.is_empty()
                    && !repo.contains('/')
                    && !repo.contains(' ') =>
            {
                Ok(Self::new(owner, repo.trim_end_matches(".git")))
            }
            _ => Err(format!(
                "invalid repository '{slug}' — expected <owner>/<repo>, \
                 a github url, or git@github.com:<owner>/<repo>.git"
            )),
        }
    }

    pub fn name_with_owner(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

pub type RunId = u64;
pub type JobId = u64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Queued,
    InProgress,
    Completed,
    Requested,
    Waiting,
    Pending,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Conclusion {
    Success,
    Failure,
    Neutral,
    Cancelled,
    Skipped,
    TimedOut,
    ActionRequired,
    Stale,
    StartupFailure,
    None,
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Health {
    Failed = 0,
    Running = 1,
    Queued = 2,
    Passed = 3,
    Cancelled = 4,
    Skipped = 5,
    Unknown = 6,
}

impl Health {
    pub fn from_status_conclusion(status: &RunStatus, conclusion: &Conclusion) -> Self {
        match status {
            RunStatus::InProgress => Health::Running,
            RunStatus::Queued | RunStatus::Requested | RunStatus::Waiting | RunStatus::Pending => {
                Health::Queued
            }
            RunStatus::Completed => match conclusion {
                Conclusion::Success | Conclusion::Neutral => Health::Passed,
                Conclusion::Failure | Conclusion::TimedOut | Conclusion::StartupFailure => {
                    Health::Failed
                }
                Conclusion::Cancelled => Health::Cancelled,
                Conclusion::Skipped => Health::Skipped,
                Conclusion::ActionRequired => Health::Failed,
                _ => Health::Unknown,
            },
            RunStatus::Unknown(_) => Health::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub database_id: RunId,
    pub number: u64,
    pub name: String,
    pub workflow_name: String,
    pub display_title: String,
    pub event: String,
    pub head_branch: String,
    pub head_sha: String,
    pub actor: String,
    pub status: RunStatus,
    pub conclusion: Conclusion,
    pub health: Health,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub url: String,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub database_id: JobId,
    pub name: String,
    pub status: RunStatus,
    pub conclusion: Conclusion,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub runner_name: Option<String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub number: u32,
    pub name: String,
    pub status: RunStatus,
    pub conclusion: Conclusion,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDetail {
    pub run: Run,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warning,
    Notice,
    Debug,
    Normal,
}

/// A workflow command the runner emitted — `::error::`, `::warning::`,
/// `::notice::`, `::debug::` (and their `#[...]` log-annotation form).
///
/// These are the only annotations GitHub Actions produces, so they are modelled
/// directly rather than smuggled in as log levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotation {
    pub kind: AnnotationKind,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub step: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationKind {
    Error,
    Warning,
    Notice,
    Debug,
}

impl AnnotationKind {
    /// Stable ordering: worst first, so a summary reads error → notice.
    pub fn rank(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Notice => 2,
            Self::Debug => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub job_id: JobId,
    pub step_name: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    /// Plain text, with every escape sequence removed. This is what search,
    /// annotation matching and copy operate on.
    pub message: String,
    pub level: LogLevel,
    pub is_group_start: bool,
    pub is_group_end: bool,
    /// The workflow command that produced this line, if it was one.
    pub annotation: Option<Annotation>,
    /// Original colour runs, recovered from the ANSI codes in the raw line.
    /// `None` when the line carried no colour, so the common case costs
    /// nothing and rendering falls back to `message` plus the theme.
    pub color_spans: Option<Vec<ColorSpan>>,
}

/// A run of text with a terminal colour recovered from an SGR sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorSpan {
    /// Byte range into `LogLine::message`.
    pub start: usize,
    pub end: usize,
    /// Basic (0-15), 256-palette index, or 24-bit RGB, as ANSI encoded it.
    pub color: AnsiColor,
    pub bold: bool,
    pub dim: bool,
    pub underline: bool,
}

/// The colour models an SGR sequence can express.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnsiColor {
    /// 0-15: black, red, green, …, bright white.
    Basic(u8),
    /// 256-colour palette index.
    Indexed(u8),
    /// 24-bit truecolor.
    Rgb(u8, u8, u8),
    /// Bold/dim/underline with no colour change: keep the surrounding style.
    Inherit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLog {
    pub job_id: JobId,
    pub lines: Vec<LogLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub used: u32,
    pub limit: u32,
    pub reset_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct RunQuery {
    pub limit: usize,
    pub branch: Option<String>,
}

#[async_trait]
pub trait GhBackend: Send + Sync {
    async fn version(&self) -> Result<GhVersion, GhError>;
    async fn current_repo(&self) -> Result<RepoRef, GhError>;
    async fn list_runs(&self, repo: &RepoRef, q: RunQuery) -> Result<Vec<Run>, GhError>;
    async fn view_run(
        &self,
        repo: &RepoRef,
        id: RunId,
        attempt: Option<u32>,
    ) -> Result<RunDetail, GhError>;
    async fn job_log(
        &self,
        repo: &RepoRef,
        job: JobId,
        failed_only: bool,
    ) -> Result<RawLog, GhError>;
    async fn rerun_failed(&self, repo: &RepoRef, id: RunId) -> Result<(), GhError>;
    async fn cancel(&self, repo: &RepoRef, id: RunId) -> Result<(), GhError>;
    async fn rate_limit(&self) -> Result<RateLimit, GhError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_parse_accepts_every_shape() {
        let cases = [
            ("octocat/hello-world", ("octocat", "hello-world")),
            ("  octocat/hello-world  ", ("octocat", "hello-world")),
            (
                "https://github.com/octocat/hello-world",
                ("octocat", "hello-world"),
            ),
            (
                "https://github.com/octocat/hello-world/",
                ("octocat", "hello-world"),
            ),
            (
                "https://github.com/octocat/hello-world.git",
                ("octocat", "hello-world"),
            ),
            (
                "http://github.com/octocat/hello-world",
                ("octocat", "hello-world"),
            ),
            (
                "git@github.com:octocat/hello-world.git",
                ("octocat", "hello-world"),
            ),
            (
                "git@github.com:octocat/hello-world",
                ("octocat", "hello-world"),
            ),
            ("gh:octocat/hello-world", ("octocat", "hello-world")),
        ];
        for (input, (owner, repo)) in cases {
            let r = RepoRef::parse(input).unwrap_or_else(|e| panic!("{input:?}: {e}"));
            assert_eq!((r.owner.as_str(), r.repo.as_str()), (owner, repo));
        }
    }

    #[test]
    fn repo_parse_rejects_garbage_usefully() {
        for bad in [
            "",
            "just-a-name",
            "a/b/c",
            "owner/",
            "/repo",
            "https://github.com",
            "owner/ ",
        ] {
            assert!(RepoRef::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }
}
