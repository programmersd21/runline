use runline::app::{AppState, update};
use runline::config::Config;
use runline::event::AppEvent;
#[test]
fn scripted_input() {
    let s = AppState::new(Config::default());
    let (n, _) = update(s, AppEvent::Tick);
    let _ = n.current_screen();
}
