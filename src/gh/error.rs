use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum GhError {
    #[error(
        "gh was not found on your path. install it from cli.github.com, then run runline again."
    )]
    NotFound,

    #[error(
        "runline needs gh 2.40.0 or newer. you have {0}. upgrade with your package manager or see cli.github.com."
    )]
    TooOld(String),

    #[error("gh is not signed in. run: gh auth login")]
    NotAuthenticated,

    #[error("your token is missing the {0} scope. run: gh auth refresh -s {0}")]
    MissingScope(String),

    #[error(
        "gh run view does not work with fine-grained tokens. sign in with a classic token or use gh auth login."
    )]
    FineGrainedToken,

    #[error(
        "this folder is not a github repository. run runline inside one, or add repos to your config."
    )]
    NotARepo,

    #[error("cannot access {0}. check the name and your permissions.")]
    RepoNotFound(String),

    #[error(
        "github rate limit reached. resets at {0}. runline is waiting and will resume on its own."
    )]
    RateLimited(String),

    #[error("cannot reach github. showing saved data. retrying in {0}s.")]
    NetworkFailure(u64),

    #[error("gh took too long to respond. retrying.")]
    Timeout,

    #[error("this repo has no workflows. add one under .github/workflows to see runs here.")]
    NoWorkflows,

    #[error("no runs match your filters. press esc to clear them.")]
    NoRunsMatch,

    #[error("action failed: {0}. press esc to close.")]
    ActionFailed(String),

    #[error("{0}")]
    Other(String),
}

impl GhError {
    pub fn classify(_code: Option<i32>, stderr: &str) -> Self {
        let s = stderr.to_lowercase();

        if s.contains("command not found") || s.contains("no such file or directory") {
            return GhError::NotFound;
        }
        if s.contains("authentication failed")
            || s.contains("not logged in")
            || s.contains("gh auth login")
        {
            return GhError::NotAuthenticated;
        }
        if s.contains("fine-grained")
            || s.contains("checks:read")
            || s.contains("resource not accessible by integration")
        {
            return GhError::FineGrainedToken;
        }
        if s.contains("must be run from a git repository") || s.contains("not a git repository") {
            return GhError::NotARepo;
        }
        if s.contains("could not resolve host")
            || s.contains("connection refused")
            || s.contains("network is unreachable")
        {
            return GhError::NetworkFailure(5);
        }
        if s.contains("rate limit exceeded") || s.contains("api rate limit") {
            return GhError::RateLimited("soon".to_string());
        }
        if s.contains("could not find") || s.contains("404") {
            return GhError::RepoNotFound("repository".to_string());
        }

        GhError::Other(stderr.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let err = GhError::classify(Some(1), "not logged in to any github hosts");
        assert_eq!(err, GhError::NotAuthenticated);

        let err = GhError::classify(Some(1), "fatal: not a git repository");
        assert_eq!(err, GhError::NotARepo);

        let err = GhError::classify(
            Some(1),
            "fine-grained personal access token does not have checks:read",
        );
        assert_eq!(err, GhError::FineGrainedToken);
    }
}
