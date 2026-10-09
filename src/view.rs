use crate::app::{AppState, Screen};

/// Fingerprint of everything the ui layer reads to build a frame.
///
/// ratatui repaints only the cells that differ from its own model of the
/// screen. That is fine while the model and the terminal agree, but they drift
/// apart easily — a resize reflows whatever was painted, content that wrapped
/// past the right edge scrolls the screen, and untrusted text containing a
/// control byte can move the cursor mid-frame. Cells the model believes are
/// already blank are then never rewritten, so the previous screen stays
/// visible as garbage on every screen that follows.
///
/// Comparing signatures lets the render loop notice "this is a different
/// screen" and wipe instead of diffing. Cheap: it only changes on a real
/// transition, not on animation or repaint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSignature {
    stack: Vec<Screen>,
    dialog: bool,
    help: bool,
    theme_picker: bool,
    width: u16,
    height: u16,
}

impl ViewSignature {
    pub fn capture(state: &AppState, width: u16, height: u16) -> Self {
        Self {
            stack: state.screen_stack.clone(),
            dialog: state.dialog.is_some(),
            help: state.show_help,
            theme_picker: state.show_theme_picker,
            width,
            height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{DialogState, DialogType};
    use crate::config::Config;

    fn state() -> AppState {
        AppState::new(Config::default())
    }

    #[test]
    fn stable_across_repaint_and_animation() {
        let mut s = state();
        let a = ViewSignature::capture(&s, 80, 24);
        s.anim.tick();
        let b = ViewSignature::capture(&s, 80, 24);
        assert_eq!(a, b, "repainting the same view must not force a wipe");
    }

    #[test]
    fn changes_on_screen_change() {
        let mut s = state();
        let a = ViewSignature::capture(&s, 80, 24);
        s.screen_stack.push(Screen::Detail(7));
        let b = ViewSignature::capture(&s, 80, 24);
        assert_ne!(a, b);
    }

    #[test]
    fn changes_on_resize() {
        let s = state();
        let a = ViewSignature::capture(&s, 80, 24);
        assert_ne!(a, ViewSignature::capture(&s, 81, 24));
        assert_ne!(a, ViewSignature::capture(&s, 80, 25));
    }

    #[test]
    fn changes_on_every_overlay() {
        let mut s = state();
        let base = ViewSignature::capture(&s, 80, 24);

        s.show_help = true;
        let help = ViewSignature::capture(&s, 80, 24);
        assert_ne!(base, help, "help overlay");
        s.show_help = false;

        s.show_theme_picker = true;
        let picker = ViewSignature::capture(&s, 80, 24);
        assert_ne!(base, picker, "theme picker");
        s.show_theme_picker = false;

        s.dialog = Some(DialogState {
            dialog_type: DialogType::CancelRun(1, "wf".into(), "main".into()),
            focus_confirm: false,
            is_running: false,
            error: None,
        });
        assert_ne!(base, ViewSignature::capture(&s, 80, 24), "dialog");

        assert_eq!(base, {
            s.dialog = None;
            ViewSignature::capture(&s, 80, 24)
        });
    }
}
