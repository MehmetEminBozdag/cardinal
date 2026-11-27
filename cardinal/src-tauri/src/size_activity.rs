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
}
