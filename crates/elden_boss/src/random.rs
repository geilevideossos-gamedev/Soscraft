#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> SplitMix64 {
        SplitMix64 { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
    }

    pub fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        ((u128::from(self.next_u64()) * u128::from(bound)) >> 64) as u64
    }

    pub fn between(&mut self, min: u16, max: u16) -> u16 {
        if max <= min {
            return min;
        }
        min + self.below(u64::from(max - min) + 1) as u16
    }

    pub fn chance(&mut self, chance: f32) -> bool {
        self.unit() < f64::from(chance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_numbers() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        let mut c = SplitMix64::new(43);
        let first: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let second: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        let other: Vec<u64> = (0..8).map(|_| c.next_u64()).collect();
        assert_eq!(first, second);
        assert_ne!(first, other);
        assert_eq!(SplitMix64::new(0).next_u64(), 0xe220_a839_7b1d_cdaf);
    }

    #[test]
    fn rolls_stay_inside_their_bounds_and_reach_both_ends() {
        let mut random = SplitMix64::new(7);
        let rolls: Vec<u16> = (0..2000).map(|_| random.between(10, 30)).collect();
        assert!(rolls.iter().all(|roll| (10..=30).contains(roll)));
        assert!(rolls.contains(&10) && rolls.contains(&30));
        assert_eq!(random.between(5, 5), 5);
        assert!((0..2000).all(|_| random.unit() < 1.0));
        assert!((0..100).all(|_| random.chance(1.0)));
        assert!((0..100).all(|_| !random.chance(0.0)));
        let thirds = (0..3000).filter(|_| random.chance(1.0 / 3.0)).count();
        assert!((800..1200).contains(&thirds), "{thirds}");
    }
}
