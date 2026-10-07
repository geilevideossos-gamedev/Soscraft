pub(crate) const SPAWN_CHUNK_RADIUS: i32 = 3;
pub(crate) const CLOSE_DELAY_MS: u64 = 500;
pub(crate) const CLIENT_WAIT_TIMEOUT_MS: u64 = 30_000;
pub(crate) const SPAWN_EYE_HEIGHT: f64 = 1.62;

#[derive(Default)]
pub(crate) struct SpawnGate {
    started_at: Option<u64>,
    ready_at: Option<u64>,
}

impl SpawnGate {
    pub(crate) fn start(&mut self, now: u64) {
        *self = Self {
            started_at: Some(now),
            ready_at: None,
        };
    }

    pub(crate) fn observe(&mut self, now: u64, chunks_ready: bool, section_visible: bool) {
        if self.ready_at.is_some() {
            return;
        }
        let waited = self
            .started_at
            .map_or(0, |started| now.saturating_sub(started));
        if chunks_ready && section_visible {
            diag::info!(World, "Minecraft spawn: terrain ready after {waited} ms");
            self.ready_at = Some(now);
        } else if self.started_at.is_some() && waited >= CLIENT_WAIT_TIMEOUT_MS {
            diag::warn!(
                World,
                "Minecraft spawn: timed out while waiting for the client to load chunks, letting the player into the world anyway (chunks {chunks_ready}, section {section_visible})"
            );
            self.ready_at = Some(now);
        }
    }

    pub(crate) fn waiting(&self) -> bool {
        self.ready_at.is_none()
    }

    pub(crate) fn holding(&self, now: u64) -> bool {
        self.ready_at
            .is_none_or(|ready| now < ready.saturating_add(CLOSE_DELAY_MS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_gate_holds() {
        let mut gate = SpawnGate::default();
        assert!(gate.holding(0));
        assert!(gate.holding(u64::MAX));
        gate.start(100);
        assert!(gate.holding(100));
        assert!(gate.holding(20_000));
    }

    #[test]
    fn holds_until_chunks_and_section_are_ready() {
        let mut gate = SpawnGate::default();
        gate.start(0);
        gate.observe(1000, true, false);
        assert!(gate.holding(5000));
        gate.observe(1000, false, true);
        assert!(gate.holding(5000));
        gate.observe(6000, true, true);
        assert!(gate.holding(6499));
        assert!(!gate.holding(6500));
    }

    #[test]
    fn releases_half_a_second_after_ready() {
        let mut gate = SpawnGate::default();
        gate.start(0);
        assert!(gate.waiting());
        gate.observe(1000, true, true);
        assert!(!gate.waiting());
        assert!(gate.holding(1499));
        assert!(!gate.holding(1500));
        gate.observe(1600, false, false);
        assert!(!gate.holding(1600));
    }

    #[test]
    fn gives_up_after_thirty_seconds() {
        let mut gate = SpawnGate::default();
        gate.start(0);
        gate.observe(29_999, false, false);
        assert!(gate.holding(30_000));
        gate.observe(30_000, false, false);
        assert!(gate.holding(30_499));
        assert!(!gate.holding(30_500));
    }

    #[test]
    fn a_new_world_waits_again() {
        let mut gate = SpawnGate::default();
        gate.start(0);
        gate.observe(10, true, true);
        assert!(!gate.holding(600));
        gate.start(700);
        assert!(gate.holding(5000));
        gate.observe(5000, true, true);
        assert!(!gate.holding(5500));
    }
}
