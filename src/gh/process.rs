use super::*;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

pub struct ProcessBackend {
    pub min_version: GhVersion,
}

impl ProcessBackend {
    pub fn new() -> Self {
        Self {
            min_version: GhVersion {
                major: 2,
                minor: 40,
                patch: 0,
            },
        }
    }

    async fn exec(&self, args: &[&str], timeout_secs: u64) -> Result<String, GhError> {
        let mut cmd = Command::new("gh");
        cmd.args(args);
        cmd.env("GH_PROMPT_DISABLED", "1");
        cmd.env("NO_COLOR", "1");
        cmd.env("GH_NO_UPDATE_NOTIFIER", "1");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().map_err(|_| GhError::NotFound)?;

        let res = timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await;

        match res {
            Ok(Ok(output)) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    let err_str = String::from_utf8_lossy(&output.stderr);
                    Err(GhError::classify(output.status.code(), &err_str))
                }
            }
            Ok(Err(_)) => Err(GhError::NotFound),
            Err(_) => Err(GhError::Timeout),
        }
    }
}

#[async_trait]
impl GhBackend for ProcessBackend {
    async fn version(&self) -> Result<GhVersion, GhError> {
        let out = self.exec(&["--version"], 5).await?;
        GhVersion::parse(&out).ok_or_else(|| GhError::Other("cannot parse gh version".to_string()))
    }

    async fn current_repo(&self) -> Result<RepoRef, GhError> {
        let out = self
            .exec(&["repo", "view", "--json", "nameWithOwner"], 10)
            .await?;
        let v: serde_json::Value = serde_json::from_str(&out)
            .map_err(|e| GhError::Other(format!("invalid json: {}", e)))?;
        let nwo = v["nameWithOwner"]
            .as_str()
            .ok_or_else(|| GhError::NotARepo)?;
        let parts: Vec<&str> = nwo.split('/').collect();
        if parts.len() == 2 {
            Ok(RepoRef::new(parts[0], parts[1]))
        } else {
            Err(GhError::NotARepo)
        }
    }

    async fn list_runs(&self, repo: &RepoRef, q: RunQuery) -> Result<Vec<Run>, GhError> {
        let repo_arg = repo.name_with_owner();
        let limit_str = q.limit.to_string();
        let mut args = vec![
            "run",
            "list",
            "-R",
            &repo_arg,
            "--limit",
            &limit_str,
            "--json",
            "attempt,conclusion,createdAt,databaseId,displayTitle,event,headBranch,headSha,name,number,startedAt,status,updatedAt,url,workflowName",
        ];

        let branch = q.branch.clone();
        if let Some(ref b) = branch {
            args.push("--branch");
            args.push(b.as_str());
        }

        let out = self.exec(&args, 20).await?;
        let v: serde_json::Value = serde_json::from_str(&out)
            .map_err(|e| GhError::Other(format!("invalid json: {}", e)))?;

        parse::parse_runs_json(&v).map_err(GhError::Other)
    }

    async fn view_run(
        &self,
        repo: &RepoRef,
        id: RunId,
        attempt: Option<u32>,
    ) -> Result<RunDetail, GhError> {
        let repo_arg = repo.name_with_owner();
        let id_str = id.to_string();
        let mut args = vec![
            "run",
            "view",
            &id_str,
            "-R",
            &repo_arg,
            "--json",
            "attempt,conclusion,createdAt,databaseId,displayTitle,event,headBranch,headSha,jobs,name,number,startedAt,status,updatedAt,url,workflowName",
        ];

        let att_str = attempt.map(|a| a.to_string());
        if let Some(ref a) = att_str {
            args.push("--attempt");
            args.push(a.as_str());
        }

        let out = self.exec(&args, 20).await?;
        let v: serde_json::Value = serde_json::from_str(&out)
            .map_err(|e| GhError::Other(format!("invalid json: {}", e)))?;

        parse::parse_run_detail_json(&v).map_err(GhError::Other)
    }

    async fn job_log(
        &self,
        repo: &RepoRef,
        job: JobId,
        failed_only: bool,
    ) -> Result<RawLog, GhError> {
        let repo_arg = repo.name_with_owner();
        let jid_str = job.to_string();
        let mut args = vec!["run", "view", "--job", &jid_str, "-R", &repo_arg];
        if failed_only {
            args.push("--log-failed");
        } else {
            args.push("--log");
        }

        let out = self.exec(&args, 60).await?;
        Ok(logs::parse_log_output(job, &out))
    }

    async fn rerun_failed(&self, repo: &RepoRef, id: RunId) -> Result<(), GhError> {
        let repo_arg = repo.name_with_owner();
        let id_str = id.to_string();
        self.exec(&["run", "rerun", &id_str, "--failed", "-R", &repo_arg], 20)
            .await?;
        Ok(())
    }

    async fn cancel(&self, repo: &RepoRef, id: RunId) -> Result<(), GhError> {
        let repo_arg = repo.name_with_owner();
        let id_str = id.to_string();
        self.exec(&["run", "cancel", &id_str, "-R", &repo_arg], 20)
            .await?;
        Ok(())
    }

    async fn rate_limit(&self) -> Result<RateLimit, GhError> {
        let out = self.exec(&["api", "rate_limit"], 10).await?;
        let v: serde_json::Value = serde_json::from_str(&out)
            .map_err(|e| GhError::Other(format!("invalid rate_limit json: {}", e)))?;

        let core = &v["resources"]["core"];
        let used = core["used"].as_u64().unwrap_or(0) as u32;
        let limit = core["limit"].as_u64().unwrap_or(5000) as u32;
        let reset_ts = core["reset"].as_i64().unwrap_or(0);

        let reset_at = DateTime::from_timestamp(reset_ts, 0).unwrap_or_else(Utc::now);

        Ok(RateLimit {
            used,
            limit,
            reset_at,
        })
    }
}
