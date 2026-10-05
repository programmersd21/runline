use crate::gh::{GhError, JobId, RateLimit, RawLog, RepoRef, Run, RunDetail, RunId};
use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum AppEvent {
    Input(KeyEvent),
    Tick,
    AnimationTick,
    RunsLoaded(RepoRef, Result<Vec<Run>, GhError>),
    RunDetailLoaded(RepoRef, RunId, Result<RunDetail, GhError>),
    LogLoaded(RepoRef, JobId, Result<RawLog, GhError>),
    RateLimitLoaded(Result<RateLimit, GhError>),
    ActionResult(String, Result<(), GhError>),
    RepoDetected(Result<RepoRef, GhError>),
    FocusGained,
    FocusLost,
    QuitRequested,
}

#[derive(Debug, Clone)]
pub enum Effect {
    FetchRuns(RepoRef),
    FetchRunDetail(RepoRef, RunId, Option<u32>),
    FetchJobLog(RepoRef, JobId, bool),
    FetchRateLimit,
    RerunFailed(RepoRef, RunId),
    CancelRun(RepoRef, RunId),
    OpenBrowser(String),
    CopyToClipboard(String),
    /// Write the visible log (as currently filtered/folded) to a file.
    ExportLog {
        job_id: JobId,
        path: std::path::PathBuf,
        text: String,
    },
}
