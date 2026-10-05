/// two-frame nerd-font spinner used for per-run status glyphs
const STATUS_FRAMES: &[&str] = &["\u{f111}", "\u{f10c}"];

/// braille dots spinner — the iconic terminal loader look
const BRAILLE_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Clone)]
pub struct AnimState {
    pub frame: usize,
    pub reduce_motion: bool,
}

impl AnimState {
    pub fn new(reduce_motion: bool) -> Self {
        Self {
            frame: 0,
            reduce_motion,
        }
    }

    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// nerd-font spinner for run status rows (● when reduce_motion)
    pub fn spinner_glyph(&self) -> &'static str {
        if self.reduce_motion {
            "●"
        } else {
            STATUS_FRAMES[self.frame % STATUS_FRAMES.len()]
        }
    }

    /// braille spinner for reload / fetch overlays (● when reduce_motion)
    pub fn braille(&self) -> &'static str {
        if self.reduce_motion {
            "●"
        } else {
            BRAILLE_FRAMES[self.frame % BRAILLE_FRAMES.len()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_frames() {
        let mut anim = AnimState::new(false);
        assert_eq!(anim.spinner_glyph(), "\u{f111}");
        anim.tick();
        assert_eq!(anim.spinner_glyph(), "\u{f10c}");
    }

    #[test]
    fn test_reduced_motion() {
        let anim = AnimState::new(true);
        assert_eq!(anim.spinner_glyph(), "●");
    }

    #[test]
    fn test_braille_cycles() {
        let mut anim = AnimState::new(false);
        let f0 = anim.braille();
        for _ in 0..BRAILLE_FRAMES.len() {
            anim.tick();
        }
        // after a full cycle we're back to the same frame index
        assert_eq!(anim.braille(), f0);
    }
}
