use ratatui::{Terminal, backend::TestBackend};
use runline::{app::AppState, config::Config, theme::builtin::dusk};

#[test]
fn dashboard_snapshot() {
    for (w, h) in [(80, 24), (120, 32)] {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).unwrap();
        let mut s = AppState::new(Config::default());
        s.theme = dusk();
        s.repos = vec![runline::gh::RepoRef::new("owner", "repo")];
        let txt = std::fs::read_to_string("tests/fixtures/runs.json").unwrap();
        let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
        s.runs.insert(
            "owner/repo".into(),
            runline::gh::parse::parse_runs_json(&v).unwrap(),
        );
        term.draw(|f| runline::ui::draw(f, &s)).unwrap();
        let buf = term.backend().buffer().clone();
        assert!(buf.area.width == w);
    }
}
