# research notes

## 1. ratatui and crossterm
- ratatui version: 0.30.2 (verified against docs.rs and the upstream breaking-changes doc, 2026-10-11). breaking changes from 0.2x include modernized terminal init/restore helpers (`ratatui::init()` and `ratatui::restore()`), widget refactoring, and buffer management improvements.
- crossterm version: 0.28.0+.
- async event loop structure: tokio runtime with a dedicated input event listener task using `crossterm::event::event_stream()` or `crossterm::event::read()`, combined with timer ticks and backend task response channels into a single tokio `tokio::sync::mpsc` channel.
- panic hook: `ratatui::restore()` should be called inside a custom panic hook wrapper before printing or logging panic info to ensure the terminal state (raw mode, alternate screen) is cleanly restored.

## 2. gh cli surface
- `gh` version requirement: minimum version 2.40.0+ (the release that added `attempt` field to `gh run view` json and stable `--json` outputs for actions).
- `gh run view` json fields verified: `attempt, conclusion, createdAt, databaseId, displayTitle, event, headBranch, headSha, jobs, name, number, startedAt, status, updatedAt, url, workflowDatabaseId, workflowName`.
- `gh run view` flags: `--attempt`, `--job`, `--log`, `--log-failed`, `--json`, `--jq`, `--exit-status`, `-R/--repo`.
- fine-grained tokens: fine-grained personal access tokens lack `checks:read` repository permission scope by default in github api, causing `gh run view` to fail. runline detects this error pattern and surfaces message: `your token is missing required scopes or a fine-grained token without checks:read was used. sign in with a classic token or use gh auth login.`
- `gh run list` flags: `-R`, `--limit`, `--json`, `--branch`, `--status`.
- `gh run rerun`: `gh run rerun <id> --failed -R <repo>` and `gh run rerun <id> --job <jid> -R <repo>`.
- `gh run cancel`: `gh run cancel <id> -R <repo>`.
- `gh run view --log` output format: outputs streaming log lines prefixed with timestamps and job/step markers.

## 3. github actions log syntax
- markers: `##[group]<name>`, `##[endgroup]`, `##[error]`, `##[warning]`, `##[notice]`, `##[debug]`.
- annotation command syntax: `::error title=Title::message`, `::warning::message`, `::notice::message`.
- timestamps: github runner prepends iso8601 timestamps (e.g., `2026-03-30T12:34:56.7890123Z `) to raw log lines. runline parses these timestamps and strips or formats them based on user preference.

## 4. rate limits
- actions endpoints consume standard github rest api rate limits (5000 requests/hour for authenticated users).
- quota check: `gh api rate_limit` returns core and actions rate limit quotas and reset times.
- conditional requests: sending `if-none-match: W/"etag"` on `gh api` calls returning `304 not modified` do not count against the primary rate limit quota.
- headers: `gh api --include` includes http response headers including `x-ratelimit-remaining`, `x-ratelimit-reset`, and `etag`.

## 5. status and conclusion enums
- run status values: `queued`, `in_progress`, `completed`, `requested`, `waiting`, `pending`, `unknown(...)`.
- run conclusion values: `success`, `failure`, `neutral`, `cancelled`, `skipped`, `timed_out`, `action_required`, `stale`, `startup_failure`, `null` (when in progress), `unknown(...)`.
- job status values: `queued`, `in_progress`, `completed`, `waiting`, `pending`, `unknown(...)`.
- step status/conclusion values: similar mapping with exhaustive `unknown(String)` fallback.

## 6. terminal capabilities
- osc queries: osc 10 queries default foreground color, osc 11 queries default background color, osc 4 queries palette slots 0-15. implemented with a 150 ms timeout using crossterm/stdout write and stdin/event read.
- truecolor detection: check `COLORTERM=truecolor` or `COLORTERM=24bit`.
- graphics protocols: kitty graphics protocol detection via query sequence `\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\` or standard term queries. ratatui-image / kitty protocol support handled optionally behind `graphics` feature flag.

## 7. comparable tools
- `gh-dash`: dashboard for pull requests and issues across many repos. covers pr review flow rather than live run streaming, step trees, and log error jumps.
- `ghr`: lightweight github actions runner/trigger tool or cli utility. focuses on triggering workflows rather than interactive terminal dashboard monitoring and log inspection.
- `wrkflw`: terminal ui for github actions. different scope; runline is a keyboard-first client with low latency, palette derived themes, 50k line logs, and a fixed v1 scope.

## 8. crate versions and justification
- `tokio` (1.40): robust async runtime for event loop and process management.
- `ratatui` (0.30.2): terminal ui library. 0.30 notes applied: `block::title` removed (titles take `into<line>`), `layout::alignment` renamed to `horizontalalignment` (alias kept), `testbackend` error is now `infallible`, `backend` requires associated `error` and `clear_region`, msrv 1.88. none of these touch runline: no custom backend, no `block::title`, no `alignment` use. edition 2024, rust 1.98. `crossterm` 0.29 to match ratatui, `toml` 1.0, `thiserror` 2.0, `unicode-width` 0.2.2.
- `crossterm` (0.28): cross-platform terminal control and event handling.
- `serde` / `serde_json` (1.0): zero-copy json serialization and deserialization for gh outputs.
- `async-trait` (0.1): async trait definitions for `GhBackend`.
- `thiserror` (1.0): ergonomic typed error definitions.
- `chrono` (0.4): robust datetime parsing for run durations and timestamps.
- `insta` (1.40): snapshot testing for ui regression prevention.
- `unicode-width` (0.2): correct string width calculations for terminal cell layout.
