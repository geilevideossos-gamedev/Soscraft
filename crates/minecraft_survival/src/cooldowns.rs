#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cooldowns {
    tick: u32,
    items: Vec<(String, u32, u32)>,
}

impl Cooldowns {
    pub fn tick(&mut self, ticks: u32) {
        self.tick = self.tick.saturating_add(ticks);
        let now = self.tick;
        self.items.retain(|&(_, _, end)| end > now);
    }

    pub fn start(&mut self, item: &str, ticks: u32) {
        self.items.retain(|(id, ..)| id != item);
        if ticks > 0 {
            self.items
                .push((item.to_owned(), self.tick, self.tick.saturating_add(ticks)));
        }
    }

    pub fn active(&self, item: &str) -> bool {
        self.items.iter().any(|(id, ..)| id == item)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn shown(&self, partial: f32) -> Vec<(String, f32)> {
        let now = self.tick as f32 + partial;
        self.items
            .iter()
            .map(|(id, start, end)| {
                let left = (*end as f32 - now) / (end - start) as f32;
                (id.clone(), left.clamp(0.0, 1.0))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cooldown_blocks_only_its_own_item_until_it_runs_out() {
        let mut cooldowns = Cooldowns::default();
        assert!(cooldowns.is_empty());
        cooldowns.start("minecraft:ender_pearl", 20);
        cooldowns.tick(3);
        cooldowns.start("minecraft:wind_charge", 10);
        assert!(cooldowns.active("minecraft:ender_pearl"));
        assert!(cooldowns.active("minecraft:wind_charge"));
        assert!(!cooldowns.active("minecraft:snowball"));
        cooldowns.tick(10);
        assert!(cooldowns.active("minecraft:ender_pearl"));
        assert!(!cooldowns.active("minecraft:wind_charge"));
        cooldowns.tick(7);
        assert!(cooldowns.is_empty());
    }

    #[test]
    fn the_slot_overlay_shrinks_with_the_time_left() {
        let mut cooldowns = Cooldowns::default();
        cooldowns.start("minecraft:ender_pearl", 20);
        assert_eq!(
            cooldowns.shown(0.0),
            vec![("minecraft:ender_pearl".to_owned(), 1.0)]
        );
        cooldowns.tick(5);
        assert_eq!(
            cooldowns.shown(0.0),
            vec![("minecraft:ender_pearl".to_owned(), 0.75)]
        );
        assert_eq!(
            cooldowns.shown(0.5),
            vec![("minecraft:ender_pearl".to_owned(), 0.725)]
        );
        cooldowns.start("minecraft:ender_pearl", 10);
        assert_eq!(
            cooldowns.shown(0.0),
            vec![("minecraft:ender_pearl".to_owned(), 1.0)]
        );
    }
}
