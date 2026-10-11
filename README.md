# runline

keyboard-first terminal client for watching and acting on github actions runs.

![demo](assets/demo.png)

## what it does
when ci fails, people open a browser, wait for pages, expand a job tree, and scroll a long log to find one error line. runline puts that loop in the terminal and makes it take seconds. it lists recent workflow runs with failures first, shows runs as trees of jobs and steps with timings and the failing step pre-selected, provides searchable logs with jump-to-error, and lets you rerun failed jobs or cancel runs safely.

- **annotations panel**: errors, warnings and notices from workflow commands (`::error file=…,line=…::`) are collected across the run's jobs and shown in their own pane on the run view, worst first, with file:line and step. press `A` to expand the full feed, `j`/`k` to scroll it.
- **original log colours**: compiler output keeps the colours the runner gave it — errors red, warnings yellow — resolved through the active theme instead of hardcoded terminal colours. non-colour escapes (cursor moves, screen clears) are still dropped.
- **log export**: `S` saves exactly what the viewer shows (step filter, folds, timestamps) to `<owner>-<repo>-<run>-job-<id>.log` in the current directory.
- **attempt switching**: `a` cycles run attempts, with the header showing the selected attempt immediately.

## installation
```bash
cargo install runline
```

or download a prebuilt archive from the [releases page](https://github.com/owner/runline/releases):
linux (glibc and musl, x86_64 and aarch64), macos (x86_64 and aarch64) and windows (x86_64).
every release ships a `SHA256SUMS` file next to the archives.

## requirements
- github cli (`gh`) version 2.40.0 or newer.
- authenticated with `gh auth login`.

## quick start
run inside a git repository with github actions:
```bash
runline
```
or point at any repository explicitly — slug, full url, or ssh remote all work, which covers the case where the current directory is not a github checkout:
```bash
runline --repo octocat/hello-world
runline --repo https://github.com/octocat/hello-world
runline --repo git@github.com:octocat/hello-world.git
```
or test offline with bundled fixtures:
```bash
runline --demo
```

## keybinding table

global:
- `?`: toggle help overlay
- `q` / `esc`: back, or quit from dashboard
- `ctrl+c`: quit immediately
- `ctrl+r` / `R`: force refresh
- `T`: interactive theme picker
- `j` / `k`: move selection
- `g` / `G`: jump to top / bottom

dashboard:
- `enter`: open run detail
- `/`: filter runs by keyword
- `b`: toggle local branch scope
- `tab` / `shift+tab`: switch repository tab
- `r`: rerun failed jobs (with confirmation)
- `c`: cancel active run (with confirmation)
- `o`: open run in browser

run detail:
- `enter`: open log for the selected job or step
- `a`: switch run attempt
- `A`: expand / collapse the annotations feed (`j`/`k` scroll it while expanded)
- `r` / `c` / `o`: rerun, cancel, browser — same as dashboard

log viewer:
- `/`: search in log (`n` / `N`: next / previous match)
- `e` / `E`: next / previous error
- `y`: copy the selected line to clipboard
- `S`: save the visible log to a file
- `z` / `Z`: fold one group / all groups
- `t`: toggle timestamps
- `w`: toggle line wrapping

## comparison with other tools

| tool | primary focus | runline comparison |
|---|---|---|
| `gh-dash` | pull requests and issues dashboard | `gh-dash` focuses on PR review workflows; runline focuses entirely on real-time actions execution monitoring, step tree debugging, and log error jumping. |
| `ghr` | triggering workflows from command line | `ghr` triggers runs; runline provides an interactive terminal dashboard and log inspection client. |
| `wrkflw` | terminal UI for github actions | runline offers strict lowercase adherence, terminal palette derivation, 50k line log streaming, and zero-stub v1 scope. |

## license
mit
