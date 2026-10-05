use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Back,
    ForceRefresh,
    ToggleHelp,
    MoveUp,
    MoveDown,
    Top,
    Bottom,
    Select,
    ToggleAnnotations,
    Filter,
    ToggleBranch,
    NextTab,
    PrevTab,
    RerunFailed,
    CancelRun,
    OpenBrowser,
    Collapse,
    Expand,
    SwitchAttempt,
    Search,
    NextMatch,
    PrevMatch,
    NextError,
    PrevError,
    ToggleFold,
    ToggleFoldAll,
    ToggleTimestamps,
    ToggleWrap,
    CopyLine,
    ExportLog,
    ThemePicker,
}

#[derive(Debug, Clone)]
pub struct KeyMap {
    pub bindings: HashMap<String, Action>,
}

impl Default for KeyMap {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // navigation
        bindings.insert("q".into(), Action::Quit);
        bindings.insert("ctrl+c".into(), Action::Quit);
        bindings.insert("esc".into(), Action::Quit);
        bindings.insert("j".into(), Action::MoveDown);
        bindings.insert("k".into(), Action::MoveUp);
        bindings.insert("down".into(), Action::MoveDown);
        bindings.insert("up".into(), Action::MoveUp);
        bindings.insert("g".into(), Action::Top);
        bindings.insert("G".into(), Action::Bottom);
        bindings.insert("enter".into(), Action::Select);

        // dashboard
        bindings.insert("/".into(), Action::Filter);
        bindings.insert("b".into(), Action::ToggleBranch);
        bindings.insert("tab".into(), Action::NextTab);
        bindings.insert("shift+tab".into(), Action::PrevTab);
        bindings.insert("r".into(), Action::RerunFailed);
        bindings.insert("c".into(), Action::CancelRun);
        bindings.insert("o".into(), Action::OpenBrowser);
        bindings.insert("a".into(), Action::SwitchAttempt);

        // detail tree
        bindings.insert("h".into(), Action::Collapse);
        bindings.insert("l".into(), Action::Expand);

        // log viewer
        bindings.insert("n".into(), Action::NextMatch);
        bindings.insert("N".into(), Action::PrevMatch);
        bindings.insert("e".into(), Action::NextError);
        bindings.insert("E".into(), Action::PrevError);
        bindings.insert("z".into(), Action::ToggleFold);
        bindings.insert("A".into(), Action::ToggleAnnotations);
        bindings.insert("Z".into(), Action::ToggleFoldAll);
        bindings.insert("t".into(), Action::ToggleTimestamps);
        bindings.insert("w".into(), Action::ToggleWrap);
        bindings.insert("y".into(), Action::CopyLine);
        bindings.insert("S".into(), Action::ExportLog);

        // global
        bindings.insert("R".into(), Action::ForceRefresh);
        bindings.insert("ctrl+r".into(), Action::ForceRefresh);
        bindings.insert("?".into(), Action::ToggleHelp);
        bindings.insert("T".into(), Action::ThemePicker);

        Self { bindings }
    }
}

impl KeyMap {
    /// Returns Some(action) only for Press events. Release/Repeat are ignored
    /// so keys don't double-fire on terminals that send both (kitty protocol,
    /// Windows ConPTY, etc.).
    pub fn parse_event(&self, key: KeyEvent) -> Option<Action> {
        // only act on actual key presses — ignore release / repeat
        if key.kind != KeyEventKind::Press {
            return None;
        }

        let base = match key.code {
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "enter".into(),
            KeyCode::Esc => "esc".into(),
            KeyCode::Tab => "tab".into(),
            KeyCode::BackTab => "shift+tab".into(),
            KeyCode::Up => "up".into(),
            KeyCode::Down => "down".into(),
            KeyCode::Left => "left".into(),
            KeyCode::Right => "right".into(),
            _ => return None,
        };

        // ctrl modifier prefix (only ctrl — shift is already encoded in the
        // uppercase char, e.g. 'Z' instead of 'z')
        let key_repr = if key.modifiers.contains(KeyModifiers::CONTROL) {
            format!("ctrl+{}", base)
        } else {
            base
        };

        self.bindings.get(&key_repr).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keymap_default() {
        let km = KeyMap::default();
        let ev = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(km.parse_event(ev), Some(Action::Quit));
    }

    #[test]
    fn test_release_ignored() {
        let km = KeyMap::default();
        let mut ev = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        ev.kind = KeyEventKind::Release;
        assert_eq!(km.parse_event(ev), None);
    }

    #[test]
    fn test_uppercase_bindings() {
        let km = KeyMap::default();
        // 'R' = ForceRefresh, 'T' = ThemePicker, 'Z' = ToggleFoldAll
        let r_ev = KeyEvent::new(KeyCode::Char('R'), KeyModifiers::NONE);
        assert_eq!(km.parse_event(r_ev), Some(Action::ForceRefresh));

        let t_ev = KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE);
        assert_eq!(km.parse_event(t_ev), Some(Action::ThemePicker));

        let z_ev = KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::NONE);
        assert_eq!(km.parse_event(z_ev), Some(Action::ToggleFoldAll));
    }

    #[test]
    fn test_ctrl_bindings() {
        let km = KeyMap::default();
        let ev = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(km.parse_event(ev), Some(Action::Quit));
    }

    #[test]
    fn test_esc_quits() {
        let km = KeyMap::default();
        let ev = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        // esc = Quit at top level (pops screen or quits)
        assert_eq!(km.parse_event(ev), Some(Action::Quit));
    }
}
