use crate::protocol::PlayerId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tracker {
    pub total: i32,
    elements: usize,
    only: Option<PlayerId>,
    non_zero: usize,
    only_non_zero: Option<PlayerId>,
}

impl Tracker {
    pub fn track(&mut self, player: PlayerId, value: i32) {
        self.total = self.total.wrapping_add(value);
        self.elements += 1;
        self.only = (self.elements == 1).then_some(player);
        if value != 0 {
            self.non_zero += 1;
            self.only_non_zero = (self.non_zero == 1).then_some(player);
        }
    }

    pub fn single(&self, non_zero: bool) -> Option<PlayerId> {
        if non_zero {
            self.only_non_zero
        } else {
            self.only
        }
    }

    pub fn count(&self, non_zero: bool) -> usize {
        if non_zero {
            self.non_zero
        } else {
            self.elements
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_and_counts_follow_the_vanilla_response_tracker() {
        let mut tracker = Tracker::default();
        assert_eq!((tracker.single(false), tracker.count(false)), (None, 0));
        tracker.track(PlayerId(0), 0);
        assert_eq!(tracker.single(false), Some(PlayerId(0)));
        assert_eq!((tracker.single(true), tracker.count(true)), (None, 0));
        tracker.track(PlayerId(1), 5);
        assert_eq!((tracker.single(false), tracker.count(false)), (None, 2));
        assert_eq!(
            (tracker.single(true), tracker.count(true)),
            (Some(PlayerId(1)), 1)
        );
        tracker.track(PlayerId(2), 3);
        assert_eq!((tracker.single(true), tracker.count(true)), (None, 2));
        assert_eq!(tracker.total, 8);
    }
}
