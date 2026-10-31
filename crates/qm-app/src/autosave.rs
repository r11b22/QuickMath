//! Debounce logic for autosaving: save once typing has paused for a while.

/// How long typing must pause before an autosave fires.
pub const AUTOSAVE_DELAY_SECONDS: f64 = 1.0;

/// Tracks when the last unsaved edit happened. Time is passed in explicitly
/// (egui's `InputState::time`) so the logic stays deterministic and testable.
#[derive(Debug, Clone, Default)]
pub struct AutosaveTimer {
    last_edit_time: Option<f64>,
}

impl AutosaveTimer {
    /// Records an edit at `now`, restarting the debounce window.
    pub fn record_edit(&mut self, now: f64) {
        self.last_edit_time = Some(now);
    }

    /// Clears the pending autosave, e.g. after any successful save.
    pub fn clear(&mut self) {
        self.last_edit_time = None;
    }

    /// Whether an edit is pending and typing has paused long enough.
    pub fn is_due(&self, now: f64) -> bool {
        self.last_edit_time
            .is_some_and(|edit_time| now - edit_time >= AUTOSAVE_DELAY_SECONDS)
    }

    /// Seconds until the pending autosave fires, if one is pending.
    pub fn seconds_until_due(&self, now: f64) -> Option<f64> {
        self.last_edit_time
            .map(|edit_time| (edit_time + AUTOSAVE_DELAY_SECONDS - now).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_due_without_edits() {
        let timer = AutosaveTimer::default();
        assert!(!timer.is_due(100.0));
        assert_eq!(timer.seconds_until_due(100.0), None);
    }

    #[test]
    fn edit_becomes_due_after_the_delay() {
        let mut timer = AutosaveTimer::default();
        timer.record_edit(10.0);
        assert!(!timer.is_due(10.5));
        assert!(timer.is_due(10.0 + AUTOSAVE_DELAY_SECONDS));
    }

    #[test]
    fn further_typing_restarts_the_window() {
        let mut timer = AutosaveTimer::default();
        timer.record_edit(10.0);
        timer.record_edit(10.8);
        assert!(!timer.is_due(11.2));
        assert!(timer.is_due(11.8));
    }

    #[test]
    fn clearing_cancels_the_pending_save() {
        let mut timer = AutosaveTimer::default();
        timer.record_edit(1.0);
        timer.clear();
        assert!(!timer.is_due(50.0));
    }

    #[test]
    fn seconds_until_due_counts_down_and_floors_at_zero() {
        let mut timer = AutosaveTimer::default();
        timer.record_edit(2.0);
        assert_eq!(timer.seconds_until_due(2.25), Some(0.75));
        assert_eq!(timer.seconds_until_due(9.0), Some(0.0));
    }
}
