use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};
use runline::{
    app::{AppState, update},
    config::load_config,
    event::{AppEvent, Effect},
    gh::{GhBackend, RepoRef, RunQuery, fixture::FixtureBackend, process::ProcessBackend},
    view::ViewSignature,
};
use std::{
    io::{self, Write, stdout},
    panic,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc;

fn print_help() {
    println!(
        "runline 1.0.0\n\
keyboard-first terminal client for watching and acting on github actions runs\n\n\
usage: runline [options]\n\n\
options:\n\
  --repo <repo>         repository as owner/repo, full url, or ssh remote\n\
  --config <path>       custom configuration file path\n\
  --theme <name>        theme name (dusk, paper, mono, terminal)\n\
  --reduce-motion       disable animations and smooth transitions\n\
  --no-cache            disable in-memory and disk cache\n\
  --demo                run in offline demo mode using fixture data\n\
  --version             print version\n\
  --help                print this help message"
    );
}

fn setup_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Always restore terminal on panic, regardless of whether TermGuard fires first.
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);

        let report_dir = std::env::var("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".local/state")
            })
            .join("runline");

        let _ = std::fs::create_dir_all(&report_dir);
        let report_file = report_dir.join("crash.log");
        let _ = std::fs::write(&report_file, format!("{:#?}", panic_info));

        eprintln!(
            "runline crashed. a crash report was saved to {}. press any key to exit.",
            report_file.display()
        );
        default_hook(panic_info);
    }));
}

