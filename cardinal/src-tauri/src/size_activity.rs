#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn selected_tab_has_fifteen_seconds_background_grace() {
        let now = Instant::now();
        let mut gate = SizeActivity::default();
        gate.foreground = true;
        assert!(!gate.allowed(now));
        gate.selected = true;
        assert!(gate.allowed(now));
        gate.set_foreground(false, now);
        assert!(gate.allowed(now + Duration::from_secs(14)));
        assert!(!gate.allowed(now + Duration::from_secs(15)));
        gate.set_foreground(true, now + Duration::from_secs(20));
        assert!(gate.allowed(now + Duration::from_secs(20)));
        gate.selected = false;
        assert!(!gate.allowed(now + Duration::from_secs(20)));
    }

    #[test]
    fn repeated_background_events_cannot_extend_grace() {
        let now = Instant::now();
        let mut gate = SizeActivity::default();
        gate.selected = true;
        assert!(!gate.allowed(now));
        gate.set_foreground(true, now);
        gate.set_foreground(false, now);
        gate.set_foreground(false, now + Duration::from_secs(10));
        assert!(!gate.allowed(now + Duration::from_secs(15)));
    }

    #[test]
    fn gate_timer_expires_at_fifteen_seconds_and_focus_cancels_deadline() {
        let now = Instant::now();
        let mut gate = SizeActivity::default();
        gate.selected = true;
        gate.set_foreground(true, now);
        assert_eq!(gate.remaining(now), None);
        gate.set_foreground(false, now);
        assert_eq!(gate.remaining(now), Some(Duration::from_secs(15)));
        assert_eq!(
            gate.remaining(now + Duration::from_secs(14)),
            Some(Duration::from_secs(1))
        );
        assert_eq!(gate.remaining(now + Duration::from_secs(15)), None);
        gate.set_foreground(true, now + Duration::from_secs(16));
        assert_eq!(gate.remaining(now + Duration::from_secs(16)), None);
    }
}
use std::time::{Duration, Instant};

pub(crate) const BACKGROUND_GRACE: Duration = Duration::from_secs(15);

#[derive(Default)]
pub(crate) struct SizeActivity {
    pub selected: bool,
    pub foreground: bool,
    background_since: Option<Instant>,
}

impl SizeActivity {
    pub fn set_foreground(&mut self, foreground: bool, now: Instant) {
        if self.foreground && !foreground {
            self.background_since = Some(now);
        }
        if foreground {
            self.background_since = None;
        }
        self.foreground = foreground;
    }

    pub fn allowed(&self, now: Instant) -> bool {
        self.selected
            && (self.foreground
                || self
                    .background_since
                    .is_some_and(|since| now.saturating_duration_since(since) < BACKGROUND_GRACE))
    }

    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        if !self.selected || self.foreground {
            return None;
        }
        self.background_since
            .and_then(|since| BACKGROUND_GRACE.checked_sub(now.saturating_duration_since(since)))
            .filter(|left| !left.is_zero())
    }
}
