use crate::anim::AnimState;
use crate::cache::Cache;
use crate::config::Config;
use crate::event::{AppEvent, Effect};
use crate::gh::{
    Annotation, Conclusion, Health, Job, JobId, LogLevel, LogLine, RateLimit, RawLog, RepoRef, Run,
    RunDetail, RunId, RunQuery, Step,
};
use crate::keymap::{Action, KeyMap};
use crate::poll::PollScheduler;
use crate::theme::{Theme, builtin::dusk};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Detail(RunId),
    Log(RunId, JobId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogType {
    RerunFailed(RunId, String, String, usize), // run_id, workflow, branch, count
    CancelRun(RunId, String, String),          // run_id, workflow, branch
}

#[derive(Debug, Clone)]
pub struct DialogState {
    pub dialog_type: DialogType,
    pub focus_confirm: bool, // false means focus on cancel (safe default per section 3.4)
    pub is_running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogViewerState {
    pub job_id: JobId,
    pub step_filter: Option<String>,
    pub log: Option<RawLog>,
    pub load_error: Option<String>,
    pub selected_line: usize,
    pub search_query: String,
    pub search_active: bool,
    pub search_matches: Vec<usize>,
    pub current_match: usize,
    pub folded_groups: std::collections::HashSet<usize>,
    pub show_timestamps: bool,
    pub wrap_lines: bool,
}

pub struct AppState {
    pub config: Config,
    pub keymap: KeyMap,
    pub theme: Theme,
    pub anim: AnimState,
    pub cache: Cache,
    pub poll: PollScheduler,
    pub repos: Vec<RepoRef>,
    pub current_repo_idx: usize,
    pub runs: HashMap<String, Vec<Run>>,
    pub selected_run_idx: usize,
    pub filter_text: String,
    pub filter_active: bool,
    pub branch_scope_local: bool,
    pub current_branch: Option<String>,
    pub rate_limit: Option<RateLimit>,
    pub screen_stack: Vec<Screen>,
    pub run_details: HashMap<RunId, RunDetail>,
    pub selected_node_idx: usize, // flat tree row, see flat_nodes
    pub detail_attempt: HashMap<RunId, u32>,
    pub log_viewer: Option<LogViewerState>,
    /// Filename of the most recent log export, shown briefly in the footer.
    pub last_export: Option<String>,
    /// Annotations (errors / warnings / notices / debug) parsed out of job
    /// logs, keyed by job so the run view can show every annotation a run
    /// produced without re-reading logs.
    pub annotations: HashMap<JobId, Vec<Annotation>>,
    /// Annotation feed expanded past its pane ("N more" pressed).
    pub annotations_expanded: bool,
    /// Scroll offset for the expanded feed.
    pub annotation_scroll: u16,
    pub dialog: Option<DialogState>,
    pub show_help: bool,
    pub show_theme_picker: bool,
    pub theme_picker_idx: usize,
    pub should_quit: bool,
    pub error_banner: Option<String>,
    /// true while a ForceRefresh fetch is in-flight — triggers braille overlay
    pub is_loading: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let reduce_motion = config.ui.reduce_motion;
        let theme = dusk();
        let keymap = KeyMap::default();
        let anim = AnimState::new(reduce_motion);
        let cache = Cache::new(config.cache.enabled);
        let poll = PollScheduler::new(3, 60);

        Self {
            config,
            keymap,
            theme,
            anim,
            cache,
            poll,
            repos: Vec::new(),
            current_repo_idx: 0,
            runs: HashMap::new(),
            selected_run_idx: 0,
            filter_text: String::new(),
            filter_active: false,
            branch_scope_local: false,
            current_branch: None,
            rate_limit: None,
            screen_stack: vec![Screen::Dashboard],
            run_details: HashMap::new(),
            selected_node_idx: 0,
            detail_attempt: HashMap::new(),
            log_viewer: None,
            last_export: None,
            annotations: HashMap::new(),
            annotations_expanded: false,
            annotation_scroll: 0,
            dialog: None,
            show_help: false,
            show_theme_picker: false,
            theme_picker_idx: 0,
            is_loading: false,
            should_quit: false,
            error_banner: None,
        }
    }

    pub fn current_repo(&self) -> Option<&RepoRef> {
        self.repos.get(self.current_repo_idx)
    }

    pub fn current_screen(&self) -> Screen {
        self.screen_stack
            .last()
            .cloned()
            .unwrap_or(Screen::Dashboard)
    }

    /// The run the log viewer (or detail view) belongs to, if any.
    pub fn detail_run_id(&self) -> Option<RunId> {
        match self.current_screen() {
            Screen::Detail(id) | Screen::Log(id, _) => Some(id),
            _ => None,
        }
    }

    pub fn filtered_runs(&self) -> Vec<Run> {
        let repo = match self.current_repo() {
            Some(r) => r.name_with_owner(),
            None => return Vec::new(),
        };

        let raw_runs = match self.runs.get(&repo) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut list: Vec<Run> = raw_runs
            .iter()
            .filter(|r| {
                if self.branch_scope_local {
                    if let Some(ref b) = self.current_branch {
                        if &r.head_branch != b {
                            return false;
                        }
                    }
                }

                if !self.filter_text.is_empty() {
                    let q = self.filter_text.to_lowercase();
                    let match_wf = r.workflow_name.to_lowercase().contains(&q);
                    let match_branch = r.head_branch.to_lowercase().contains(&q);
                    let match_actor = r.actor.to_lowercase().contains(&q);
                    let match_title = r.display_title.to_lowercase().contains(&q);
                    if !match_wf && !match_branch && !match_actor && !match_title {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort: failed/timed_out top (0), in-progress (1), then rest. Within group, newest first.
        list.sort_by(|a, b| {
            let rank_a = match a.health {
                Health::Failed => 0,
                Health::Running => 1,
                Health::Queued => 2,
                _ => 3,
            };
            let rank_b = match b.health {
                Health::Failed => 0,
                Health::Running => 1,
                Health::Queued => 2,
                _ => 3,
            };

            if rank_a != rank_b {
                rank_a.cmp(&rank_b)
            } else {
                b.created_at.cmp(&a.created_at)
            }
        });

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatNode {
    Job(usize),
    Step(usize, usize),
}

/// flattened tree rows: each job followed by its steps, so j/k walks
/// steps as well as jobs. steps render under their job in the ui.
pub fn flat_nodes(detail: &RunDetail) -> Vec<FlatNode> {
    let mut out = Vec::new();
    for (ji, job) in detail.jobs.iter().enumerate() {
        out.push(FlatNode::Job(ji));
        for (si, _) in job.steps.iter().enumerate() {
            out.push(FlatNode::Step(ji, si));
        }
    }
    out
}

pub fn node_job_idx(node: FlatNode) -> usize {
    match node {
        FlatNode::Job(ji) | FlatNode::Step(ji, _) => ji,
    }
}

/// index of the first failed step, else first running, else first row.
pub fn default_node(detail: &RunDetail) -> usize {
    let nodes = flat_nodes(detail);
    if let Some(i) = nodes.iter().position(|n| match n {
        FlatNode::Step(ji, si) => detail.jobs[*ji].steps[*si].conclusion == Conclusion::Failure,
        FlatNode::Job(ji) => {
            detail.jobs[*ji].steps.is_empty() && detail.jobs[*ji].conclusion == Conclusion::Failure
        }
    }) {
        return i;
    }
    if let Some(i) = nodes.iter().position(|n| match n {
        FlatNode::Step(ji, si) => matches!(
            detail.jobs[*ji].steps[*si].status,
            crate::gh::RunStatus::InProgress
        ),
        FlatNode::Job(ji) => matches!(detail.jobs[*ji].status, crate::gh::RunStatus::InProgress),
    }) {
        return i;
    }
    0
}

/// recompute search matches for the viewer's current query.
pub fn apply_search(lv: &mut LogViewerState) {
    lv.search_matches.clear();
    lv.current_match = 0;
    if lv.search_query.is_empty() {
        return;
    }
    if let Some(ref log) = lv.log {
        let q = lv.search_query.clone();
        let smart_case = q.chars().any(|c| c.is_uppercase());
        for (i, line) in log.lines.iter().enumerate() {
            let hit = if smart_case {
                line.message.contains(&q)
            } else {
                line.message.to_lowercase().contains(&q.to_lowercase())
            };
            if hit {
                lv.search_matches.push(i);
            }
        }
        if let Some(&first) = lv.search_matches.first() {
            lv.selected_line = first;
        }
    }
}

/// index of the innermost group start enclosing line idx, if any.
pub fn group_start_at(lines: &[LogLine], idx: usize) -> Option<usize> {
    let mut stack: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate().take(idx + 1) {
        if line.is_group_start {
            stack.push(i);
        } else if line.is_group_end {
            stack.pop();
        }
    }
    stack.pop()
}

/// A row the log screen can draw: the index into `RawLog::lines`, plus how
/// many rows this one hides (non-zero only for a folded group header).
pub type LogRow = (usize, usize);

/// The rows the log screen is currently showing.
///
/// Two filters apply, in this order: folded group bodies are dropped, and when
/// the viewer was opened on a step, everything outside that step is dropped.
///
/// Navigation, the header counter and the renderer all have to agree on this
/// set — otherwise the cursor walks over rows that are not on screen and the
/// counter counts lines nobody can see.
pub fn visible_rows(lv: &LogViewerState) -> Vec<LogRow> {
    let Some(ref log) = lv.log else {
        return Vec::new();
    };
    let lines = &log.lines;

    // Pair every group start with its matching end. Unbalanced markers leave
    // `None`, which renders as "cannot fold".
    let mut ends: Vec<Option<usize>> = vec![None; lines.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.is_group_start {
            stack.push(i);
        } else if l.is_group_end {
            if let Some(s) = stack.pop() {
                ends[s] = Some(i);
            }
        }
    }

    // A group marker belongs to the scope it was opened in. Letting markers
    // through unconditionally fills a step-scoped view with `▼ title` rows
    // whose bodies were filtered out.
    let in_scope = |l: &LogLine| match lv.step_filter {
        None => true,
        Some(ref s) => l.step_name.as_deref() == Some(s.as_str()),
    };

    let mut out: Vec<LogRow> = Vec::new();
    let mut skip_until: Option<usize> = None;

    for (i, l) in lines.iter().enumerate() {
        if let Some(end) = skip_until {
            if i < end {
                continue;
            }
            skip_until = None;
        }
        if !in_scope(l) {
            continue;
        }

        if lv.folded_groups.contains(&i) {
            if let Some(end) = ends[i] {
                // Count only the rows this view is actually showing, so the
                // "N lines" badge stays true under a step filter.
                let hidden = (i + 1..end).filter(|&j| in_scope(&lines[j])).count();
                out.push((i, hidden));
                skip_until = Some(end);
                continue;
            }
        }

        out.push((i, 0));
    }

    // `gh` labels every line `UNKNOWN STEP` whenever the platform cannot map
    // logs to steps, and then no step-scoped view can ever match. A blank
    // screen is the least useful thing we could render, so fall back to the
    // whole job log; the breadcrumb in the header still says what was asked
    // for.
    if out.is_empty() && lv.step_filter.is_some() {
        return rows_without_step_filter(lines, &lv.folded_groups, &ends);
    }

    out
}

fn rows_without_step_filter(
    lines: &[LogLine],
    folded: &std::collections::HashSet<usize>,
    ends: &[Option<usize>],
) -> Vec<LogRow> {
    let mut out: Vec<LogRow> = Vec::new();
    let mut skip_until: Option<usize> = None;
    for (i, _l) in lines.iter().enumerate() {
        if let Some(end) = skip_until {
            if i < end {
                continue;
            }
            skip_until = None;
        }
        if folded.contains(&i) {
            if let Some(end) = ends[i] {
                out.push((i, end.saturating_sub(i + 1)));
                skip_until = Some(end);
                continue;
            }
        }
        out.push((i, 0));
    }
    out
}

/// Position of the selected line within `rows`, or 0 when it is not shown.
fn row_of(rows: &[LogRow], line: usize) -> usize {
    rows.iter().position(|&(i, _)| i == line).unwrap_or(0)
}

/// Keep the cursor on a row that is actually on screen.
///
/// The row set changes whenever a log arrives, a group is folded, or the view
/// is scoped to a step. Without this the cursor can end up parked on a row that
/// was filtered away, and every `j` press then appears to do nothing.
pub fn clamp_log_selection(state: &mut AppState) {
    let Some(lv) = state.log_viewer.as_ref() else {
        return;
    };
    let rows = visible_rows(lv);
    if rows.is_empty() {
        if let Some(lv) = state.log_viewer.as_mut() {
            lv.selected_line = 0;
        }
        return;
    }
    let target = if rows.iter().any(|&(i, _)| i == lv.selected_line) {
        lv.selected_line
    } else {
        rows[0].0
    };
    if let Some(lv) = state.log_viewer.as_mut() {
        lv.selected_line = target;
    }
}

/// Move the log cursor by `delta` *visible* rows, clamped at both ends.
pub fn move_log_selection(state: &mut AppState, delta: isize) {
    let Some(lv) = state.log_viewer.as_ref() else {
        return;
    };
    let rows = visible_rows(lv);
    if rows.is_empty() {
        return;
    }
    let pos = row_of(&rows, lv.selected_line) as isize;
    let next = (pos + delta).clamp(0, rows.len() as isize - 1) as usize;
    if let Some(lv) = state.log_viewer.as_mut() {
        lv.selected_line = rows[next].0;
    }
}

/// Jump to the first or last visible row.
pub fn log_end(state: &mut AppState, to_end: bool) {
    let Some(lv) = state.log_viewer.as_ref() else {
        return;
    };
    let rows = visible_rows(lv);
    let target = if to_end { rows.last() } else { rows.first() };
    let Some(&target) = target else {
        return;
    };
    if let Some(lv) = state.log_viewer.as_mut() {
        lv.selected_line = target.0;
    }
}

fn jump_error(state: &mut AppState, forward: bool) {
    let Some(lv) = state.log_viewer.as_ref() else {
        return;
    };
    let Some(ref log) = lv.log else {
        return;
    };
    // Only errors the viewer is actually showing, so `e` cannot park the
    // cursor on a filtered-out row.
    let errs: Vec<usize> = visible_rows(lv)
        .into_iter()
        .map(|r| r.0)
        .filter(|&i| log.lines[i].level == LogLevel::Error)
        .collect();
    if errs.is_empty() {
        return;
    }
    let target = if forward {
        errs.iter()
            .find(|&&i| i > lv.selected_line)
            .copied()
            .unwrap_or(errs[0])
    } else {
        errs.iter()
            .rev()
            .find(|&&i| i < lv.selected_line)
            .copied()
            .unwrap_or(*errs.last().unwrap())
    };
    if let Some(lv) = state.log_viewer.as_mut() {
        lv.selected_line = target;
    }
}

/// Every annotation a run produced, gathered across its jobs and ordered worst
/// first so the summary leads with what actually broke.
pub fn run_annotations(state: &AppState, run_id: RunId) -> Vec<&Annotation> {
    let Some(detail) = state.run_details.get(&run_id) else {
        return Vec::new();
    };
    let mut out: Vec<&Annotation> = detail
        .jobs
        .iter()
        .filter_map(|j| state.annotations.get(&j.database_id))
        .flatten()
        .collect();
    out.sort_by_key(|a| a.kind.rank());
    out
}

/// Render the log exactly as the viewer shows it — same step filter, same
/// folds, same timestamps — as plain text for export.
///
/// Folded groups collapse to a single `▸ <title> (N lines hidden)` row, so the
/// file stays honest about what was elided instead of silently dropping it.
pub fn export_log_text(lv: &LogViewerState) -> Option<String> {
    let log = lv.log.as_ref()?;
    let rows = visible_rows(lv);

    let mut out = String::new();
    for (idx, hidden) in &rows {
        let line = &log.lines[*idx];

        if lv.show_timestamps {
            match line.timestamp {
                Some(ts) => out.push_str(&ts.format("%Y-%m-%dT%H:%M:%SZ ").to_string()),
                None => out.push_str("                          "),
            }
        }

        if line.is_group_start && lv.folded_groups.contains(idx) {
            out.push_str("▸ ");
            out.push_str(line.message.trim_start_matches("##[group]").trim());
            out.push_str(&format!(" ({hidden} lines hidden)"));
        } else if line.is_group_end {
            out.push_str("───");
        } else {
            out.push_str(&line.message);
        }
        out.push('\n');
    }
    Some(out)
}

pub fn update(mut state: AppState, event: AppEvent) -> (AppState, Vec<Effect>) {
    let mut effects = Vec::new();

    match event {
        AppEvent::Input(key) => {
            if state.show_theme_picker {
                let themes = crate::theme::THEMES;
                match key.code {
                    crossterm::event::KeyCode::Esc
                    | crossterm::event::KeyCode::Char('q')
                    | crossterm::event::KeyCode::Char('T') => {
                        state.show_theme_picker = false;
                    }
                    crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                        if state.theme_picker_idx > 0 {
                            state.theme_picker_idx -= 1;
                        } else {
                            state.theme_picker_idx = themes.len().saturating_sub(1);
                        }
                        // Live preview
                        let theme_name = themes[state.theme_picker_idx];
                        state.theme = crate::theme::by_name(theme_name);
                    }
                    crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                        if state.theme_picker_idx + 1 < themes.len() {
                            state.theme_picker_idx += 1;
                        } else {
                            state.theme_picker_idx = 0;
                        }
                        // Live preview
                        let theme_name = themes[state.theme_picker_idx];
                        state.theme = crate::theme::by_name(theme_name);
                    }
                    crossterm::event::KeyCode::Enter => {
                        let theme_name = themes[state.theme_picker_idx];
                        state.theme = crate::theme::by_name(theme_name);
                        state.config.ui.theme = theme_name.to_string();
                        let _ = crate::config::save_config(&state.config, None);
                        state.show_theme_picker = false;
                    }
                    _ => {}
                }
                return (state, effects);
            }

            if state.show_help {
                if key.code == crossterm::event::KeyCode::Esc
                    || key.code == crossterm::event::KeyCode::Char('?')
                    || key.code == crossterm::event::KeyCode::Char('q')
                {
                    state.show_help = false;
                }
                return (state, effects);
            }

            let dlgtype = state.dialog.clone().map(|d| d.dialog_type.clone());
            if let Some(ref mut dialog) = state.dialog {
                match key.code {
                    crossterm::event::KeyCode::Esc
                    | crossterm::event::KeyCode::Char('n')
                    | crossterm::event::KeyCode::Char('q') => {
                        state.dialog = None;
                    }
                    crossterm::event::KeyCode::Tab
                    | crossterm::event::KeyCode::Right
                    | crossterm::event::KeyCode::Left => {
                        dialog.focus_confirm = !dialog.focus_confirm;
                    }
                    crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char('y') => {
                        if dialog.focus_confirm || key.code == crossterm::event::KeyCode::Char('y')
                        {
                            dialog.is_running = true;
                            let repo = state.repos.get(state.current_repo_idx).cloned();
                            if let Some(repo) = repo {
                                match &dlgtype.clone().unwrap() {
                                    DialogType::RerunFailed(id, _, _, _) => {
                                        effects.push(Effect::RerunFailed(repo, *id));
                                    }
                                    DialogType::CancelRun(id, _, _) => {
                                        effects.push(Effect::CancelRun(repo, *id));
                                    }
                                }
                            }
                        } else {
                            state.dialog = None;
                        }
                    }
                    _ => {}
                }
                return (state, effects);
            }

            if state.filter_active {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        state.filter_active = false;
                        state.filter_text.clear();
                    }
                    crossterm::event::KeyCode::Enter => {
                        state.filter_active = false;
                    }
                    crossterm::event::KeyCode::Backspace => {
                        state.filter_text.pop();
                    }
                    crossterm::event::KeyCode::Char(c) => {
                        state.filter_text.push(c);
                    }
                    _ => {}
                }
                return (state, effects);
            }

            // log search input mode: typing filters, esc exits
            if matches!(state.current_screen(), Screen::Log(_, _))
                && state
                    .log_viewer
                    .as_ref()
                    .map(|lv| lv.search_active)
                    .unwrap_or(false)
            {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        if let Some(ref mut lv) = state.log_viewer {
                            lv.search_active = false;
                            lv.search_query.clear();
                            lv.search_matches.clear();
                            lv.current_match = 0;
                        }
                    }
                    crossterm::event::KeyCode::Enter => {
                        if let Some(ref mut lv) = state.log_viewer {
                            lv.search_active = false;
                        }
                    }
                    crossterm::event::KeyCode::Backspace => {
                        if let Some(ref mut lv) = state.log_viewer {
                            lv.search_query.pop();
                            apply_search(lv);
                        }
                    }
                    crossterm::event::KeyCode::Char(c) => {
                        if let Some(ref mut lv) = state.log_viewer {
                            lv.search_query.push(c);
                            apply_search(lv);
                        }
                    }
                    _ => {}
                }
                clamp_log_selection(&mut state);
                return (state, effects);
            }

            if let Some(action) = state.keymap.parse_event(key) {
                match action {
                    Action::Quit => {
                        if state.screen_stack.len() > 1 {
                            state.screen_stack.pop();
                        } else {
                            state.should_quit = true;
                        }
                    }
                    Action::ToggleHelp => {
                        state.show_help = !state.show_help;
                    }
                    Action::ThemePicker => {
                        let themes = crate::theme::THEMES;
                        let cur_theme = state.config.ui.theme.as_str();
                        state.theme_picker_idx = themes
                            .iter()
                            .position(|&t| t.eq_ignore_ascii_case(cur_theme))
                            .unwrap_or(0);
                        state.show_theme_picker = true;
                    }
                    Action::ForceRefresh => {
                        state.poll.reset();
                        state.is_loading = true; // show braille overlay until data arrives
                        match state.current_screen() {
                            Screen::Dashboard => {
                                if let Some(repo) = state.current_repo().cloned() {
                                    effects.push(Effect::FetchRuns(repo));
                                }
                            }
                            Screen::Detail(run_id) => {
                                if let Some(repo) = state.current_repo().cloned() {
                                    let attempt =
                                        state.run_details.get(&run_id).map(|d| d.run.attempt);
                                    effects.push(Effect::FetchRunDetail(repo, run_id, attempt));
                                }
                            }
                            Screen::Log(run_id, job_id) => {
                                if let Some(repo) = state.current_repo().cloned() {
                                    let attempt =
                                        state.run_details.get(&run_id).map(|d| d.run.attempt);
                                    effects.push(Effect::FetchRunDetail(
                                        repo.clone(),
                                        run_id,
                                        attempt,
                                    ));
                                    effects.push(Effect::FetchJobLog(repo, job_id, false));
                                }
                            }
                        }
                    }
                    Action::MoveDown => match state.current_screen() {
                        // While the annotation feed is expanded, j/k scroll it
                        // instead of walking the tree — otherwise the feed is
                        // taller than its pane with no way to reach the rest.
                        Screen::Detail(run_id) if state.annotations_expanded => {
                            // Bounded generously here; the ui clamps to the
                            // feed's true overflow when it renders.
                            let _ = run_id;
                            state.annotation_scroll = state.annotation_scroll.saturating_add(1);
                        }
                        Screen::Dashboard => {
                            let len = state.filtered_runs().len();
                            if len > 0 && state.selected_run_idx + 1 < len {
                                state.selected_run_idx += 1;
                            }
                        }
                        Screen::Detail(run_id) => {
                            if let Some(detail) = state.run_details.get(&run_id) {
                                let max = flat_nodes(detail).len().saturating_sub(1);
                                if state.selected_node_idx < max {
                                    state.selected_node_idx += 1;
                                }
                            }
                        }
                        Screen::Log(_, _) => {
                            move_log_selection(&mut state, 1);
                        }
                    },
                    Action::MoveUp => match state.current_screen() {
                        Screen::Detail(run_id) if state.annotations_expanded => {
                            state.annotation_scroll = state.annotation_scroll.saturating_sub(1);
                        }
                        Screen::Dashboard => {
                            if state.selected_run_idx > 0 {
                                state.selected_run_idx -= 1;
                            }
                        }
                        Screen::Detail(_) => {
                            if state.selected_node_idx > 0 {
                                state.selected_node_idx -= 1;
                            }
                        }
                        Screen::Log(_, _) => {
                            move_log_selection(&mut state, -1);
                        }
                    },
                    Action::Filter => {
                        // on the log screen / starts a log search,
                        // everywhere else it filters the run list.
                        if matches!(state.current_screen(), Screen::Log(_, _)) {
                            if let Some(ref mut lv) = state.log_viewer {
                                lv.search_active = true;
                            }
                        } else {
                            state.filter_active = true;
                        }
                    }
                    Action::ToggleBranch => {
                        state.branch_scope_local = !state.branch_scope_local;
                    }
                    Action::NextTab => {
                        if !state.repos.is_empty() {
                            state.current_repo_idx =
                                (state.current_repo_idx + 1) % state.repos.len();
                            if let Some(repo) = state.current_repo().cloned() {
                                effects.push(Effect::FetchRuns(repo));
                            }
                        }
                    }
                    Action::PrevTab => {
                        if !state.repos.is_empty() {
                            if state.current_repo_idx == 0 {
                                state.current_repo_idx = state.repos.len() - 1;
                            } else {
                                state.current_repo_idx -= 1;
                            }
                            if let Some(repo) = state.current_repo().cloned() {
                                effects.push(Effect::FetchRuns(repo));
                            }
                        }
                    }
                    Action::ExportLog => {
                        // Save-what-you-see: the file contains exactly the rows
                        // the viewer is showing — step filter, folds and
                        // timestamps included. Written next to the repo when
                        // run inside one, else the current directory.
                        if !matches!(state.current_screen(), Screen::Log(_, _)) {
                            return (state, effects);
                        }
                        if let Some(lv) = state.log_viewer.as_ref() {
                            if let Some(text) = export_log_text(lv) {
                                let name = match state.current_repo() {
                                    Some(r) => format!(
                                        "{}-{}-job-{}.log",
                                        r.name_with_owner().replace('/', "-"),
                                        state
                                            .detail_run_id()
                                            .map(|id| id.to_string())
                                            .unwrap_or_else(|| "run".to_string()),
                                        lv.job_id
                                    ),
                                    None => format!("runline-job-{}.log", lv.job_id),
                                };
                                let path = std::env::current_dir()
                                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                    .join(&name);
                                effects.push(Effect::ExportLog {
                                    job_id: lv.job_id,
                                    path,
                                    text,
                                });
                                state.last_export = Some(name);
                            }
                        }
                    }
                    Action::ToggleAnnotations => {
                        // Only meaningful on the run view, where the feed
                        // lives. Toggling collapses back to the fitted view
                        // and resets the scroll so the top is reachable again.
                        if matches!(state.current_screen(), Screen::Detail(_)) {
                            state.annotations_expanded = !state.annotations_expanded;
                            state.annotation_scroll = 0;
                        }
                    }
                    Action::Select => match state.current_screen() {
                        Screen::Dashboard => {
                            let runs = state.filtered_runs();
                            if let Some(run) = runs.get(state.selected_run_idx) {
                                let run_id = run.database_id;
                                state.screen_stack.push(Screen::Detail(run_id));
                                if let Some(repo) = state.current_repo().cloned() {
                                    effects.push(Effect::FetchRunDetail(
                                        repo,
                                        run_id,
                                        Some(run.attempt),
                                    ));
                                }
                            }
                        }
                        Screen::Detail(run_id) => {
                            if let Some(detail) = state.run_details.get(&run_id).cloned() {
                                let nodes = flat_nodes(&detail);
                                if let Some(node) = nodes.get(state.selected_node_idx).copied() {
                                    let ji = node_job_idx(node);
                                    if let Some(job) = detail.jobs.get(ji) {
                                        let job_id = job.database_id;
                                        let step_filter = match node {
                                            FlatNode::Step(_, si) => {
                                                job.steps.get(si).map(|st| st.name.clone())
                                            }
                                            FlatNode::Job(_) => None,
                                        };
                                        state.screen_stack.push(Screen::Log(run_id, job_id));
                                        state.log_viewer = Some(LogViewerState {
                                            job_id,
                                            step_filter,
                                            log: None,
                                            load_error: None,
                                            selected_line: 0,
                                            search_query: String::new(),
                                            search_active: false,
                                            search_matches: Vec::new(),
                                            current_match: 0,
                                            folded_groups: std::collections::HashSet::new(),
                                            show_timestamps: state.config.ui.timestamps,
                                            wrap_lines: state.config.ui.wrap,
                                        });
                                        if let Some(repo) = state.current_repo().cloned() {
                                            effects.push(Effect::FetchJobLog(repo, job_id, false));
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    Action::RerunFailed => {
                        let runs = state.filtered_runs();
                        if let Some(run) = runs.get(state.selected_run_idx) {
                            state.dialog = Some(DialogState {
                                dialog_type: DialogType::RerunFailed(
                                    run.database_id,
                                    run.workflow_name.clone(),
                                    run.head_branch.clone(),
                                    1,
                                ),
                                focus_confirm: false, // default cancel
                                is_running: false,
                                error: None,
                            });
                        }
                    }
                    Action::CancelRun => {
                        let runs = state.filtered_runs();
                        if let Some(run) = runs.get(state.selected_run_idx) {
                            if run.health == Health::Running || run.health == Health::Queued {
                                state.dialog = Some(DialogState {
                                    dialog_type: DialogType::CancelRun(
                                        run.database_id,
                                        run.workflow_name.clone(),
                                        run.head_branch.clone(),
                                    ),
                                    focus_confirm: false, // default cancel
                                    is_running: false,
                                    error: None,
                                });
                            }
                        }
                    }
                    Action::OpenBrowser => {
                        let runs = state.filtered_runs();
                        if let Some(run) = runs.get(state.selected_run_idx) {
                            effects.push(Effect::OpenBrowser(run.url.clone()));
                        }
                    }
                    Action::Top => match state.current_screen() {
                        Screen::Dashboard => state.selected_run_idx = 0,
                        Screen::Detail(_) => state.selected_node_idx = 0,
                        Screen::Log(_, _) => {
                            if let Some(ref mut lv) = state.log_viewer {
                                lv.selected_line = 0;
                            }
                        }
                    },
                    Action::Bottom => match state.current_screen() {
                        Screen::Dashboard => {
                            state.selected_run_idx = state.filtered_runs().len().saturating_sub(1);
                        }
                        Screen::Detail(run_id) => {
                            if let Some(detail) = state.run_details.get(&run_id) {
                                state.selected_node_idx =
                                    flat_nodes(detail).len().saturating_sub(1);
                            }
                        }
                        Screen::Log(_, _) => {
                            log_end(&mut state, false);
                        }
                    },
                    Action::NextMatch => {
                        if let Some(ref mut lv) = state.log_viewer {
                            if !lv.search_matches.is_empty() {
                                lv.current_match = (lv.current_match + 1) % lv.search_matches.len();
                                lv.selected_line = lv.search_matches[lv.current_match];
                            }
                        }
                    }
                    Action::PrevMatch => {
                        if let Some(ref mut lv) = state.log_viewer {
                            if !lv.search_matches.is_empty() {
                                lv.current_match = lv.current_match.saturating_sub(1);
                                if lv.current_match >= lv.search_matches.len() {
                                    lv.current_match = lv.search_matches.len().saturating_sub(1);
                                }
                                lv.selected_line = lv.search_matches[lv.current_match];
                            }
                        }
                    }
                    Action::CopyLine => {
                        // Copy the selected log line, prefixed with the
                        // timestamp when the viewer is showing them, so what
                        // lands in the clipboard matches what is on screen.
                        let to_copy = state.log_viewer.as_ref().and_then(|lv| {
                            let log = lv.log.as_ref()?;
                            let line = log.lines.get(lv.selected_line)?;
                            let text = match (lv.show_timestamps, line.timestamp) {
                                (true, Some(ts)) => {
                                    format!("{} {}", ts.format("%H:%M:%S"), line.message)
                                }
                                _ => line.message.clone(),
                            };
                            Some(text)
                        });
                        if let Some(text) = to_copy {
                            effects.push(Effect::CopyToClipboard(text));
                        }
                    }
                    Action::NextError => {
                        jump_error(&mut state, true);
                    }
                    Action::PrevError => {
                        jump_error(&mut state, false);
                    }
                    Action::ToggleFold => {
                        if let Some(ref mut lv) = state.log_viewer {
                            if let Some(ref log) = lv.log {
                                let idx = lv.selected_line.min(log.lines.len().saturating_sub(1));
                                if let Some(start) = group_start_at(&log.lines, idx) {
                                    if !lv.folded_groups.remove(&start) {
                                        lv.folded_groups.insert(start);
                                    }
                                }
                            }
                        }
                        clamp_log_selection(&mut state);
                    }
                    Action::ToggleFoldAll => {
                        if let Some(ref mut lv) = state.log_viewer {
                            if let Some(ref log) = lv.log {
                                let all: Vec<usize> = log
                                    .lines
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, l)| l.is_group_start)
                                    .map(|(i, _)| i)
                                    .collect();
                                if lv.folded_groups.len() == all.len() && !all.is_empty() {
                                    lv.folded_groups.clear();
                                } else {
                                    lv.folded_groups.extend(all);
                                }
                            }
                        }
                        clamp_log_selection(&mut state);
                    }
                    Action::ToggleTimestamps => {
                        if let Some(ref mut lv) = state.log_viewer {
                            lv.show_timestamps = !lv.show_timestamps;
                        }
                    }
                    Action::ToggleWrap => {
                        if let Some(ref mut lv) = state.log_viewer {
                            lv.wrap_lines = !lv.wrap_lines;
                        }
                    }
                    Action::SwitchAttempt => {
                        if let Screen::Detail(run_id) = state.current_screen() {
                            let max_attempt = state
                                .run_details
                                .get(&run_id)
                                .map(|d| d.run.attempt)
                                .unwrap_or(1)
                                .max(1);
                            let cur = state.detail_attempt.get(&run_id).copied().unwrap_or(1);
                            // cycle 1..=max+1 so a fresh rerun attempt is reachable
                            let next = cur % (max_attempt + 1) + 1;
                            state.detail_attempt.insert(run_id, next);
                            state.selected_node_idx = 0;
                            if let Some(repo) = state.current_repo().cloned() {
                                effects.push(Effect::FetchRunDetail(repo, run_id, Some(next)));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        AppEvent::Tick => {
            state.anim.tick();
        }

        AppEvent::QuitRequested => {
            state.should_quit = true;
        }

        AppEvent::RepoDetected(res) => match res {
            Ok(repo) => {
                if !state.repos.contains(&repo) {
                    state.repos.push(repo.clone());
                }
                effects.push(Effect::FetchRuns(repo));
                effects.push(Effect::FetchRateLimit);
            }
            Err(e) => {
                state.error_banner = Some(e.to_string());
            }
        },

        AppEvent::RunsLoaded(repo, res) => match res {
            Ok(runs) => {
                state.is_loading = false;
                state.runs.insert(repo.name_with_owner(), runs);
                // keep selection in bounds after list changes
                let len = state.filtered_runs().len();
                if state.selected_run_idx >= len {
                    state.selected_run_idx = len.saturating_sub(1);
                }
            }
            Err(e) => {
                state.is_loading = false;
                state.error_banner = Some(e.to_string());
            }
        },

        AppEvent::RunDetailLoaded(_repo, run_id, res) => match res {
            Ok(detail) => {
                state.is_loading = false;
                state.cache.insert_run_detail(run_id, detail.clone());
                state.run_details.insert(run_id, detail.clone());
                // preselect first failed step so the failure is visible
                state.selected_node_idx =
                    default_node(&detail).min(flat_nodes(&detail).len().saturating_sub(1));

                // Annotations live in the job logs, so a freshly opened run has
                // none until those land. Pull the interesting ones now rather
                // than leaving the summary silent: a failing job always matters,
                // and capped so a wide matrix does not fan out into a wall of
                // requests.
                let interesting: Vec<JobId> = detail
                    .jobs
                    .iter()
                    .filter(|j| {
                        j.conclusion == Conclusion::Failure
                            || j.steps.iter().any(|s| s.conclusion == Conclusion::Failure)
                    })
                    .map(|j| j.database_id)
                    .take(8)
                    .collect();
                if !interesting.is_empty()
                    && let Some(repo) = state.current_repo().cloned()
                {
                    for job_id in interesting {
                        if state.cache.get_log(job_id).is_none() {
                            effects.push(Effect::FetchJobLog(repo.clone(), job_id, false));
                        }
                    }
                }
            }
            Err(e) => {
                state.is_loading = false;
                state.error_banner = Some(e.to_string());
            }
        },

        AppEvent::LogLoaded(_repo, job_id, res) => match res {
            Ok(log) => {
                state.is_loading = false;
                state.cache.insert_log(job_id, log.clone());
                // Harvest annotations from the lines we just parsed, before the
                // viewer consumes them.
                let found: Vec<Annotation> = log
                    .lines
                    .iter()
                    .filter_map(|l| l.annotation.clone())
                    .collect();
                if !found.is_empty() {
                    state.annotations.insert(job_id, found);
                }
                if let Some(ref mut lv) = state.log_viewer {
                    if lv.job_id == job_id {
                        lv.log = Some(log);
                        lv.load_error = None;
                        // re-apply search over the fresh lines
                        apply_search(lv);
                    }
                }
                clamp_log_selection(&mut state);
            }
            Err(e) => {
                state.is_loading = false;
                // keep any existing log visible; surface the failure
                // inline so the next poll retry can replace it.
                if let Some(ref mut lv) = state.log_viewer {
                    if lv.job_id == job_id {
                        lv.load_error = Some(e.to_string().to_lowercase());
                    }
                } else {
                    state.error_banner = Some(e.to_string());
                }
            }
        },

        AppEvent::RateLimitLoaded(res) => match res {
            Ok(rl) => {
                state.rate_limit = Some(rl);
            }
            Err(_) => {}
        },

        AppEvent::ActionResult(name, res) => {
            // Log exports report through the footer rather than a dialog —
            // they are fire-and-forget by design.
            if let Some(job) = name.strip_prefix("export job ") {
                match res {
                    Ok(_) => {
                        // keep the confirmation the footer already shows
                        let _ = job;
                    }
                    Err(e) => {
                        state.error_banner = Some(e.to_string());
                        state.last_export = None;
                    }
                }
                return (state, effects);
            }
            if let Some(ref mut d) = state.dialog {
                d.is_running = false;
                match res {
                    Ok(_) => {
                        state.dialog = None;
                        if let Some(repo) = state.current_repo().cloned() {
                            effects.push(Effect::FetchRuns(repo));
                        }
                    }
                    Err(e) => {
                        d.error = Some(e.to_string());
                    }
                }
            }
        }

        _ => {}
    }

    (state, effects)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewer_with_line(message: &str, timestamps: bool) -> AppState {
        let mut s = AppState::new(Config::default());
        s.screen_stack = vec![Screen::Log(1, 2)];
        s.log_viewer = Some(LogViewerState {
            job_id: 2,
            step_filter: None,
            log: Some(RawLog {
                job_id: 2,
                lines: vec![LogLine {
                    job_id: 2,
                    step_name: None,
                    timestamp: Some(chrono::DateTime::from_timestamp(0, 0).unwrap()),
                    message: message.to_string(),
                    level: LogLevel::Normal,
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
            folded_groups: Default::default(),
            show_timestamps: timestamps,
            wrap_lines: false,
        });
        s
    }

    fn press_y(s: AppState) -> Vec<Effect> {
        let key = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('y'),
            crossterm::event::KeyModifiers::NONE,
        );
        update(s, AppEvent::Input(key)).1
    }

    #[test]
    fn y_copies_the_selected_line() {
        let fx = press_y(viewer_with_line("assertion failed", false));
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::CopyToClipboard(t) if t == "assertion failed")),
            "expected the line text, got {fx:?}"
        );
    }

    #[test]
    fn y_includes_the_timestamp_when_shown() {
        let fx = press_y(viewer_with_line("boom", true));
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::CopyToClipboard(t) if t == "00:00:00 boom")),
            "timestamp should prefix the copy, got {fx:?}"
        );
    }

    #[test]
    fn export_saves_what_you_see() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let raw = concat!(
            "j\tstep one\t2026-01-01T00:00:00Z ##[group]setup\n",
            "j\tstep one\t2026-01-01T00:00:01Z installing\n",
            "j\tstep two\t2026-01-01T00:00:02Z testing\n",
            "j\tstep one\t2026-01-01T00:00:03Z ##[endgroup]\n",
        );
        let log = crate::gh::logs::parse_log_output(9, raw);

        let mut s = AppState::new(Config::default());
        s.repos = vec![RepoRef::new("o", "r")];
        s.screen_stack = vec![Screen::Log(1, 9)];
        s.log_viewer = Some(LogViewerState {
            job_id: 9,
            step_filter: Some("step one".to_string()),
            log: Some(log),
            load_error: None,
            selected_line: 0,
            search_query: String::new(),
            search_active: false,
            search_matches: Vec::new(),
            current_match: 0,
            folded_groups: [0usize].into_iter().collect(),
            show_timestamps: false,
            wrap_lines: false,
        });

        let key = KeyEvent::new(KeyCode::Char('S'), KeyModifiers::NONE);
        let (s, fx) = update(s, AppEvent::Input(key));

        let Some(Effect::ExportLog { path, text, .. }) = fx
            .into_iter()
            .find(|e| matches!(e, Effect::ExportLog { .. }))
        else {
            panic!("expected an ExportLog effect");
        };

        // step filter + folded group: only the collapsed header survives
        assert!(text.contains("setup"), "folded group header must show");
        assert!(
            text.contains("lines hidden"),
            "fold must be marked, not silent"
        );
        assert!(!text.contains("testing"), "other step must be filtered out");
        assert!(!text.contains("installing"), "folded body must not leak");

        assert!(
            path.to_string_lossy().contains("o-r-1-job-9.log"),
            "filename should identify repo, run and job: {}",
            path.display()
        );
        assert!(
            s.last_export
                .as_deref()
                .unwrap_or("")
                .contains("o-r-1-job-9.log"),
            "footer confirmation must name the file"
        );
    }

    #[test]
    fn export_only_fires_on_the_log_screen() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let s = AppState::new(Config::default());
        let key = KeyEvent::new(KeyCode::Char('S'), KeyModifiers::NONE);
        let (_, fx) = update(s, AppEvent::Input(key));
        assert!(
            !fx.iter().any(|e| matches!(e, Effect::ExportLog { .. })),
            "S on the dashboard must not export anything"
        );
    }

    #[test]
    fn export_result_error_surfaces_as_banner() {
        let s = AppState::new(Config::default());
        let (s, _) = update(
            s,
            AppEvent::ActionResult(
                "export job 9".to_string(),
                Err(crate::gh::GhError::Other("disk full".to_string())),
            ),
        );
        assert!(
            s.error_banner
                .as_deref()
                .unwrap_or("")
                .contains("disk full"),
            "export failure must be visible, not silent"
        );
        assert!(s.last_export.is_none());
    }

    #[test]
    fn y_is_inert_without_a_log() {
        let mut s = viewer_with_line("x", false);
        s.log_viewer = None;
        assert!(
            !press_y(s)
                .iter()
                .any(|e| matches!(e, Effect::CopyToClipboard(_)))
        );
    }

    #[test]
    fn test_app_state_initialization() {
        let app = AppState::new(Config::default());
        assert_eq!(app.current_screen(), Screen::Dashboard);
        assert!(!app.should_quit);
    }

    fn fixture_detail() -> RunDetail {
        let txt = std::fs::read_to_string("tests/fixtures/run_detail.json").unwrap();
        let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
        crate::gh::parse::parse_run_detail_json(&v).unwrap()
    }

    #[test]
    fn flat_nodes_walk_jobs_and_steps() {
        let d = fixture_detail();
        let nodes = flat_nodes(&d);
        // 2 jobs, first job has 2 steps in the fixture
        assert!(nodes.len() >= 3);
        assert!(matches!(nodes[0], FlatNode::Job(0)));
    }

    #[test]
    fn detail_loaded_preselects_failed_step() {
        let d = fixture_detail();
        let id = d.run.database_id;
        let repo = RepoRef::new("owner", "repo");
        let (s, _) = update(
            AppState::new(Config::default()),
            AppEvent::RunDetailLoaded(repo, id, Ok(d.clone())),
        );
        let nodes = flat_nodes(&d);
        let expect = default_node(&d);
        assert_eq!(s.selected_node_idx, expect);
        assert!(expect > 0, "failed step should not be row 0: {nodes:?}");
    }

    #[test]
    fn step_select_opens_scoped_log() {
        let d = fixture_detail();
        let id = d.run.database_id;
        let repo = RepoRef::new("owner", "repo");
        let (mut s, _) = update(
            AppState::new(Config::default()),
            AppEvent::RunDetailLoaded(repo.clone(), id, Ok(d.clone())),
        );
        // select the failed step row and open it
        s.selected_node_idx = default_node(&d);
        s.screen_stack.push(Screen::Detail(id));
        s.current_repo_idx = 0;
        s.repos = vec![repo];
        let key = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        let (s, fx) = update(s, AppEvent::Input(key));
        assert!(matches!(s.current_screen(), Screen::Log(_, _)));
        assert!(s.log_viewer.as_ref().unwrap().step_filter.is_some());
        assert!(fx.iter().any(|e| matches!(e, Effect::FetchJobLog(_, _, _))));
    }

    #[test]
    fn log_error_keeps_old_log() {
        let mut s = AppState::new(Config::default());
        s.log_viewer = Some(LogViewerState {
            job_id: 7,
            step_filter: None,
            log: None,
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
        let repo = RepoRef::new("owner", "repo");
        let (s, _) = update(
            s,
            AppEvent::LogLoaded(repo, 7, Err(crate::gh::GhError::Timeout)),
        );
        assert!(s.log_viewer.as_ref().unwrap().load_error.is_some());
    }

    #[test]
    fn q_dismisses_dialog() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut s = AppState::new(Config::default());
        s.dialog = Some(DialogState {
            dialog_type: DialogType::CancelRun(1, "wf".into(), "main".into()),
            focus_confirm: false,
            is_running: false,
            error: None,
        });
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let (s, _) = update(s, AppEvent::Input(key));
        assert!(s.dialog.is_none());
        assert!(!s.should_quit);
    }

    #[test]
    fn q_closes_help() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut s = AppState::new(Config::default());
        s.show_help = true;
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let (s, _) = update(s, AppEvent::Input(key));
        assert!(!s.show_help);
        assert!(!s.should_quit);
    }

    #[test]
    fn q_quits_from_dashboard() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let s = AppState::new(Config::default());
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let (s, _) = update(s, AppEvent::Input(key));
        assert!(s.should_quit);
    }

    #[test]
    fn q_pops_log_to_detail() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut s = AppState::new(Config::default());
        s.screen_stack.push(Screen::Detail(1));
        s.screen_stack.push(Screen::Log(1, 2));
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let (s, _) = update(s, AppEvent::Input(key));
        assert!(matches!(s.current_screen(), Screen::Detail(1)));
        assert!(!s.should_quit);
    }

    #[test]
    fn group_fold_math() {
        use crate::gh::{LogLevel, LogLine};
        let mk = |g: bool, e: bool| LogLine {
            job_id: 1,
            step_name: None,
            timestamp: None,
            message: String::new(),
            level: LogLevel::Normal,
            is_group_start: g,
            is_group_end: e,
            annotation: None,
            color_spans: None,
        };
        let lines = vec![mk(true, false), mk(false, false), mk(false, true)];
        assert_eq!(group_start_at(&lines, 1), Some(0));
        assert_eq!(group_start_at(&lines, 2), None);
    }
}