/// RAII guard that restores the terminal on any exit path — normal return,
/// early `?` error propagation, or panic (panic hook also fires, idempotent).
struct TermGuard;

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = stdout().flush();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut repo_override: Option<String> = None;
    let mut config_path: Option<PathBuf> = None;
    let mut theme_override: Option<String> = None;
    let mut reduce_motion = false;
    let mut no_cache = false;
    let mut demo_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" => {
                println!("runline 1.0.0");
                return Ok(());
            }
            "--repo" => {
                if i + 1 < args.len() {
                    repo_override = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--config" => {
                if i + 1 < args.len() {
                    config_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--theme" => {
                if i + 1 < args.len() {
                    theme_override = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--reduce-motion" => {
                reduce_motion = true;
            }
            "--no-cache" => {
                no_cache = true;
            }
            "--demo" => {
                demo_mode = true;
            }
            _ => {}
        }
        i += 1;
    }

    setup_panic_hook();

    let mut config = load_config(config_path.as_ref()).unwrap_or_default();
    if reduce_motion {
        config.ui.reduce_motion = true;
    }
    if no_cache {
        config.cache.enabled = false;
    }
    if let Some(t) = theme_override {
        config.ui.theme = t;
    }

    let backend: Arc<dyn GhBackend> = if demo_mode {
        Arc::new(FixtureBackend::demo())
    } else {
        Arc::new(ProcessBackend::new())
    };

    let mut state = AppState::new(config.clone());

    // Apply persisted theme
    state.theme = runline::theme::by_name(&state.config.ui.theme);

    // ── Terminal setup ──────────────────────────────────────────────────────
    // enable_raw_mode first; if it succeeds, install the RAII guard immediately
    // so that *any* subsequent failure (execute!, Terminal::new, even a panic
    // later in the event loop) will restore the terminal before we exit.
    enable_raw_mode()?;
    let _guard = TermGuard; // <── installed RIGHT after raw mode succeeds

    let mut stdout_handle = stdout();
    execute!(stdout_handle, EnterAlternateScreen, EnableMouseCapture)?;
    let term_backend = CrosstermBackend::new(stdout_handle);
    let mut terminal = Terminal::new(term_backend)?;
    // wipe any residue frozen in the alternate buffer so a previous
    // killed session can never bleed through as stray characters.
    // resize-to-current-size rather than clear(): same wipe, but no cursor
    // position query (see the note in the render loop).
    let boot = terminal.size()?;
    terminal.resize(Rect::new(0, 0, boot.width, boot.height))?;

    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(100);

    // ── OS signal handling ─────────────────────────────────────────────
    // SIGTERM (kill) and SIGHUP (terminal closed) bypass key input, so
    // without this the process dies mid-frame: raw mode stays on and the
    // frozen ui remains painted in the terminal. route them through the
    // normal quit path instead. (SIGINT arrives as a key event in raw
    // mode and is already handled by the keymap.)
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let sig_tx = event_tx.clone();
        tokio::spawn(async move {
            let mut term = signal(SignalKind::terminate()).ok();
            let mut hup = signal(SignalKind::hangup()).ok();
            tokio::select! {
                _ = async {
                    if let Some(s) = term.as_mut() {
                        let _ = s.recv().await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {}
                _ = async {
                    if let Some(s) = hup.as_mut() {
                        let _ = s.recv().await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {}
            }
            let _ = sig_tx.send(AppEvent::QuitRequested).await;
        });
    }

    // ── Initial repo setup ─────────────────────────────────────────────────
    if let Some(r_str) = repo_override {
        match RepoRef::parse(&r_str) {
            Ok(r) => {
                state.repos.push(r.clone());
                let _ = event_tx.send(AppEvent::RepoDetected(Ok(r))).await;
            }
            Err(e) => {
                state.error_banner = Some(format!("invalid --repo value: {e}"));
            }
        }
    } else if demo_mode {
        let r = backend.current_repo().await;
        let _ = event_tx.send(AppEvent::RepoDetected(r)).await;
    } else {
        let b = backend.clone();
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let res = b.current_repo().await;
            let _ = tx.send(AppEvent::RepoDetected(res)).await;
        });
    }

    // ── Input reader ───────────────────────────────────────────────────────
    // crossterm's `event::read` is a blocking syscall, so it must NOT live in
    // a tokio task: when main returns, the runtime waits for that task to
    // finish, and a task parked inside `read()` never wakes up. That is why
    // `q` tore the ui down (alternate screen restored) but the process then
    // hung instead of exiting.
    //
    // A plain OS thread is detached instead — nothing waits on it, so the
    // process exits as soon as main is done. `blocking_send` wakes up with an
    // error once the receiver is dropped, so the thread also stops tidily.
    let input_tx = event_tx.clone();
    std::thread::spawn(move || {
        loop {
            let Ok(CEvent::Key(k)) = event::read() else {
                continue;
            };
            if input_tx.blocking_send(AppEvent::Input(k)).is_err() {
                break;
            }
        }
    });

    // ── Animation / poll ticker ────────────────────────────────────────────
    let tick_tx = event_tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(80));
        loop {
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).await.is_err() {
                break;
            }
        }
    });

    // ── Main event loop ────────────────────────────────────────────────────
    let mut last_view: Option<ViewSignature> = None;

    loop {
        // ratatui repaints only the cells that differ from its own model of
        // the screen. That is normally enough, but the model and the real
        // terminal can drift apart — a resize reflows whatever was painted,
        // and content that wrapped past the right edge scrolls the screen.
        // Cells the model believes are already blank are then never rewritten,
        // so the previous screen stays visible as garbage *forever*.
        //
        // Signing the view and wiping on change makes every screen change
        // start from a known-clean screen, for all menus and overlays.
        let size = terminal.size()?;
        let view = ViewSignature::capture(&state, size.width, size.height);
        if last_view.as_ref() != Some(&view) {
            // `resize` to the size we already have wipes the screen and resets
            // ratatui's back buffer, so the next frame repaints every cell.
            //
            // deliberately *not* `Terminal::clear()`: that queries the cursor
            // position (DSR), and with mouse capture on crossterm parses the
            // reply `\x1b[n;MR` as a mouse event, so the query times out and
            // takes the whole app down with it.
            terminal.resize(Rect::new(0, 0, size.width, size.height))?;
            last_view = Some(view);
        }

        terminal.draw(|f| runline::ui::draw(f, &state))?;

        if state.should_quit {
            break;
        }

        tokio::select! {
            Some(ev) = event_rx.recv() => {
                let (new_state, effects) = update(state, ev);
                state = new_state;

                for eff in effects {
                    let b = backend.clone();
                    let tx = event_tx.clone();
                    match eff {
                        Effect::FetchRuns(repo) => {
                            tokio::spawn(async move {
                                let res = b.list_runs(&repo, RunQuery { limit: 30, branch: None }).await;
                                let _ = tx.send(AppEvent::RunsLoaded(repo, res)).await;
                            });
                        }
                        Effect::FetchRunDetail(repo, run_id, attempt) => {
                            tokio::spawn(async move {
                                let res = b.view_run(&repo, run_id, attempt).await;
                                let _ = tx.send(AppEvent::RunDetailLoaded(repo, run_id, res)).await;
                            });
                        }
                        Effect::FetchJobLog(repo, job_id, failed_only) => {
                            tokio::spawn(async move {
                                let res = b.job_log(&repo, job_id, failed_only).await;
                                let _ = tx.send(AppEvent::LogLoaded(repo, job_id, res)).await;
                            });
                        }
                        Effect::FetchRateLimit => {
                            tokio::spawn(async move {
                                let res = b.rate_limit().await;
                                let _ = tx.send(AppEvent::RateLimitLoaded(res)).await;
                            });
                        }
                        Effect::RerunFailed(repo, run_id) => {
                            tokio::spawn(async move {
                                let res = b.rerun_failed(&repo, run_id).await;
                                let _ = tx.send(AppEvent::ActionResult("rerun".to_string(), res)).await;
                            });
                        }
                        Effect::CancelRun(repo, run_id) => {
                            tokio::spawn(async move {
                                let res = b.cancel(&repo, run_id).await;
                                let _ = tx.send(AppEvent::ActionResult("cancel".to_string(), res)).await;
                            });
                        }
                        Effect::OpenBrowser(url) => {
                            // detach fully: browsers dump zygote/chromium
                            // noise on stderr when attaching to an
                            // existing session, which would spray over
                            // the terminal. send all stdio to null.
                            use std::process::Stdio;
                            let _ = if cfg!(target_os = "macos") {
                                std::process::Command::new("open")
                                    .arg(&url)
                                    .stdin(Stdio::null())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    .spawn()
                            } else {
                                std::process::Command::new("xdg-open")
                                    .arg(&url)
                                    .stdin(Stdio::null())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    .spawn()
                            };
                        }
                        Effect::ExportLog { job_id, path, text } => {
                            // The only place runline writes a file the user
                            // asked for. Errors surface as an event so the ui
                            // can say what happened instead of failing silent.
                            let tx2 = tx.clone();
                            tokio::task::spawn_blocking(move || {
                                let res = std::fs::write(&path, &text)
                                    .map(|_| ())
                                    .map_err(|e| {
                                        runline::gh::GhError::Other(format!(
                                            "could not write {}: {}",
                                            path.display(),
                                            e
                                        ))
                                    });
                                let _ = tx2.blocking_send(AppEvent::ActionResult(
                                    format!("export job {job_id}"),
                                    res,
                                ));
                            });
                        }
                        Effect::CopyToClipboard(text) => {
                            // OSC 52 goes straight to the tty, bypassing the
                            // frame buffer, so it must be built by `osc52_copy`
                            // (base64 + ST terminator) — never interpolated
                            // raw, or log content could forge the framing.
                            let osc52 = runline::copy::osc52_copy(&text);
                            let mut out = io::stdout();
                            let _ = out.write_all(osc52.as_bytes());
                            let _ = out.flush();
                        }
                    }
                }
            }
        }
    }

    // TermGuard Drop will run here (and also on any earlier `?` return),
    // restoring raw mode, alternate screen, and mouse capture.
    terminal.show_cursor()?;
    drop(_guard);
    let _ = io::stdout().flush();
    std::process::exit(0);
}
