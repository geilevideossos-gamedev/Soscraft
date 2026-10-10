use std::collections::BTreeMap;

use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::rng::LegacyRandom;

pub const END_CRYSTAL: &str = "minecraft:end_crystal";
pub const WIDTH: f64 = 2.0;
pub const HEIGHT: f64 = 2.0;
pub const EXPLOSION_POWER: f32 = 6.0;
pub const TRACKING_RANGE: f64 = 256.0;
pub const OBSIDIAN: &str = "minecraft:obsidian";
pub const BEDROCK: &str = "minecraft:bedrock";
const USE_REACH: f64 = 4.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageKind {
    Attack,
    Explosion,
    Fire,
    Dragon,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndDamage {
    pub amount: f32,
    pub player: Option<u64>,
    pub kind: DamageKind,
}

impl EndDamage {
    pub fn attack(amount: f32, player: u64) -> Self {
        Self {
            amount,
            player: Some(player),
            kind: DamageKind::Attack,
        }
    }

    pub fn explosion(amount: f32, player: Option<u64>) -> Self {
        Self {
            amount,
            player,
            kind: DamageKind::Explosion,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndTarget {
    Crystal(u64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EndCrystal {
    pub id: u64,
    pub position: DVec3,
    pub time: i32,
    pub beam_target: Option<(i32, i32, i32)>,
    pub show_bottom: bool,
    pub invulnerable: bool,
    pub tag: Tag,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrystalView {
    pub id: u64,
    pub position: DVec3,
    pub time: i32,
    pub beam_target: Option<(i32, i32, i32)>,
    pub show_bottom: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Destroyed {
    pub id: u64,
    pub position: DVec3,
    pub explodes: bool,
    pub player: Option<u64>,
}

pub fn destroyed_by(damage: &EndDamage, invulnerable: bool) -> Option<bool> {
    if invulnerable || damage.kind == DamageKind::Fire || damage.kind == DamageKind::Dragon {
        return None;
    }
    Some(damage.kind != DamageKind::Explosion)
}

pub fn in_blast(position: DVec3, center: DVec3, radius: f32) -> bool {
    let reach = f64::from(radius * 2.0);
    reach >= f64::from(1.0e-5_f32) && position.distance(center) / reach <= 1.0
}

pub fn takes_crystal(block: &str) -> bool {
    block == OBSIDIAN || block == BEDROCK
}

pub fn placed_at(base: (i32, i32, i32)) -> DVec3 {
    DVec3::new(
        f64::from(base.0) + 0.5,
        f64::from(base.1 + 1),
        f64::from(base.2) + 0.5,
    )
}

pub fn placement_box(base: (i32, i32, i32)) -> (DVec3, DVec3) {
    let low = DVec3::new(f64::from(base.0), f64::from(base.1 + 1), f64::from(base.2));
    (low, low + DVec3::new(1.0, 2.0, 1.0))
}

pub fn crystal_use(
    player: &minecraftoss_player::Player,
    world: &impl minecraftoss_player::World,
    held: &minecraftoss_player::inventory::ItemStack,
) -> Option<crate::bucket::ItemUse> {
    if held.id != END_CRYSTAL {
        return None;
    }
    let hit = player.target(world, USE_REACH)?;
    if !takes_crystal(&world.block(hit.pos)?.id) {
        return None;
    }
    Some(crate::bucket::ItemUse {
        change: crate::bucket::Change::Server(hit.pos, crate::server::ItemOnBlock::EndCrystal),
        result: None,
        wear: 0,
        consumed: true,
        sounds: Vec::new(),
    })
}

pub fn pick_box(position: DVec3) -> (DVec3, DVec3) {
    let half = WIDTH / 2.0;
    (
        position - DVec3::new(half, 0.0, half),
        position + DVec3::new(half, HEIGHT, half),
    )
}

pub fn ray_box(eye: DVec3, look: DVec3, reach: f64, (low, high): (DVec3, DVec3)) -> Option<f64> {
    let direction = look.normalize_or_zero();
    let (mut near, mut far) = (0.0_f64, reach);
    for axis in 0..3 {
        let (start, step) = (eye[axis], direction[axis]);
        if step.abs() < 1.0e-12 {
            if start < low[axis] || start > high[axis] {
                return None;
            }
            continue;
        }
        let a = (low[axis] - start) / step;
        let b = (high[axis] - start) / step;
        near = near.max(a.min(b));
        far = far.min(a.max(b));
        if near > far {
            return None;
        }
    }
    Some(near)
}

pub fn on_ray<'a>(
    crystals: impl IntoIterator<Item = &'a CrystalView>,
    eye: DVec3,
    look: DVec3,
    reach: f64,
) -> Option<(u64, f64)> {
    crystals
        .into_iter()
        .filter_map(|crystal| {
            ray_box(eye, look, reach, pick_box(crystal.position))
                .map(|distance| (crystal.id, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

fn doubles(tag: &Tag, key: &str) -> Option<DVec3> {
    let Some(Tag::List(values)) = tag.get(key) else {
        return None;
    };
    let value = |i: usize| values.get(i).and_then(Tag::as_f64);
    Some(DVec3::new(value(0)?, value(1)?, value(2)?))
}

fn block_pos(tag: &Tag, key: &str) -> Option<(i32, i32, i32)> {
    match tag.get(key)? {
        Tag::IntArray(values) if values.len() == 3 => Some((values[0], values[1], values[2])),
        _ => None,
    }
}

fn flag(tag: &Tag, key: &str, default: bool) -> bool {
    tag.get(key)
        .and_then(Tag::as_i64)
        .map_or(default, |value| value != 0)
}

impl EndCrystal {
    pub fn from_tag(tag: &Tag, id: u64, random: &mut LegacyRandom) -> Option<Self> {
        if tag.get("id").and_then(Tag::as_str) != Some(END_CRYSTAL) {
            return None;
        }
        Some(Self {
            id,
            position: doubles(tag, "Pos")?,
            time: random.next_int(100_000) as i32,
            beam_target: block_pos(tag, "beam_target"),
            show_bottom: flag(tag, "ShowBottom", true),
            invulnerable: flag(tag, "Invulnerable", false),
            tag: tag.clone(),
        })
    }

    pub fn to_tag(&self) -> Tag {
        let mut tag = self.tag.clone();
        if let Tag::Compound(map) = &mut tag {
            map.insert(
                "Pos".to_owned(),
                Tag::List(self.position.to_array().map(Tag::Double).to_vec()),
            );
            match self.beam_target {
                Some((x, y, z)) => {
                    map.insert("beam_target".to_owned(), Tag::IntArray(vec![x, y, z]));
                }
                None => {
                    map.remove("beam_target");
                }
            }
            map.insert(
                "ShowBottom".to_owned(),
                Tag::Byte(i8::from(self.show_bottom)),
            );
        }
        tag
    }

    pub fn block_position(&self) -> (i32, i32, i32) {
        (
            self.position.x.floor() as i32,
            self.position.y.floor() as i32,
            self.position.z.floor() as i32,
        )
    }

    pub fn view(&self) -> CrystalView {
        CrystalView {
            id: self.id,
            position: self.position,
            time: self.time,
            beam_target: self.beam_target,
            show_bottom: self.show_bottom,
        }
    }
}

pub struct Crystals {
    pub list: Vec<EndCrystal>,
    next_id: u64,
    random: LegacyRandom,
}

impl Default for Crystals {
    fn default() -> Self {
        Self {
            list: Vec::new(),
            next_id: 0,
            random: LegacyRandom::new(0x00E1_DC12_57A1),
        }
    }
}

impl Crystals {
    pub fn add(&mut self, tag: &Tag) -> Option<u64> {
        let crystal = EndCrystal::from_tag(tag, self.next_id + 1, &mut self.random)?;
        self.next_id += 1;
        self.list.push(crystal);
        Some(self.next_id)
    }

    pub fn fresh_tag(&mut self, position: DVec3, show_bottom: bool) -> Tag {
        let most = (self.random.next_long() & !0xF000) | 0x4000;
        let least = (self.random.next_long() & 0x3FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000;
        let uuid = [
            (most >> 32) as i32,
            most as i32,
            (least >> 32) as i32,
            least as i32,
        ];
        Tag::Compound(BTreeMap::from([
            ("id".to_owned(), Tag::String(END_CRYSTAL.to_owned())),
            (
                "Pos".to_owned(),
                Tag::List(position.to_array().map(Tag::Double).to_vec()),
            ),
            (
                "Rotation".to_owned(),
                Tag::List(vec![Tag::Float(0.0), Tag::Float(0.0)]),
            ),
            ("UUID".to_owned(), Tag::IntArray(uuid.to_vec())),
            ("ShowBottom".to_owned(), Tag::Byte(i8::from(show_bottom))),
        ]))
    }

    pub fn tick(&mut self) -> Vec<(i32, i32, i32)> {
        self.list
            .iter_mut()
            .map(|crystal| {
                crystal.time += 1;
                crystal.block_position()
            })
            .collect()
    }

    pub fn hurt(&mut self, id: u64, damage: &EndDamage) -> Option<Destroyed> {
        let index = self.list.iter().position(|crystal| crystal.id == id)?;
        let explodes = destroyed_by(damage, self.list[index].invulnerable)?;
        let crystal = self.list.remove(index);
        Some(Destroyed {
            id,
            position: crystal.position,
            explodes,
            player: damage.player,
        })
    }

    pub fn blast(&mut self, center: DVec3, radius: f32, player: Option<u64>) -> Vec<Destroyed> {
        let hit: Vec<u64> = self
            .list
            .iter()
            .filter(|crystal| in_blast(crystal.position, center, radius))
            .map(|crystal| crystal.id)
            .collect();
        hit.into_iter()
            .filter_map(|id| self.hurt(id, &EndDamage::explosion(1.0, player)))
            .collect()
    }

    pub fn tags_where(&self, keep: impl Fn(DVec3) -> bool) -> Vec<Tag> {
        self.list
            .iter()
            .filter(|crystal| keep(crystal.position))
            .map(EndCrystal::to_tag)
            .collect()
    }

    pub fn remove_where(&mut self, remove: impl Fn(DVec3) -> bool) {
        self.list.retain(|crystal| !remove(crystal.position));
    }

    pub fn views(&self) -> Vec<CrystalView> {
        self.list.iter().map(EndCrystal::view).collect()
    }

    pub fn touches(&self, (low, high): (DVec3, DVec3)) -> bool {
        self.list.iter().any(|crystal| {
            let (a, b) = pick_box(crystal.position);
            a.x < high.x
                && b.x > low.x
                && a.y < high.y
                && b.y > low.y
                && a.z < high.z
                && b.z > low.z
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crystal_tag(at: [f64; 3]) -> Tag {
        Tag::Compound(BTreeMap::from([
            ("id".to_owned(), Tag::String(END_CRYSTAL.to_owned())),
            ("Pos".to_owned(), Tag::List(at.map(Tag::Double).to_vec())),
            ("UUID".to_owned(), Tag::IntArray(vec![1, 2, 3, 4])),
        ]))
    }

    #[test]
    fn any_hit_but_fire_or_the_dragon_destroys_a_crystal_and_only_a_non_explosion_blows_it_up() {
        let bullet = EndDamage::attack(0.5, 0);
        assert_eq!(destroyed_by(&bullet, false), Some(true));
        assert_eq!(
            destroyed_by(&EndDamage::explosion(30.0, Some(0)), false),
            Some(false)
        );
        assert_eq!(
            destroyed_by(&EndDamage::explosion(1.0, None), false),
            Some(false)
        );
        let fire = EndDamage {
            kind: DamageKind::Fire,
            ..bullet
        };
        assert_eq!(destroyed_by(&fire, false), None, "it is fire immune");
        let dragon = EndDamage {
            kind: DamageKind::Dragon,
            player: None,
            ..bullet
        };
        assert_eq!(destroyed_by(&dragon, false), None);
        assert_eq!(destroyed_by(&bullet, true), None, "invulnerable");
    }

    #[test]
    fn a_blast_reaches_crystals_whose_feet_lie_within_twice_its_power() {
        let center = DVec3::new(0.0, 64.0, 0.0);
        assert!(in_blast(DVec3::new(12.0, 64.0, 0.0), center, 6.0));
        assert!(!in_blast(DVec3::new(12.01, 64.0, 0.0), center, 6.0));
        assert!(!in_blast(center, center, 0.0));
        let mut crystals = Crystals::default();
        let near = crystals.add(&crystal_tag([3.5, 65.0, 0.5])).unwrap();
        let far = crystals.add(&crystal_tag([40.5, 65.0, 0.5])).unwrap();
        let gone = crystals.blast(center, 4.0, Some(0));
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].id, near);
        assert!(
            !gone[0].explodes,
            "a crystal killed by an explosion does not explode"
        );
        assert_eq!(crystals.list.len(), 1);
        assert_eq!(crystals.list[0].id, far);
        let hit = crystals.hurt(far, &EndDamage::attack(0.2, 0)).unwrap();
        assert!(hit.explodes);
        assert_eq!(hit.position, DVec3::new(40.5, 65.0, 0.5));
        assert!(crystals.list.is_empty());
        assert_eq!(crystals.hurt(far, &EndDamage::attack(0.2, 0)), None);
    }

    #[test]
    fn crystals_keep_their_beam_target_and_base_through_a_save() {
        let mut crystals = Crystals::default();
        assert_eq!(crystals.add(&Tag::Compound(BTreeMap::new())), None);
        let mut tag = crystal_tag([3.5, 76.0, -41.5]);
        if let Tag::Compound(map) = &mut tag {
            map.insert("beam_target".to_owned(), Tag::IntArray(vec![0, 128, 0]));
            map.insert("ShowBottom".to_owned(), Tag::Byte(0));
        }
        let id = crystals.add(&tag).unwrap();
        let crystal = crystals.list[0].clone();
        assert_eq!(crystal.id, id);
        assert_eq!(crystal.beam_target, Some((0, 128, 0)));
        assert!(!crystal.show_bottom);
        assert!((0..100_000).contains(&crystal.time));
        assert_eq!(crystal.block_position(), (3, 76, -42));
        let saved = crystal.to_tag();
        assert_eq!(saved.get("UUID"), Some(&Tag::IntArray(vec![1, 2, 3, 4])));
        assert_eq!(saved.get("ShowBottom"), Some(&Tag::Byte(0)));
        let mut again = Crystals::default();
        again.add(&saved);
        assert_eq!(again.list[0].beam_target, Some((0, 128, 0)));
        assert!(!again.list[0].show_bottom);
        let plain = Crystals::default().list.len();
        assert_eq!(plain, 0);
        let mut fresh = Crystals::default();
        fresh.add(&crystal_tag([0.5, 1.0, 0.5]));
        assert!(
            fresh.list[0].show_bottom,
            "a crystal shows its base by default"
        );
        assert_eq!(fresh.list[0].to_tag().get("beam_target"), None);
        let before = fresh.list[0].time;
        assert_eq!(fresh.tick(), vec![(0, 1, 0)]);
        assert_eq!(fresh.list[0].time, before + 1);
    }

    #[test]
    fn a_crystal_goes_on_obsidian_or_bedrock_one_block_up_in_its_middle() {
        assert!(takes_crystal(OBSIDIAN) && takes_crystal(BEDROCK));
        assert!(!takes_crystal("minecraft:end_stone"));
        assert_eq!(placed_at((3, 64, -2)), DVec3::new(3.5, 65.0, -1.5));
        assert_eq!(
            placement_box((3, 64, -2)),
            (DVec3::new(3.0, 65.0, -2.0), DVec3::new(4.0, 67.0, -1.0))
        );
        let mut crystals = Crystals::default();
        crystals.add(&crystal_tag([3.5, 65.0, -1.5]));
        assert!(crystals.touches(placement_box((3, 64, -2))));
        assert!(
            crystals.touches(placement_box((4, 64, -2))),
            "two blocks wide"
        );
        assert!(!crystals.touches(placement_box((6, 64, -2))));
    }

    struct Below(minecraftoss_player::Block);

    impl minecraftoss_player::World for Below {
        fn block(&self, pos: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            (pos == (0, 0, 0)).then(|| self.0.clone())
        }

        fn set_block(
            &mut self,
            _: minecraftoss_player::Pos,
            _: Option<minecraftoss_player::Block>,
        ) {
        }
    }

    #[test]
    fn an_end_crystal_used_on_obsidian_or_bedrock_asks_the_server_and_is_spent() {
        let mut player = minecraftoss_player::Player::new(DVec3::new(0.5, 1.0, 0.5));
        player.pitch = 90.0;
        let crystal = minecraftoss_player::inventory::ItemStack::new(END_CRYSTAL, 8);
        for base in [OBSIDIAN, BEDROCK] {
            let world = Below(minecraftoss_player::Block::new(base));
            let used = crystal_use(&player, &world, &crystal).expect("aimed at the base");
            assert_eq!(
                used.change,
                crate::bucket::Change::Server((0, 0, 0), crate::server::ItemOnBlock::EndCrystal)
            );
            assert!(used.consumed && used.result.is_none() && used.sounds.is_empty());
        }
        let stone = Below(minecraftoss_player::Block::new("minecraft:end_stone"));
        assert_eq!(crystal_use(&player, &stone, &crystal), None);
        let pearl = minecraftoss_player::inventory::ItemStack::new("minecraft:ender_pearl", 1);
        let obsidian = Below(minecraftoss_player::Block::new(OBSIDIAN));
        assert_eq!(crystal_use(&player, &obsidian, &pearl), None);
    }

    #[test]
    fn a_look_at_a_crystal_finds_its_two_block_box() {
        let mut crystals = Crystals::default();
        let id = crystals.add(&crystal_tag([0.5, 64.0, 5.5])).unwrap();
        let eye = DVec3::new(0.5, 65.0, 0.5);
        let views = crystals.views();
        let (found, distance) = on_ray(&views, eye, DVec3::Z, 6.0).unwrap();
        assert_eq!(found, id);
        assert!((distance - 4.0).abs() < 1e-9);
        assert_eq!(on_ray(&views, eye, DVec3::Z, 3.0), None);
        assert_eq!(on_ray(&views, eye, -DVec3::Z, 6.0), None);
    }
}
