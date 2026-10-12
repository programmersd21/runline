# changelog

## 1.0.0 - 2026-10-12
- keyboard-first terminal client for watching and acting on github actions runs.
- runs dashboard with failures-first sorting, duration bars, branch filter, and multi-repo tabs.
- run detail tree of jobs and steps with timings, failed-step preselect, attempt switching, and first-error preview.
- annotations panel: errors, warnings and notices collected across failing jobs, worst first with file:line and step, expandable and scrollable.
- log viewer with the runner's original colours, search, jump-to-error, folds, timestamps, wrapping, clipboard copy (`y`) and file export (`S`).
- rerun-failed and cancel actions with confirmation dialogs, plus open-in-browser.
- `--repo` accepts a slug, full url, ssh remote or `gh:` shorthand; interactive theme picker; offline demo mode.
- hardened terminal handling: full repaint on screen change, control-character scrubbing, display-width layout, clean quit on every path.
