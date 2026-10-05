use super::*;
use serde_json::Value;

pub fn parse_run_status(s: &str) -> RunStatus {
    match s.to_lowercase().as_str() {
        "queued" => RunStatus::Queued,
        "in_progress" | "in-progress" | "running" => RunStatus::InProgress,
        "completed" => RunStatus::Completed,
        "requested" => RunStatus::Requested,
        "waiting" => RunStatus::Waiting,
        "pending" => RunStatus::Pending,
        other => RunStatus::Unknown(other.to_string()),
    }
}

pub fn parse_conclusion(s: &str) -> Conclusion {
    match s.to_lowercase().as_str() {
        "success" | "passed" => Conclusion::Success,
        "failure" | "failed" => Conclusion::Failure,
        "neutral" => Conclusion::Neutral,
        "cancelled" => Conclusion::Cancelled,
        "skipped" => Conclusion::Skipped,
        "timed_out" | "timed-out" => Conclusion::TimedOut,
        "action_required" => Conclusion::ActionRequired,
        "stale" => Conclusion::Stale,
        "startup_failure" => Conclusion::StartupFailure,
        "" | "null" | "none" => Conclusion::None,
        other => Conclusion::Unknown(other.to_string()),
    }
}

pub fn parse_run_json(v: &Value) -> Result<Run, String> {
    let database_id = v["databaseId"].as_u64().ok_or("missing databaseId")?;
    let number = v["number"].as_u64().unwrap_or(database_id);
    let name = v["name"].as_str().unwrap_or("").to_string();
    let workflow_name = v["workflowName"]
        .as_str()
        .or_else(|| v["name"].as_str())
        .unwrap_or("workflow")
        .to_string();
    let display_title = v["displayTitle"]
        .as_str()
        .unwrap_or(&workflow_name)
        .to_string();
    let event = v["event"].as_str().unwrap_or("push").to_string();
    let head_branch = v["headBranch"].as_str().unwrap_or("main").to_string();
    let head_sha = v["headSha"].as_str().unwrap_or("").to_string();
    let actor = v["actor"]["login"]
        .as_str()
        .or_else(|| v["actor"].as_str())
        .unwrap_or("unknown")
        .to_string();

    let status_str = v["status"].as_str().unwrap_or("completed");
    let conclusion_str = v["conclusion"].as_str().unwrap_or("");

    let status = parse_run_status(status_str);
    let conclusion = parse_conclusion(conclusion_str);
    let health = Health::from_status_conclusion(&status, &conclusion);

    let created_at = v["createdAt"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    let started_at = v["startedAt"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let updated_at = v["updatedAt"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(created_at);

    let url = v["url"].as_str().unwrap_or("").to_string();
    let attempt = v["attempt"].as_u64().unwrap_or(1) as u32;

    Ok(Run {
        database_id,
        number,
        name,
        workflow_name,
        display_title,
        event,
        head_branch,
        head_sha,
        actor,
        status,
        conclusion,
        health,
        created_at,
        started_at,
        updated_at,
        url,
        attempt,
    })
}

pub fn parse_runs_json(v: &Value) -> Result<Vec<Run>, String> {
    let arr = v.as_array().ok_or("expected array")?;
    let mut runs = Vec::new();
    for item in arr {
        if let Ok(run) = parse_run_json(item) {
            runs.push(run);
        }
    }
    Ok(runs)
}

pub fn parse_job_json(v: &Value) -> Result<Job, String> {
    let database_id = v["databaseId"].as_u64().ok_or("missing databaseId")?;
    let name = v["name"].as_str().unwrap_or("job").to_string();
    let status_str = v["status"].as_str().unwrap_or("completed");
    let conclusion_str = v["conclusion"].as_str().unwrap_or("");

    let status = parse_run_status(status_str);
    let conclusion = parse_conclusion(conclusion_str);

    let started_at = v["startedAt"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let completed_at = v["completedAt"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let runner_name = v["runnerName"].as_str().map(|s| s.to_string());

    let mut steps = Vec::new();
    if let Some(steps_arr) = v["steps"].as_array() {
        for (idx, step_v) in steps_arr.iter().enumerate() {
            let step_number = (idx + 1) as u32;
            let step_name = step_v["name"].as_str().unwrap_or("step").to_string();
            let s_status = parse_run_status(step_v["status"].as_str().unwrap_or("completed"));
            let s_conc = parse_conclusion(step_v["conclusion"].as_str().unwrap_or(""));
            let s_started = step_v["startedAt"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));
            let s_completed = step_v["completedAt"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            steps.push(Step {
                number: step_number,
                name: step_name,
                status: s_status,
                conclusion: s_conc,
                started_at: s_started,
                completed_at: s_completed,
            });
        }
    }

    Ok(Job {
        database_id,
        name,
        status,
        conclusion,
        started_at,
        completed_at,
        runner_name,
        steps,
    })
}

pub fn parse_run_detail_json(v: &Value) -> Result<RunDetail, String> {
    let run = parse_run_json(v)?;
    let mut jobs = Vec::new();
    if let Some(jobs_arr) = v["jobs"].as_array() {
        for j_v in jobs_arr {
            if let Ok(job) = parse_job_json(j_v) {
                jobs.push(job);
            }
        }
    }
    Ok(RunDetail { run, jobs })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enum_parsing() {
        assert_eq!(parse_run_status("in_progress"), RunStatus::InProgress);
        assert_eq!(
            parse_run_status("something_new"),
            RunStatus::Unknown("something_new".to_string())
        );

        assert_eq!(parse_conclusion("success"), Conclusion::Success);
        assert_eq!(parse_conclusion("failure"), Conclusion::Failure);
    }
}
