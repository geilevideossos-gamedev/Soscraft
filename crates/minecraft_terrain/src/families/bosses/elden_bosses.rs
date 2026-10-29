use crate::client_mobs::ClientMobs;
use crate::elden_effects::Sparks;
use crate::families::{ClientFx, FamilyDraw, FamilyShow, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use elden_boss::shape::Frame;
use elden_boss::{Clip, State};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::bosses::elden_bosses::EldenBoss;

pub const TRACKING_RANGE: i32 = 10;
pub const SHADOW_RADIUS: f32 = 1.0;
pub const STATE_BITS: u32 = 0xf;
pub const HELD: u32 = 1 << 4;
pub const AWAKE: u32 = 1 << 5;

pub fn show_clip(clip: &Clip) -> FamilyShow {
    let state = State::ALL.iter().position(|state| *state == clip.state);
    let mut flags = state.map_or(0, |state| state as u32);
    if clip.held {
        flags |= HELD;
    }
    if clip.awake {
        flags |= AWAKE;
    }
    let mut values = [0; 8];
    values[0] = i32::from(clip.index);
    values[1] = i32::from(clip.clock);
    values[2] = i32::from(clip.phase);
    values[3] = i32::from(clip.from);
    values[4] = i32::from(clip.criticals);
    FamilyShow { flags, values }
}

pub fn shown_clip(show: &FamilyShow) -> Option<Clip> {
    let state = *State::ALL.get((show.flags & STATE_BITS) as usize)?;
    let [index, clock, phase, from, criticals, ..] = show.values;
    Some(Clip {
        state,
        index: u8::try_from(index).ok()?,
        clock: u16::try_from(clock).ok()?,
        held: show.flags & HELD != 0,
        phase: u8::try_from(phase).ok()?,
        awake: show.flags & AWAKE != 0,
        from: u16::try_from(from).ok()?,
        criticals: u8::try_from(criticals).ok()?,
    })
}

impl TerrainFamily for EldenBoss {
    const TRACKING_RANGE: i32 = TRACKING_RANGE;
    const TEXTURES: &'static [&'static str] = &[];
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        crate::elden_save::read(mob, tag);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        crate::elden_save::fields(mob)
    }

    fn show(mob: &FamilyMob<Self>) -> FamilyShow {
        show_clip(&mob.state.shown)
    }

    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow) {
        if let Some(clip) = shown_clip(show) {
            mob.state.shown = clip;
        }
    }

    fn draw(
        out: &mut FamilyDraw<'_>,
        mobs: &[FamilyMob<Self>],
        poses: &ClientMobs,
        atlas: &Atlas,
        light: &SkyLight,
        partial: f32,
    ) {
        crate::elden_render::append_bosses(out.models, mobs, poses, atlas, light, partial);
    }

    fn client_effects(mobs: &[FamilyMob<Self>], poses: &ClientMobs, fx: &mut ClientFx<'_>) {
        for mob in mobs.iter().filter(|mob| !mob.core.removed) {
            let id = mob.core.id;
            let Some(shown) = poses.get(id) else {
                continue;
            };
            let frame = Frame {
                origin: shown.position,
                yaw: f64::from(shown.y_rot),
            };
            let mut sparks = Sparks {
                ambient: &mut *fx.ambient,
                spell: &mut *fx.spell,
                random: &mut *fx.random,
            };
            let last = fx.seen.get(&id).copied();
            let now = sparks.show(mob.state.def, &mob.state.shown, &frame, last);
            fx.seen.insert(id, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mob_actions::{Actor, attack};
    use elden_boss::margit::MARGIT;
    use glam::DVec3;
    use minecraftoss_entities::tempt::PlayerCandidate;
    use minecraftoss_entities::world::families::Family;
    use minecraftoss_entities::world::{EntityWorld, MobHit, PlayerAttack};
    use minecraftoss_player::inventory::Inventory;
    use minecraftoss_player::{Block, Pos, World};

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn player_at(id: u64, position: DVec3) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position,
            eye_height: 1.62,
            main_hand_cow_food: false,
            offhand_cow_food: false,
            main_hand_pig_food: false,
            offhand_pig_food: false,
            main_hand_chicken_food: false,
            offhand_chicken_food: false,
            main_hand_carrot_on_a_stick: false,
            offhand_carrot_on_a_stick: false,
            main_hand_wolf_interest: false,
            offhand_wolf_interest: false,
            main_hand_horse_tempt: false,
            offhand_horse_tempt: false,
            alive: true,
            spectator: false,
            attackable: true,
        }
    }

    fn sword(player_id: u64, position: DVec3) -> PlayerAttack {
        PlayerAttack {
            player_id,
            position,
            yaw: 0.0,
            attack_damage: 11.0,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        }
    }

    #[test]
    fn two_players_sword_hits_in_one_tick_both_count() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = world.spawn_elden_boss(&MARGIT, at, 0.0, at, false);
        let one = DVec3::new(0.5, 64.0, 3.0);
        let two = DVec3::new(3.0, 64.0, 0.5);
        let players = [player_at(1, one), player_at(2, two)];
        for _ in 0..5 {
            world.tick_with_players(&mut Field, &players);
        }
        let health = |world: &EntityWorld| world.elden_bosses()[0].state.brain.health();
        let before = health(&world);
        assert_eq!(before, 1280.0, "two fighters at the wake");
        let hit = MobHit::Family(EldenBoss::KEY, id);
        let mut sounds = Vec::new();
        for (player, position) in [(1, one), (2, two)] {
            let mut inventory = Inventory::default();
            let mut actor = Actor {
                player,
                inventory: &mut inventory,
                selected: 0,
                infinite: false,
                entity_loot: None,
                shearing_loot: None,
            };
            let outcome = attack(&mut world, hit, &mut actor, &sword(player, position));
            assert!(outcome.handled);
            sounds.extend(outcome.sounds.into_iter().map(|sound| sound.event));
        }
        assert_eq!(health(&world), before - 22.0);
        assert_eq!(world.elden_bosses()[0].core.health, before - 22.0);
        assert_eq!(world.elden_bosses()[0].state.brain.hurt_by(), &[1, 2]);
        let count = |event: &str| sounds.iter().filter(|sound| *sound == event).count();
        assert_eq!(count("entity.player.attack.strong"), 2, "{sounds:?}");
        assert_eq!(
            count("entity.illusioner.hurt"),
            1,
            "one hurt voice for both"
        );
    }

    #[test]
    fn margit_is_drawn_from_his_painted_skin_through_his_family() {
        let temp = crate::test_dir::tempdir().unwrap();
        let meta = r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#;
        std::fs::write(temp.path().join("pack.mcmeta"), meta).unwrap();
        let packs = crate::pack::PackStack::open(vec![temp.path().into()]).unwrap();
        let texture = crate::pack::ResourceId::parse(crate::elden_skin::TEXTURE).unwrap();
        let atlas = crate::mesh::make_atlas(&packs, vec![texture.clone()]).unwrap();
        assert!(atlas.missing.is_empty(), "{:?}", atlas.missing);
        let width = atlas.pixels.width() as f32;
        let region = atlas.region(&texture);
        let (x, y) = (
            (region[0] * width).floor() as u32,
            (region[1] * width).floor() as u32,
        );
        let skin = crate::elden_skin::paint();
        for (u, v) in [(9, 11), (33, 3), (45, 20)] {
            assert_eq!(atlas.pixels.get_pixel(x + u, y + v), skin.get_pixel(u, v));
        }
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        world.spawn_elden_boss(&MARGIT, at, 0.0, at, false);
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&crate::client_mobs::server_mobs(&world));
        let light =
            SkyLight::from_levels((-8, 56, -8), (17, 17, 17), vec![15; 4913], vec![0; 4913]);
        let (mut models, mut culled, mut translucent, mut held) = Default::default();
        let mut out = FamilyDraw {
            models: &mut models,
            culled: &mut culled,
            translucent: &mut translucent,
            held: &mut held,
        };
        crate::families::draw(&mut out, &world, &poses, &atlas, &light, 0.0);
        assert_eq!(models.faces, 6 * 16, "one Margit of sixteen cubes");
        let skin = atlas.entity_region(&texture);
        assert!(models.vertices.iter().all(|vertex| {
            (skin[0]..=skin[2]).contains(&vertex.uv[0])
                && (skin[1]..=skin[3]).contains(&vertex.uv[1])
        }));
        let top = models
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MIN, f32::max);
        assert!(top > 66.4 && top < 66.7, "asleep, he kneels: {top}");
    }

    #[test]
    fn a_friend_sees_margits_clip_from_his_show() {
        let mut host = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = host.spawn_elden_boss(&MARGIT, at, 0.0, at, false);
        let players = [player_at(1, DVec3::new(0.5, 64.0, 3.0))];
        for _ in 0..40 {
            host.tick_with_players(&mut Field, &players);
        }
        assert!(host.elden_bosses()[0].state.brain.awake());
        host.elden_boss_mut(id)
            .unwrap()
            .state
            .brain
            .begin("dagger_throw");
        let shown = |world: &EntityWorld| world.elden_bosses()[0].state.shown;
        for _ in 0..12 {
            host.tick_with_players(&mut Field, &players);
        }
        let held = shown(&host);
        let index = MARGIT.move_index("dagger_throw").unwrap() as u8;
        assert_eq!(
            (held.state, held.index, held.clock, held.held),
            (State::Move, index, 10, true)
        );
        let tags = crate::server_mobs::mob_tags_where(
            &host,
            |_, _| true,
            &std::collections::HashMap::new(),
            false,
        );
        let (_, tag) = tags.into_iter().find(|(saved, _)| *saved == id).unwrap();
        let mut friend = EntityWorld::default();
        assert!(crate::mob_tracking::spawn_tracked(&mut friend, id, &tag));
        assert_eq!(
            shown(&friend).state,
            State::Return,
            "a loaded copy walks home"
        );
        for mob in crate::client_mobs::server_mobs(&host) {
            crate::mob_tracking::show_on(&mut friend, &mob);
        }
        assert_eq!(shown(&friend), held);
        for state in State::ALL {
            let clip = Clip {
                state,
                index: 5,
                clock: 77,
                held: true,
                phase: 2,
                awake: false,
                from: 26,
                criticals: 3,
            };
            assert_eq!(shown_clip(&show_clip(&clip)), Some(clip));
        }
        let unknown = FamilyShow {
            flags: 12,
            values: [0; 8],
        };
        assert_eq!(shown_clip(&unknown), None);
    }

    struct View {
        world: EntityWorld,
        poses: ClientMobs,
        ambient: crate::ambient_particles::AmbientParticles,
        spell: crate::spell_particles::SpellParticles,
        portal: crate::portal_particles::PortalParticles,
        poof: crate::poof_particles::PoofParticles,
        sounds: Vec<(String, DVec3, f32, f32)>,
        random: minecraftoss_player::rng::LegacyRandom,
        seen: std::collections::HashMap<u64, i32>,
    }

    impl View {
        fn of(world: EntityWorld) -> View {
            let mut poses = ClientMobs::default();
            poses.spawn_missing(&crate::client_mobs::server_mobs(&world));
            View {
                world,
                poses,
                ambient: crate::ambient_particles::AmbientParticles::new(1),
                spell: crate::spell_particles::SpellParticles::new(1),
                portal: Default::default(),
                poof: Default::default(),
                sounds: Vec::new(),
                random: minecraftoss_player::rng::LegacyRandom::new(1),
                seen: Default::default(),
            }
        }

        fn effects(&mut self) {
            let mut fx = ClientFx {
                ambient: &mut self.ambient,
                spell: &mut self.spell,
                portal: &mut self.portal,
                poof: &mut self.poof,
                sounds: &mut self.sounds,
                random: &mut self.random,
                seen: &mut self.seen,
            };
            crate::families::client_effects(&self.world, &self.poses, &mut fx);
        }

        fn count(&self, kind: crate::ambient_particles::Kind) -> usize {
            let kinds = self.ambient.kinds_at();
            kinds.filter(|(seen, _)| *seen == kind).count()
        }
    }

    #[test]
    fn a_slam_cue_bursts_on_its_tick_for_host_and_friend_views() {
        use crate::ambient_particles::Kind;
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = world.spawn_elden_boss(&MARGIT, at, 0.0, at, false);
        let players = [player_at(1, DVec3::new(0.5, 64.0, 3.0))];
        for _ in 0..40 {
            world.tick_with_players(&mut Field, &players);
        }
        let tags = crate::server_mobs::mob_tags_where(
            &world,
            |_, _| true,
            &std::collections::HashMap::new(),
            false,
        );
        let (_, tag) = tags.into_iter().find(|(saved, _)| *saved == id).unwrap();
        let mut copy = EntityWorld::default();
        assert!(crate::mob_tracking::spawn_tracked(&mut copy, id, &tag));
        let mut host = View::of(world);
        let mut friend = View::of(copy);
        host.world
            .elden_boss_mut(id)
            .unwrap()
            .state
            .brain
            .begin("cane_slam");
        let mut slams = Vec::new();
        for _ in 0..80 {
            host.world.tick_with_players(&mut Field, &players);
            for mob in crate::client_mobs::server_mobs(&host.world) {
                crate::mob_tracking::show_on(&mut friend.world, &mob);
            }
            let before = (host.count(Kind::Explosion), friend.count(Kind::Explosion));
            host.effects();
            friend.effects();
            let after = (host.count(Kind::Explosion), friend.count(Kind::Explosion));
            if after != before {
                let clip = host.world.elden_bosses()[0].state.shown;
                slams.push((clip.state, clip.clock, clip.held, after));
            }
        }
        assert_eq!(slams, [(State::Move, 29, false, (1, 1))]);
        let poofs = host.count(Kind::Poof);
        assert_eq!(poofs, crate::elden_effects::SLAM_POOFS);
        assert_eq!(friend.count(Kind::Poof), poofs);
    }

    #[test]
    fn golden_mist_shows_his_rise_and_phase_two_drips_gold() {
        use crate::ambient_particles::Kind;
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = world.spawn_elden_boss(&MARGIT, at, 0.0, at, false);
        let mut view = View::of(world);
        view.effects();
        assert_eq!(
            (view.count(Kind::Totem), view.spell.len()),
            (0, 0),
            "asleep"
        );
        let players = [player_at(1, DVec3::new(0.5, 64.0, 10.0))];
        for _ in 0..2 {
            view.world.tick_with_players(&mut Field, &players);
        }
        let rise = view.world.elden_bosses()[0].state.shown;
        assert_eq!((rise.state, rise.clock), (State::Wake, 0));
        view.effects();
        assert!(view.count(Kind::Totem) > 0);
        assert_eq!(view.spell.len(), crate::elden_effects::MIST_SPELLS);
        let mut drips = |clip: Clip| {
            view.world.elden_boss_mut(id).unwrap().state.shown = clip;
            let before = view.spell.len();
            view.effects();
            view.spell.len() - before
        };
        let idle = Clip {
            state: State::Idle,
            index: 0,
            clock: 3,
            held: false,
            phase: 1,
            awake: true,
            from: 0,
            criticals: 0,
        };
        assert_eq!(drips(idle), 0, "phase 1");
        let second = Clip { phase: 2, ..idle };
        assert_eq!(drips(second), crate::elden_effects::SHIMMER);
        assert_eq!(drips(second), crate::elden_effects::SHIMMER, "every tick");
        let dead = Clip {
            state: State::Dead,
            clock: 15,
            ..second
        };
        assert_eq!(drips(dead), 0);
    }
}
