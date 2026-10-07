use runline::app::{AppState, Screen, update};
use runline::config::Config;
use runline::event::AppEvent;
#[test]
fn open_run_flow() {
    let txt = std::fs::read_to_string("tests/fixtures/run_detail.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    let d = runline::gh::parse::parse_run_detail_json(&v).unwrap();
    let st = AppState::new(Config::default());
    let (s, _) = update(
        st,
        AppEvent::RunDetailLoaded(
            runline::gh::RepoRef::new("owner", "repo"),
            d.run.database_id,
            Ok(d.clone()),
        ),
    );
    assert!(s.run_details.contains_key(&d.run.database_id));
    let _ = Screen::Dashboard;
}
