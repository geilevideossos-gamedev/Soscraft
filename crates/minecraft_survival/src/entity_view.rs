use std::collections::{HashMap, HashSet};

use glam::{DVec3, Vec3};
use minecraft_terrain::client_mobs::{ClientMobs, ServerMob, server_mobs};
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::{Atlas, ItemVisuals};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::poof_particles::PoofParticles;
use minecraft_terrain::portal_particles::PortalParticles;
use minecraft_terrain::scene::{Block, BlockPos, HandcraftedScene, Scene};
use minecraft_terrain::server::{EntitySnapshot, ServerItem, ServerOrb};
use minecraftoss_entities::world::{EntityWorld, MobHit};
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::items::{ItemEntity, WorldItems};

use crate::entities::{
    CRYSTAL_KEY, FIREBALL_KEY, FRAME_KEY, MobMeshes, SHULKER_BULLET_KEY, TICK_SECONDS, bob_offset,
    decode, dragon_part_key, encode, mirrored,
};

const FLAME_RANGE: f64 = 160.0;

pub struct EntityView {
    pub world: EntityWorld,
    pub client: ClientMobs,
    pub(crate) poof: PoofParticles,
    pub(crate) portal: PortalParticles,
    pub(crate) eyes: minecraft_terrain::eye_of_ender::EyeTrails,
    pub(crate) spell: minecraft_terrain::spell_particles::SpellParticles,
    pub(crate) ambient: minecraft_terrain::ambient_particles::AmbientParticles,
    pub(crate) bodies: HashMap<u128, minecraft_terrain::ambient_particles::Body>,
    pub(crate) rope_holders: HashMap<u128, minecraft_terrain::leash_render::RopeHolder>,
    pub(crate) items: ItemVisuals,
    pub(crate) clock: f64,
    pub(crate) ride_clock: f64,
    pub(crate) tick_seconds: f64,
    pub(crate) ticks: u64,
    pub world_items: WorldItems,
    pub(crate) burning_items: Vec<u32>,
    pub(crate) thrown: Vec<minecraft_terrain::thrown::ThrownView>,
    pub(crate) clouds: Vec<minecraft_terrain::effect_cloud::CloudView>,
    pub(crate) swirls: Vec<minecraft_terrain::spell_particles::SwirlView>,
    pub(crate) tridents: Vec<minecraft_terrain::trident::TridentView>,
    pub(crate) crystals: Vec<minecraft_terrain::end_crystal::CrystalView>,
    pub(crate) gateways: minecraft_terrain::end_gateway::ClientGateways,
    pub(crate) dragons: Vec<minecraft_terrain::ender_dragon::DragonView>,
    pub(crate) dragon_fireballs: Vec<minecraft_terrain::dragon_fireball::FireballView>,
    drawn_dragons: minecraft_terrain::client_dragon::ClientDragons,
    drawn_fireballs: minecraft_terrain::client_dragon::ClientFireballs,
    pub(crate) frames: Vec<minecraft_terrain::item_frame::FrameView>,
    pub(crate) growls: HashMap<u64, i32>,
    pub(crate) orbs: Vec<ServerOrb>,
    pub(crate) falling: minecraft_terrain::falling_render::FallingBlocks,
    pub lids: minecraft_terrain::lids::Lids,
    pub(crate) spawners: minecraft_terrain::spawner_display::SpawnerDisplays,
    pub(crate) orb_pickups: Vec<minecraft_terrain::orb_render::OrbPickup>,
    pub item_crumbs: Vec<(String, DVec3, Vec<Vec3>)>,
    pub potion_crumbs: Vec<DVec3>,
    pub sounds: Vec<(String, DVec3, f32, f32)>,
    pub(crate) random: minecraftoss_player::rng::LegacyRandom,
    shown: HashSet<u64>,
    fang_crits: HashSet<u64>,
    ravager_roars: HashSet<u64>,
    fireworks: minecraft_terrain::firework_particles::FireworkParticles,
    firework_camera: DVec3,
}

impl EntityView {
    pub fn new(seed: i64) -> Self {
        Self {
            world: EntityWorld::default(),
            client: ClientMobs::default(),
            poof: PoofParticles::default(),
            portal: PortalParticles::default(),
            eyes: minecraft_terrain::eye_of_ender::EyeTrails::default(),
            spell: minecraft_terrain::spell_particles::SpellParticles::new((seed ^ 0x5be1) as u64),
            ambient: minecraft_terrain::ambient_particles::AmbientParticles::new(
                (seed ^ 0xa3b1) as u64,
            ),
            bodies: HashMap::new(),
            rope_holders: HashMap::new(),
            items: ItemVisuals::default(),
            clock: 0.0,
            ride_clock: 0.0,
            tick_seconds: TICK_SECONDS,
            ticks: 0,
            world_items: WorldItems::default(),
            burning_items: Vec::new(),
            thrown: Vec::new(),
            clouds: Vec::new(),
            swirls: Vec::new(),
            tridents: Vec::new(),
            crystals: Vec::new(),
            gateways: Default::default(),
            dragons: Vec::new(),
            dragon_fireballs: Vec::new(),
            drawn_dragons: Default::default(),
            drawn_fireballs: Default::default(),
            frames: Vec::new(),
            growls: HashMap::new(),
            orbs: Vec::new(),
            falling: Default::default(),
            lids: Default::default(),
            spawners: minecraft_terrain::spawner_display::SpawnerDisplays::new(
                (seed ^ 0x5b3d) as u64,
            ),
            orb_pickups: Vec::new(),
            item_crumbs: Vec::new(),
            potion_crumbs: Vec::new(),
            sounds: Vec::new(),
            random: minecraftoss_player::rng::LegacyRandom::new((seed ^ 0x1735) as u64),
            shown: HashSet::new(),
            fang_crits: HashSet::new(),
            ravager_roars: HashSet::new(),
            fireworks: minecraft_terrain::firework_particles::FireworkParticles::new((seed ^ 0xf14e) as u64),
            firework_camera: DVec3::ZERO,
        }
    }

    pub fn advance(
        &mut self,
        dt: f64,
        client: crate::tick_rate::ClientTiming,
        near: &[DVec3],
        orb_target: DVec3,
    ) -> bool {
        self.tick_seconds = client.tick_seconds;
        if client.runs_normally {
            self.clock += dt;
        }
        let ticked = client.runs_normally && self.clock >= self.tick_seconds;
        if ticked {
            self.clock = (self.clock - self.tick_seconds).min(self.tick_seconds);
            self.ticks += 1;
            self.falling.tick_sent();
            for tnt in self.falling.primed.iter().filter(|tnt| tnt.fuse > 0) {
                let smoke = DVec3::from_array(tnt.position) + DVec3::Y * 0.5;
                self.ambient.add(
                    minecraft_terrain::ambient_particles::Kind::Smoke,
                    smoke,
                    DVec3::ZERO,
                );
            }
            self.lids.tick();
            self.spawners.tick(near, &mut self.ambient);
            self.client.tick();
            self.spell_casts();
            self.fang_crits();
            self.ravager_effects();
            self.wither_effects();
            let flaps = self.drawn_dragons.tick();
            self.dragon_voices(&flaps);
            self.drawn_fireballs.tick();
            for enderman in self.world.endermen() {
                if let Some(mob) = self.client.get(enderman.id) {
                    self.portal.emit_enderman(mob.position, 0.6, 2.9);
                }
            }
            self.eyes.tick(&mut self.portal);
            for blaze in self.world.blazes() {
                if let Some(mob) = self
                    .client
                    .get(blaze.id)
                    .filter(|_| self.random.next_int(24) == 0)
                {
                    let volume = 1.0 + self.random.next_float();
                    let pitch = self.random.next_float() * 0.7 + 0.3;
                    self.sounds.push((
                        "minecraft:entity.blaze.burn".to_owned(),
                        mob.position + DVec3::splat(0.5),
                        volume,
                        pitch,
                    ));
                }
            }
            self.portal.tick();
            for cloud in &self.clouds {
                if cloud.breath {
                    self.ambient
                        .breath_cloud(cloud.position, cloud.radius, cloud.waiting);
                } else {
                    self.spell
                        .cloud(cloud.position, cloud.radius, cloud.color, cloud.waiting);
                }
            }
            for swirl in &self.swirls {
                self.spell.swirl(swirl);
            }
            self.spell.tick();
            minecraft_terrain::orb_render::tick_pickups(&mut self.orb_pickups, orb_target);
            if self.ticks.is_multiple_of(600) {
                diag::info!(World, "Minecraft mobs: {} tracked", self.boxes().len());
            }
        }
        if client.runs_normally {
            self.ride_clock = self.clock;
        } else {
            self.ride_clock += dt;
            if self.ride_clock >= self.tick_seconds {
                self.ride_clock = (self.ride_clock - self.tick_seconds).min(self.tick_seconds);
                self.client.tick_only(&self.world.player_carriers());
            }
        }

        ticked
    }

    fn spell_casts(&mut self) {
        use minecraftoss_entities::illager::IllagerSpell;
        use minecraftoss_player::mth;
        for evoker in self.world.illagers() {
            if evoker.illager.spell == IllagerSpell::None {
                continue;
            }
            let Some(mob) = self.client.pose(evoker.id, 1.0) else {
                continue;
            };
            let [r, g, b] = evoker.illager.spell.color();
            let turn = mob.body_rot * 0.017_453_292 + mth::cos(f64::from(mob.age_in_ticks * 0.6662)) * 0.25;
            let (cos, sin) = (f64::from(mth::cos(f64::from(turn))), f64::from(mth::sin(f64::from(turn))));
            for side in [1.0, -1.0] {
                let at = mob.feet + DVec3::new(side * cos * 0.6, 1.8, side * sin * 0.6);
                self.spell.spawn(
                    minecraft_terrain::spell_particles::SpellKind::Effect,
                    at,
                    DVec3::ZERO,
                    [r as f32, g as f32, b as f32],
                    1.0,
                );
            }
        }
    }

    fn fang_crits(&mut self) {
        use minecraftoss_entities::evoker_fangs::{CRIT_LIFE, WIDTH};
        let fangs: Vec<(u64, DVec3, i32)> = self.world.evoker_fangs().iter().filter(|e| e.fangs.sent_spike).map(|e| (e.id, e.fangs.position, e.fangs.life_ticks)).collect();
        self.fang_crits.retain(|id| fangs.iter().any(|(fang, ..)| fang == id));
        for (id, at, life) in fangs {
            if life > CRIT_LIFE || !self.fang_crits.insert(id) {
                continue;
            }
            for _ in 0..12 {
                let x = at.x + (self.random.next_double() * 2.0 - 1.0) * WIDTH * 0.5;
                let y = at.y + 0.05 + self.random.next_double();
                let z = at.z + (self.random.next_double() * 2.0 - 1.0) * WIDTH * 0.5;
                let motion = DVec3::new(
                    (self.random.next_double() * 2.0 - 1.0) * 0.3,
                    0.3 + self.random.next_double() * 0.3,
                    (self.random.next_double() * 2.0 - 1.0) * 0.3,
                );
                self.ambient.add(minecraft_terrain::ambient_particles::Kind::Crit, DVec3::new(x, y + 1.0, z), motion);
            }
        }
    }

    fn ravager_effects(&mut self) {
        use minecraftoss_entities::ravager::{ROAR_AT, ROAR_PARTICLES, ROAR_SPREAD, STUNNED_COLOR};
        let ravagers: Vec<(u64, minecraftoss_entities::ravager::Ravager)> = self.world.ravagers().iter().map(|e| (e.id, e.ravager.clone())).collect();
        self.ravager_roars.retain(|id| ravagers.iter().any(|(ravager, r)| ravager == id && r.roar_tick > 0 && r.roar_tick <= ROAR_AT));
        for (id, ravager) in ravagers {
            if ravager.stunned_tick > 0 {
                let body_yaw = self.client.pose(id, 1.0).map_or(ravager.yaw, |mob| mob.body_rot);
                if let Some(at) = ravager.stun_particle(&mut self.random, body_yaw) {
                    self.spell.spawn(minecraft_terrain::spell_particles::SpellKind::Effect, at, DVec3::ZERO, STUNNED_COLOR, 1.0);
                }
            }
            if ravager.roar_tick > 0 && ravager.roar_tick <= ROAR_AT && self.ravager_roars.insert(id) {
                let center = ravager.body.position + DVec3::new(0.0, f64::from(ravager.body.height) / 2.0, 0.0);
                for _ in 0..ROAR_PARTICLES {
                    let motion = DVec3::new(self.random.next_gaussian() * ROAR_SPREAD, self.random.next_gaussian() * ROAR_SPREAD, self.random.next_gaussian() * ROAR_SPREAD);
                    self.ambient.add(minecraft_terrain::ambient_particles::Kind::Poof, center, motion);
                }
            }
        }
    }

    fn wither_effects(&mut self) {
        use minecraftoss_entities::wither::{head_position, CHARGE_COLOR, CHARGE_PARTICLE_HEIGHT, POWERED_COLOR, SMOKE_SPREAD};
        use minecraft_terrain::spell_particles::SpellKind;
        let withers: Vec<(u64, minecraftoss_entities::wither::Wither)> = self.world.withers().iter().filter(|e| e.wither.health > 0.0).map(|e| (e.id, e.wither.clone())).collect();
        for (id, wither) in withers {
            let body_yaw = self.client.pose(id, 1.0).map_or(wither.body_yaw, |mob| mob.body_rot);
            let spread = f64::from(SMOKE_SPREAD);
            for head in 0..3 {
                let at = head_position(wither.body.position, body_yaw, head);
                let smoke = at + DVec3::new(self.random.next_gaussian() * spread, self.random.next_gaussian() * spread, self.random.next_gaussian() * spread);
                self.ambient.add(minecraft_terrain::ambient_particles::Kind::Smoke, smoke, DVec3::ZERO);
                if wither.powered() && self.random.next_int(4) == 0 {
                    let glow = at + DVec3::new(self.random.next_gaussian() * spread, self.random.next_gaussian() * spread, self.random.next_gaussian() * spread);
                    self.spell.spawn(SpellKind::Effect, glow, DVec3::ZERO, POWERED_COLOR, 1.0);
                }
            }
            if wither.charging() {
                let p = wither.body.position;
                for _ in 0..3 {
                    let at = DVec3::new(p.x + self.random.next_gaussian(), p.y + f64::from(self.random.next_float() * CHARGE_PARTICLE_HEIGHT), p.z + self.random.next_gaussian());
                    self.spell.spawn(SpellKind::Effect, at, DVec3::ZERO, CHARGE_COLOR, 1.0);
                }
            }
        }
    }

    fn mob_puffs(&mut self, puffs: &[minecraft_terrain::mob_particles::MobPuff]) {
        use minecraft_terrain::ambient_particles::Kind;
        use minecraft_terrain::mob_particles::PuffKind;
        for puff in puffs {
            let kind = match puff.kind {
                PuffKind::Heart => Kind::Heart,
                PuffKind::Smoke => Kind::Smoke,
                PuffKind::HappyVillager => Kind::HappyVillager,
                PuffKind::AngryVillager | PuffKind::Splash => continue,
            };
            let (width, height) = (f64::from(puff.width), f64::from(puff.height));
            for _ in 0..puff.count {
                let random = &mut self.random;
                let motion = DVec3::new(random.next_gaussian() * 0.02, random.next_gaussian() * 0.02, random.next_gaussian() * 0.02);
                let x = puff.feet.x + width * (2.0 * random.next_double() - 1.0);
                let y = puff.feet.y + height * random.next_double() + puff.lift;
                let z = puff.feet.z + width * (2.0 * random.next_double() - 1.0);
                self.ambient.add(kind, DVec3::new(x, y, z), motion);
            }
        }
    }

    pub fn bursts(&mut self, snapshot: &mut EntitySnapshot) {
        self.mob_puffs(&std::mem::take(&mut snapshot.mob_puffs));
        let shattered = std::mem::take(&mut snapshot.eyes.shattered);
        self.eyes.shatter(shattered, &mut self.portal, &mut self.item_crumbs);
        for at in std::mem::take(&mut snapshot.breath_bursts) {
            self.ambient.breath_burst(at);
        }
        for breath in std::mem::take(&mut snapshot.dragon_breath) {
            self.ambient.mouth_breath(&breath);
        }
        for (at, emitter) in std::mem::take(&mut snapshot.dragon_explosions) {
            let kind = if emitter {
                minecraft_terrain::ambient_particles::Kind::ExplosionEmitter
            } else {
                minecraft_terrain::ambient_particles::Kind::Explosion
            };
            self.ambient.add(kind, at, DVec3::ZERO);
        }
        for burst in std::mem::take(&mut snapshot.firework_bursts) {
            let (event, volume, pitch) = self.fireworks.burst(&burst, self.firework_camera);
            self.sounds.push((event.to_owned(), burst.position, volume, pitch));
        }
    }

    pub fn receive(
        &mut self,
        snapshot: &mut EntitySnapshot,
        changes: &[(BlockPos, Option<Block>)],
    ) {
        self.thrown = std::mem::take(&mut snapshot.thrown_views);
        self.eyes.receive(
            std::mem::take(&mut snapshot.eyes),
            &mut self.portal,
            &mut self.item_crumbs,
        );
        self.clouds = std::mem::take(&mut snapshot.clouds);
        self.swirls = std::mem::take(&mut snapshot.swirls);
        self.tridents = std::mem::take(&mut snapshot.tridents);
        self.crystals = std::mem::take(&mut snapshot.crystals);
        self.gateways
            .receive(std::mem::take(&mut snapshot.gateways), self.ticks);
        self.dragons = std::mem::take(&mut snapshot.dragons);
        self.drawn_dragons.receive(&self.dragons);
        self.dragon_fireballs = std::mem::take(&mut snapshot.dragon_fireballs);
        self.drawn_fireballs.receive(&self.dragon_fireballs);
        for fireball in &self.dragon_fireballs {
            self.ambient.breath_trail(fireball.position);
        }
        self.frames = std::mem::take(&mut snapshot.frames);
        self.bursts(snapshot);
        self.orbs = snapshot.orbs.clone();
        self.falling.receive(
            std::mem::take(&mut snapshot.falling),
            std::mem::take(&mut snapshot.tnt),
            changes,
            self.ticks,
        );
    }

    pub fn show_items(&mut self, items: Vec<ServerItem>, recipes: &RecipeBook) {
        self.burning_items = items
            .iter()
            .filter(|item| item.on_fire)
            .map(|item| mirrored(item.id))
            .collect();
        self.world_items.entities = items
            .into_iter()
            .map(|item: ServerItem| {
                let mut stack = ItemStack::new(&item.item, item.count.clamp(0, 255) as u8);
                stack.components = item
                    .components
                    .as_deref()
                    .and_then(|c| serde_json::from_str(c).ok());
                stack.max = stack.max.min(recipes.max_stack(&item.item));
                ItemEntity {
                    entity_id: mirrored(item.id),
                    stack,
                    position: DVec3::from_array(item.position),
                    previous_position: DVec3::from_array(item.previous_position),
                    velocity: DVec3::from_array(item.velocity),
                    age: item.age.max(0) as u32,
                    bob_offset: bob_offset(item.id),
                    pickup_delay: item.pickup_delay.clamp(0, i32::from(u16::MAX)) as u16,
                    on_ground: item.on_ground,
                }
            })
            .collect();
    }

    pub fn show_mobs(&mut self, shown: Vec<ServerMob>) {
        self.shown = shown.iter().map(|mob| mob.id).collect();
        for (feet, width, height) in self.client.receive(shown) {
            self.poof.spawn(feet, width, height);
        }
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn animate_tick<S: Scene + ?Sized>(
        &mut self,
        scene: &S,
        feet: [f64; 3],
    ) -> Vec<minecraft_terrain::display_ticks::AmbientSound> {
        let at = (
            feet[0].floor() as i32,
            feet[1].floor() as i32,
            feet[2].floor() as i32,
        );
        let tick = minecraft_terrain::display_ticks::animate_tick(scene, at, &mut self.random);
        for (start, motion) in tick.portal_particles {
            self.portal.emit(start, motion);
        }
        for (kind, start, motion) in tick.particles {
            self.ambient.add(kind, start, motion);
        }
        tick.sounds
    }

    pub(crate) fn present(&mut self, effects: minecraft_terrain::display_ticks::DisplayTick) {
        for sound in effects.sounds {
            self.sounds.push((
                sound.event.to_owned(),
                DVec3::from_array(sound.at),
                sound.volume,
                sound.pitch,
            ));
        }
        for (kind, at, motion) in effects.particles {
            self.ambient.add(kind, at, motion);
        }
    }

    pub fn level_event(
        &mut self,
        scene: &HandcraftedScene,
        id: i32,
        pos: (i32, i32, i32),
        data: i32,
    ) {
        use crate::heard::{INSTANT_POTION_EVENT, SPLASH_POTION_EVENT};
        if id == SPLASH_POTION_EVENT || id == INSTANT_POTION_EVENT {
            let at = DVec3::new(
                f64::from(pos.0) + 0.5,
                f64::from(pos.1),
                f64::from(pos.2) + 0.5,
            );
            self.spell
                .burst(at, data as u32, id == INSTANT_POTION_EVENT);
            self.potion_crumbs
                .push(minecraft_terrain::spell_particles::break_center(at));
            return;
        }
        let is_a = minecraft_terrain::level_events::class_test(scene);
        let effects = minecraft_terrain::level_events::level_event(
            id,
            pos,
            data,
            scene,
            &is_a,
            &mut self.random,
        );
        self.present(effects);
    }

    pub fn explosion(&mut self, center: DVec3, radius: f32, blocks: usize, interacts: bool) {
        self.ambient.explosion(center, radius, blocks, interacts);
    }

    pub fn track_body(&mut self, key: u128, feet: [f64; 3], width: f64, height: f64) {
        let feet = DVec3::from_array(feet);
        let body = minecraft_terrain::ambient_particles::Body {
            feet,
            width,
            height,
        };
        self.bodies.insert(key, body);
    }

    pub fn hold_rope(&mut self, key: u128, feet: [f64; 3], yaw: f32, height: f64) {
        let holder = minecraft_terrain::leash_render::RopeHolder {
            feet: DVec3::from_array(feet),
            yaw,
            height,
        };
        self.rope_holders.insert(key, holder);
    }

    pub fn totem_pop(&mut self, key: u128, feet: [f64; 3], width: f64, height: f64) {
        self.track_body(key, feet, width, height);
        let body = self.bodies[&key];
        self.ambient.totem_emitter(key, body);
    }

    pub fn held_item_mesh(
        &mut self,
        posed: &[(glam::Mat4, String, bool)],
        light_at: glam::Vec3,
        packs: &PackStack,
        atlas: &Atlas,
        light: &SkyLight,
    ) -> minecraft_terrain::mesh::ChunkMesh {
        let mut mesh = minecraft_terrain::mesh::ChunkMesh::default();
        for (pose, id, glint) in posed {
            let start = mesh.vertices.len() as u32;
            let at = [(*pose, light_at, id.clone())];
            let _ = self
                .items
                .append_posed_items(&mut mesh, &at, packs, atlas, light);
            if *glint {
                mesh.glint.push(start..mesh.vertices.len() as u32);
            }
        }
        mesh
    }

    pub fn powder_snow_flake(&mut self, feet: [f64; 3], block_y: i32) {
        let mut across = || f64::from((self.random.next_float() * 2.0 - 1.0) * 0.083_333_336);
        let motion = DVec3::new(across(), f64::from(0.05_f32), across());
        let at = DVec3::new(feet[0], f64::from(block_y + 1), feet[2]);
        self.ambient
            .add(minecraft_terrain::ambient_particles::Kind::Snowflake, at, motion);
    }

    pub fn snowflakes(
        &mut self,
        world: &impl minecraftoss_player::World,
        feet: [f64; 3],
        height: f32,
    ) {
        for block_y in crate::survival::powder_snow_flakes(world, feet, height) {
            if self.random.next_float() < 0.5 {
                self.powder_snow_flake(feet, block_y);
            }
        }
    }

    pub fn rain_tick(
        &mut self,
        scene: &HandcraftedScene,
        camera: [f64; 3],
        rain_level: f32,
    ) -> Vec<minecraft_terrain::display_ticks::AmbientSound> {
        self.ambient
            .rain_tick(scene, DVec3::from_array(camera), rain_level)
    }

    pub fn weather_mesh(
        &self,
        scene: &HandcraftedScene,
        camera: [f64; 3],
        light: &SkyLight,
        (rain_level, partial): (f32, f32),
    ) -> (Vec<minecraft_terrain::weather::WeatherVertex>, Vec<u32>) {
        if rain_level <= 0.0 {
            return Default::default();
        }
        let camera = DVec3::from_array(camera);
        let radius = minecraft_terrain::rain::WEATHER_RADIUS;
        let columns = minecraft_terrain::weather::columns(
            camera,
            radius,
            self.ticks as i64,
            partial,
            |x, z| minecraft_terrain::rain::motion_blocking_height(scene, x, z),
            |pos| self.ambient.precipitation_at(scene, pos),
            |pos| (light.get(pos), light.get_block(pos)),
        );
        minecraft_terrain::weather::mesh(&columns, camera, radius, rain_level)
    }

    pub fn mob_on_ray(&self, eye: DVec3, look: DVec3, reach: f64) -> Option<(u64, f64)> {
        self.world
            .mob_on_ray_except(eye, look, reach, None)
            .map(|(hit, distance)| (hit.id(), distance))
    }

    pub fn random_float(&mut self) -> f32 {
        self.random.next_float()
    }

    pub fn partial(&self) -> f32 {
        (self.clock / self.tick_seconds).clamp(0.0, 1.0) as f32
    }

    pub fn ride_partial(&self) -> f32 {
        (self.ride_clock / self.tick_seconds).clamp(0.0, 1.0) as f32
    }

    pub fn drawn_dragons(&self) -> Vec<minecraft_terrain::ender_dragon::DragonView> {
        self.drawn_dragons.views()
    }

    fn dragon_voices(&mut self, flaps: &[(u64, DVec3, bool)]) {
        use minecraft_terrain::ender_dragon::{FLAP_SOUND, GROWL_SOUND, SOUND_VOLUME};
        let drawn = self.drawn_dragons.views();
        self.growls
            .retain(|id, _| drawn.iter().any(|dragon| dragon.id == *id));
        for dragon in &drawn {
            let Some(&(_, _, flapped)) = flaps.iter().find(|&&(id, _, _)| id == dragon.id) else {
                continue;
            };
            if flapped {
                let pitch = 0.8 + self.random.next_float() * 0.3;
                self.sounds
                    .push((FLAP_SOUND.to_owned(), dragon.position, SOUND_VOLUME, pitch));
            }
            if dragon.phase == minecraft_terrain::ender_dragon::Phase::SittingAttacking.id() {
                let pitch = 0.8 + self.random.next_float() * 0.3;
                self.sounds
                    .push((GROWL_SOUND.to_owned(), dragon.position, 2.5, pitch));
            }
            if dragon.sitting {
                continue;
            }
            let growl = self.growls.entry(dragon.id).or_insert(100);
            *growl -= 1;
            if *growl < 0 {
                let pitch = 0.8 + self.random.next_float() * 0.3;
                self.sounds
                    .push((GROWL_SOUND.to_owned(), dragon.position, 2.5, pitch));
                *growl = 200 + self.random.next_int(200) as i32;
            }
        }
    }

    pub fn boxes(&self) -> Vec<(u64, [f64; 6])> {
        let w = &self.world;
        let mut out = Vec::new();
        let mut add = |hit: MobHit, body: &minecraftoss_entities::movement::Body| {
            let p = body.position;
            let half = f64::from(body.width) * 0.5;
            out.push((
                encode(hit),
                [
                    p.x - half,
                    p.y,
                    p.z - half,
                    p.x + half,
                    p.y + f64::from(body.height),
                    p.z + half,
                ],
            ));
        };
        for e in w.bats().iter().filter(|e| e.bat.health > 0.0) {
            add(MobHit::Bat(e.id), &e.bat.body);
        }
        for e in w.zombies().iter().filter(|e| e.zombie.health > 0.0) {
            add(MobHit::Zombie(e.id), &e.zombie.body);
        }
        for e in w.skeletons().iter().filter(|e| e.skeleton.health > 0.0) {
            add(MobHit::Skeleton(e.id), &e.skeleton.body);
        }
        for e in w
            .creepers()
            .iter()
            .filter(|e| e.creeper.health > 0.0 && !e.creeper.exploded)
        {
            add(MobHit::Creeper(e.id), &e.creeper.body);
        }
        for e in w.spiders().iter().filter(|e| e.spider.health > 0.0) {
            add(MobHit::Spider(e.id), &e.spider.body);
        }
        for e in w.slimes().iter().filter(|e| e.slime.health > 0.0) {
            add(MobHit::Slime(e.id), &e.slime.body);
        }
        for e in w.endermen().iter().filter(|e| e.enderman.health > 0.0) {
            add(MobHit::Enderman(e.id), &e.enderman.body);
        }
        for e in w.witches().iter().filter(|e| e.witch.health > 0.0) {
            add(MobHit::Witch(e.id), &e.witch.body);
        }
        for e in w.illagers().iter().filter(|e| e.illager.health > 0.0) {
            add(MobHit::Illager(e.id), &e.illager.body);
        }
        for e in w.vexes().iter().filter(|e| e.vex.health > 0.0) {
            add(MobHit::Vex(e.id), &e.vex.body);
        }
        for e in w.ravagers().iter().filter(|e| e.ravager.health > 0.0) {
            add(MobHit::Ravager(e.id), &e.ravager.body);
        }
        for e in w.withers().iter().filter(|e| e.wither.health > 0.0) {
            add(MobHit::Wither(e.id), &e.wither.body);
        }
        for e in w.iron_golems().iter().filter(|e| e.golem.health > 0.0) {
            add(MobHit::IronGolem(e.id), &e.golem.body);
        }
        for e in w.wolves().iter().filter(|e| e.wolf.health > 0.0) {
            add(MobHit::Wolf(e.id), &e.wolf.body);
        }
        for e in w.felines().iter().filter(|e| e.feline.health > 0.0) {
            add(MobHit::Feline(e.id), &e.feline.body);
        }
        for e in w.wandering_traders().iter().filter(|e| e.trader.health > 0.0) {
            add(MobHit::WanderingTrader(e.id), &e.trader.body);
        }
        for e in w.snow_golems().iter().filter(|e| e.golem.health > 0.0) {
            add(MobHit::SnowGolem(e.id), &e.golem.body);
        }
        for e in w.villagers().iter().filter(|e| e.villager.health > 0.0) {
            add(MobHit::Villager(e.id), &e.villager.body);
        }
        for e in w.cows().iter().filter(|e| e.cow.health > 0.0) {
            let hit = if e.mooshroom.is_some() {
                MobHit::Mooshroom(e.id)
            } else {
                MobHit::Cow(e.id)
            };
            add(hit, &e.cow.body);
        }
        for e in w.sheep().iter().filter(|e| e.health > 0.0) {
            add(MobHit::Sheep(e.id), &e.body);
        }
        for e in w.pigs().iter().filter(|e| e.pig.health > 0.0) {
            add(MobHit::Pig(e.id), &e.pig.body);
        }
        for e in w.chickens().iter().filter(|e| e.chicken.health > 0.0) {
            add(MobHit::Chicken(e.id), &e.chicken.body);
        }
        for e in w.ghasts().iter().filter(|e| e.ghast.health > 0.0) {
            add(MobHit::Ghast(e.id), &e.ghast.body);
        }
        for e in w.blazes().iter().filter(|e| e.blaze.health > 0.0) {
            add(MobHit::Blaze(e.id), &e.blaze.body);
        }
        for e in w.striders().iter().filter(|e| e.strider.health > 0.0) {
            add(MobHit::Strider(e.id), &e.strider.body);
        }
        for e in w.hoglins().iter().filter(|e| e.hoglin.health > 0.0) {
            add(MobHit::Hoglin(e.id), &e.hoglin.body);
        }
        for e in w.piglins().iter().filter(|e| e.piglin.health > 0.0) {
            add(MobHit::Piglin(e.id), &e.piglin.body);
        }
        for e in w.shulkers().iter().filter(|e| e.shulker.health > 0.0) {
            let (low, high) = e.shulker.bounding_box();
            out.push((
                encode(MobHit::Shulker(e.id)),
                [low.x, low.y, low.z, high.x, high.y, high.z],
            ));
        }
        for crystal in &self.crystals {
            let (low, high) = minecraft_terrain::end_crystal::pick_box(crystal.position);
            out.push((
                CRYSTAL_KEY | (crystal.id & ((1 << 56) - 1)),
                [low.x, low.y, low.z, high.x, high.y, high.z],
            ));
        }
        out
    }

    pub fn shot_boxes(&self, riding: Option<u64>) -> Vec<(u64, [f64; 6])> {
        let mut out = self.boxes();
        out.retain(|(key, _)| riding.is_none() || decode(*key).map(|hit| hit.id()) != riding);
        for e in self
            .world
            .fireballs()
            .iter()
            .filter(|e| e.fireball.alive && e.fireball.kind.redirectable())
        {
            let (min, max) =
                minecraftoss_entities::fireball::pick_box(e.fireball.kind, e.fireball.position);
            out.push((
                FIREBALL_KEY | (e.id & ((1 << 56) - 1)),
                [min.x, min.y, min.z, max.x, max.y, max.z],
            ));
        }
        for e in self
            .world
            .shulker_bullets()
            .iter()
            .filter(|e| e.bullet.alive)
        {
            let (min, max) = e.bullet.bounds();
            out.push((
                SHULKER_BULLET_KEY | (e.id & ((1 << 56) - 1)),
                [min.x, min.y, min.z, max.x, max.y, max.z],
            ));
        }
        for dragon in self.dragons.iter().filter(|dragon| dragon.health > 0.0) {
            for (part, &feet) in dragon.parts.iter().enumerate() {
                let (low, high) = minecraft_terrain::ender_dragon::part_box(feet, part);
                out.push((
                    dragon_part_key(dragon.id, part),
                    [low.x, low.y, low.z, high.x, high.y, high.z],
                ));
            }
        }
        for frame in &self.frames {
            let (low, high) =
                minecraft_terrain::item_frame::hanging_box(frame.position, frame.facing);
            out.push((
                FRAME_KEY | (frame.id & ((1 << 56) - 1)),
                [low.x, low.y, low.z, high.x, high.y, high.z],
            ));
        }
        out
    }

    pub fn tick_scene(&mut self, scene: &HandcraftedScene, ticks: u32) {
        for _ in 0..ticks {
            for rocket in self.thrown.iter().filter(|view| minecraft_terrain::firework_rocket::is_rocket(view)) {
                self.fireworks.trail(rocket.position, rocket.position.y - rocket.previous.y);
            }
            self.fireworks.tick(|cell| !minecraftoss_player::World::collision_boxes(scene, cell).is_empty());
            self.poof.tick(scene);
            let bodies = &self.bodies;
            self.ambient.tick_emitters(|key| bodies.get(&key).copied());
            self.ambient.tick(scene);
            for entity in self
                .world
                .shulker_bullets()
                .iter()
                .filter(|e| e.bullet.alive)
            {
                self.ambient.add(
                    minecraft_terrain::ambient_particles::Kind::EndRod,
                    entity.bullet.trail(),
                    DVec3::ZERO,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn meshes(
        &mut self,
        scene: &HandcraftedScene,
        packs: &PackStack,
        atlas: &Atlas,
        light: &SkyLight,
        forward: Vec3,
        camera: DVec3,
        sky_darken: u8,
        cell_settled: &dyn Fn((i32, i32, i32)) -> bool,
        placed_items: &[minecraft_terrain::mesh::HeldItem],
    ) -> MobMeshes {
        use minecraft_terrain::*;
        let missing: Vec<ServerMob> = server_mobs(&self.world)
            .into_iter()
            .filter(|mob| self.shown.contains(&mob.id))
            .collect();
        self.client.spawn_missing(&missing);
        let carriers = self.world.player_carriers();
        let ride_partial = self.ride_partial();
        self.client.set_ride_partial(&carriers, ride_partial);
        let w = &self.world;
        let poses = &self.client;
        let partial = (self.clock / self.tick_seconds).clamp(0.0, 1.0) as f32;
        let mut out = MobMeshes::default();
        cow_render::append_cows(
            &mut out.models,
            w.cows().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        sheep_render::append_sheep(
            &mut out.models,
            w.sheep().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        pig_render::append_pigs(
            &mut out.models,
            w.pigs().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        chicken_render::append_chickens(
            &mut out.models,
            w.chickens().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        bat_render::append_bats(
            &mut out.culled,
            w.bats().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        let zombie_items = zombie_render::append_zombies(
            &mut out.models,
            w.zombies().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        creeper_render::append_creepers(
            &mut out.models,
            w.creepers().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        spider_render::append_spiders(
            &mut out.models,
            w.spiders().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        let skeleton_items = skeleton_render::append_skeletons(
            &mut out.models,
            w.skeletons().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        let villager_items = villager_render::append_villagers(
            &mut out.models,
            w.villagers().iter(),
            poses,
            &|pos| Scene::block(scene, pos).and_then(|b| b.properties.get("facing").cloned()),
            atlas,
            light,
            partial,
        );
        let trader_items = villager_render::append_wandering_traders(
            &mut out.models,
            w.wandering_traders().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        horse_render::append_horses(
            &mut out.models,
            &mut out.translucent,
            w.cows().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        llama_render::append_llamas(
            &mut out.models,
            w.cows().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        slime_render::append_slimes(
            &mut out.models,
            &mut out.translucent,
            w.slimes().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        magma_cube_render::append_magma_cubes(
            &mut out.models,
            w.slimes().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        orb_render::append_orbs(
            &mut out.translucent,
            &self.orbs,
            &self.orb_pickups,
            partial,
            atlas,
            forward,
            light,
        );
        let carried = enderman_render::append_endermen(
            &mut out.models,
            w.endermen().iter(),
            poses,
            atlas,
            light,
            partial,
            self.ticks.rotate_left(32) ^ (partial.to_bits() as u64),
        );
        let witch_items = witch_render::append_witches(
            &mut out.models,
            w.witches().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        ghast_render::append_ghasts(
            &mut out.models,
            w.ghasts().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        end_crystal_render::append_crystals(&mut out.models, &self.crystals, atlas, light, partial);
        let drawn_dragons = self.drawn_dragons.views();
        dragon_render::append_dragons(&mut out.models, &drawn_dragons, atlas, light, partial);
        dragon_render::append_beams(
            &mut out.models,
            &drawn_dragons,
            &self.crystals,
            atlas,
            light,
            partial,
        );
        dragon_render::append_rays(&mut out.translucent, &drawn_dragons, atlas, partial);
        end_gateway_render::append_beams(
            &mut out.models,
            &mut out.translucent,
            self.gateways.views(),
            atlas,
            camera,
            scene.vertical_range().end - 1,
            w.game_time(),
            partial,
        );
        blaze_render::append_blazes(
            &mut out.models,
            w.blazes().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        self.spawners.append(
            &mut out.models,
            &mut out.culled,
            &mut out.translucent,
            scene,
            atlas,
            light,
            camera,
            partial,
        );
        strider_render::append_striders(
            &mut out.models,
            w.striders().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        hoglin_render::append_hoglins(
            &mut out.models,
            w.hoglins().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        let piglin_items = piglin_render::append_piglins(
            &mut out.models,
            w.piglins().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        let illager_items = minecraft_terrain::illager_render::append_illagers(
            &mut out.models,
            w.illagers().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        let vex_items = minecraft_terrain::vex_render::append_vexes(
            &mut out.models,
            w.vexes().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        minecraft_terrain::ravager_render::append_ravagers(
            &mut out.models,
            w.ravagers().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        minecraft_terrain::wither_render::append_withers(
            &mut out.models,
            &mut out.translucent,
            w.withers().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        for e in w.illagers().iter().filter(|e| w.glowing(e.id)) {
            let mut mob = minecraft_terrain::mesh::ChunkMesh::default();
            let _ = minecraft_terrain::illager_render::append_illagers(&mut mob, std::iter::once(e), poses, atlas, light, partial);
            minecraft_terrain::glow_render::append_glowing(&mut out.glow, &mut out.glow_cover, &mob);
        }
        for e in w.ravagers().iter().filter(|e| w.glowing(e.id)) {
            let mut mob = minecraft_terrain::mesh::ChunkMesh::default();
            minecraft_terrain::ravager_render::append_ravagers(&mut mob, std::iter::once(e), poses, atlas, light, partial);
            minecraft_terrain::glow_render::append_glowing(&mut out.glow, &mut out.glow_cover, &mob);
        }
        for e in w.witches().iter().filter(|e| w.glowing(e.id)) {
            let mut mob = minecraft_terrain::mesh::ChunkMesh::default();
            let _ = witch_render::append_witches(&mut mob, std::iter::once(e), poses, atlas, light, partial);
            minecraft_terrain::glow_render::append_glowing(&mut out.glow, &mut out.glow_cover, &mob);
        }
        shulker_render::append_shulkers(
            &mut out.models,
            w.shulkers().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        let near = |at: DVec3| (at.x - camera.x).hypot(at.z - camera.z) <= FLAME_RANGE;
        shulker_render::append_bullets(
            &mut out.models,
            &mut out.translucent,
            w.shulker_bullets()
                .iter()
                .filter(|e| near(e.bullet.position)),
            atlas,
            partial,
        );
        llama_render::append_spits(
            &mut out.models,
            w.llama_spits().iter().filter(|e| near(e.spit.position)),
            atlas,
            light,
            partial,
        );
        minecraft_terrain::evoker_fangs_render::append_fangs(
            &mut out.models,
            w.evoker_fangs().iter().filter(|e| near(e.fangs.position)),
            atlas,
            light,
            partial,
        );
        let poppies = golem_render::append_iron_golems(
            &mut out.models,
            w.iron_golems().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        let pumpkins = minecraft_terrain::snow_golem_render::append_snow_golems(
            &mut out.models,
            w.snow_golems().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        wolf_render::append_wolves(
            &mut out.models,
            w.wolves().iter(),
            poses,
            atlas,
            light,
            partial,
            w.game_time(),
        );
        cat_render::append_felines(
            &mut out.models,
            w.felines().iter(),
            poses,
            atlas,
            light,
            partial,
        );
        leash_render::append_knots(&mut out.models, &w.knots(), atlas, light);
        leash_render::append_ropes(
            &mut out.models,
            w,
            poses,
            &self.rope_holders,
            atlas,
            light,
            partial,
        );
        flame_render::append_flames(
            &mut out.items,
            w.burning()
                .into_iter()
                .filter(|(_, now, _, _)| near(*now))
                .chain(
                    self.world_items
                        .entities
                        .iter()
                        .filter(|e| self.burning_items.contains(&e.entity_id))
                        .map(|e| (e.previous_position, e.position, 0.25, 0.25)),
                )
                .map(|(previous, now, width, height)| {
                    (previous.lerp(now, f64::from(partial)), width, height)
                }),
            atlas,
            forward,
            light,
        );
        witch_render::append_potions(
            &mut out.items,
            w.potions()
                .iter()
                .filter(|p| p.potion.alive && near(p.potion.position))
                .map(|p| {
                    (
                        p,
                        p.previous_position
                            .lerp(p.potion.position, f64::from(partial)),
                    )
                }),
            atlas,
            forward,
            light,
        );
        minecraft_terrain::dragon_fireball::append_fireballs(
            &mut out.items,
            &self.drawn_fireballs.views(),
            atlas,
            forward,
            partial,
        );
        ghast_render::append_fireballs(
            &mut out.items,
            w.fireballs()
                .iter()
                .filter(|f| f.fireball.alive && near(f.fireball.position))
                .map(|f| {
                    (
                        f,
                        f.previous_position
                            .lerp(f.fireball.position, f64::from(partial)),
                    )
                }),
            atlas,
            forward,
        );
        minecraft_terrain::wither_render::append_skulls(
            &mut out.models,
            w.fireballs()
                .iter()
                .filter(|f| f.fireball.alive && near(f.fireball.position))
                .map(|f| {
                    (
                        f,
                        f.previous_position
                            .lerp(f.fireball.position, f64::from(partial)),
                    )
                }),
            atlas,
            light,
        );
        if let Ok(items) = self
            .items
            .mesh(&self.world_items, packs, atlas, light, partial)
        {
            let base = out.items.vertices.len() as u32;
            out.items.vertices.extend(items.vertices);
            out.items
                .indices
                .extend(items.indices.iter().map(|i| i + base));
            out.items
                .glint
                .extend(items.glint.iter().map(|r| r.start + base..r.end + base));
        }
        self.falling.settle(scene, cell_settled, self.ticks);
        let moving = self.falling.moving_blocks(scene, partial);
        let _ =
            self.items
                .append_moving_blocks(&mut out.items, &moving, scene, packs, atlas, light);
        self.lids.settle(scene, cell_settled);
        let lids = self.lids.posed(scene, partial);
        let _ = self
            .items
            .append_open_lids(&mut out.items, &lids, scene, packs, atlas, light);
        let frames: Vec<_> = self
            .frames
            .iter()
            .filter(|frame| minecraft_terrain::item_frame::tracked(frame, camera))
            .cloned()
            .collect();
        let _ = self
            .items
            .append_frames(&mut out.items, &frames, packs, atlas, light);
        let _ = self.items.append_primed_tnt(
            &mut out.items,
            &self.falling.primed_tnt(partial),
            packs,
            atlas,
            light,
        );
        minecraft_terrain::thrown::append_views(
            &mut out.items,
            &self.thrown,
            partial,
            camera,
            atlas,
            forward,
            light,
        );
        minecraft_terrain::eye_of_ender::append_eyes(
            &mut out.items,
            self.eyes.views(),
            partial,
            camera,
            atlas,
            forward,
        );
        minecraft_terrain::trident::append_tridents(
            &mut out.items,
            &self.tridents,
            partial,
            atlas,
            light,
        );
        self.poof
            .append_mesh(&mut out.items, atlas, forward, partial, light);
        self.ambient.append_mesh(
            &mut out.items,
            &mut out.translucent,
            atlas,
            forward,
            partial,
            light,
        );
        self.portal
            .append_mesh(&mut out.items, atlas, forward, partial, light);
        self.spell
            .append_mesh(&mut out.translucent, atlas, forward, partial, light);
        self.firework_camera = camera;
        self.fireworks
            .append_mesh(&mut out.translucent, atlas, forward, partial, light);
        let held: Vec<_> = skeleton_items
            .into_iter()
            .chain(zombie_items)
            .chain(villager_items)
            .chain(trader_items)
            .chain(witch_items)
            .chain(piglin_items)
            .chain(illager_items)
            .chain(vex_items)
            .chain(placed_items.iter().cloned())
            .collect();
        let _ = self
            .items
            .append_held_items(&mut out.items, &held, packs, atlas, light);
        let carried: Vec<_> = carried.into_iter().chain(poppies).chain(pumpkins).collect();
        let _ = self
            .items
            .append_posed_blocks(&mut out.items, &carried, packs, atlas, light);
        let mut casters = client_mobs::shadow_casters(w, poses, camera, partial);
        casters.extend(orb_render::shadow_casters(&self.orbs, camera, partial));
        casters.extend(self.falling.shadow_casters(scene, camera, partial));
        let raw = |pos: (i32, i32, i32)| {
            light
                .get(pos)
                .saturating_sub(sky_darken)
                .max(light.get_block(pos))
        };
        if let Ok(shadows) = mesh::entity_shadows(&casters, scene, atlas, &raw, 0.0) {
            out.shadows = shadows;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_terrain::ender_dragon::{DragonView, FLAP_SOUND, GROWL_SOUND};

    fn dragon(tick_count: i32) -> DragonView {
        let flap_time = tick_count as f32 * 0.1;
        DragonView {
            id: 3,
            uuid: [0; 4],
            position: DVec3::new(0.0, 80.0, 0.0),
            previous_position: DVec3::new(0.0, 80.0, 0.0),
            y_rot: 0.0,
            flap_time,
            o_flap_time: flap_time - 0.1,
            history: Vec::new(),
            health: 200.0,
            hurt_time: 0,
            death_time: 0,
            sitting: false,
            phase: 0,
            parts: [DVec3::new(0.0, 80.0, 0.0); 8],
            tick_count,
            nearest_crystal: None,
        }
    }

    fn voices(frozen: bool) -> usize {
        let mut view = EntityView::new(7);
        for tick in 0..400 {
            let mut snapshot = EntitySnapshot {
                dragons: vec![dragon(if frozen { 0 } else { tick })],
                ..Default::default()
            };
            view.receive(&mut snapshot, &[]);
            view.advance(
                0.05,
                crate::tick_rate::ClientTiming::default(),
                &[],
                DVec3::ZERO,
            );
        }
        view.sounds
            .iter()
            .filter(|(sound, ..)| sound == FLAP_SOUND || sound == GROWL_SOUND)
            .count()
    }

    #[test]
    fn a_dragon_frozen_on_the_host_makes_no_flap_or_growl_on_the_client() {
        assert!(voices(false) > 0);
        assert_eq!(voices(true), 0);
    }

    #[test]
    fn a_wither_smokes_from_its_three_heads_and_glows_while_it_charges_or_is_powered() {
        let mut view = EntityView::new(7);
        let id = view.world.spawn_wither(minecraftoss_entities::wither::Wither::new(DVec3::new(0.5, 64.0, 0.5)), true);
        for _ in 0..10 {
            view.wither_effects();
        }
        assert_eq!(view.ambient.len(), 30, "a puff of smoke at every head each tick");
        assert_eq!(view.spell.len(), 0, "a healthy wither does not glow");
        view.world.wither_mut(id).unwrap().wither.make_invulnerable();
        for _ in 0..40 {
            view.wither_effects();
        }
        let glows = view.spell.len();
        assert!((120 + 15..=120 + 50).contains(&glows), "three charge sparks a tick and the powered glow one head in four: {glows}");
        view.world.wither_mut(id).unwrap().wither.health = 0.0;
        let before = (view.ambient.len(), view.spell.len());
        view.wither_effects();
        assert_eq!((view.ambient.len(), view.spell.len()), before, "a dead wither stops");
    }

    #[test]
    fn a_stunned_ravager_swirls_now_and_then_and_its_roar_puffs_once() {
        let mut view = EntityView::new(7);
        let ravager = minecraftoss_entities::ravager::Ravager::new(DVec3::new(0.5, 64.0, 0.5));
        let id = view.world.spawn_ravager(ravager, true);
        view.world.ravager_mut(id).unwrap().ravager.stunned_tick = 30;
        for _ in 0..60 {
            view.ravager_effects();
        }
        let swirls = view.spell.len();
        assert!((3..=25).contains(&swirls), "about one tick in six: {swirls}");
        assert_eq!(view.ambient.len(), 0);
        let mut puffs = Vec::new();
        for roar in [15, 11, 10, 9, 5, 0, 10] {
            let entity = view.world.ravager_mut(id).unwrap();
            entity.ravager.stunned_tick = 0;
            entity.ravager.roar_tick = roar;
            view.ravager_effects();
            puffs.push(view.ambient.len());
        }
        assert_eq!(puffs, vec![0, 0, 40, 40, 40, 40, 80]);
        assert_eq!(view.spell.len(), swirls, "no swirls once the stun is over");
    }

    #[test]
    fn a_firework_burst_bangs_and_sparkles_for_whoever_sees_it() {
        use minecraft_terrain::firework_rocket::{Burst, BLAST_FAR_SOUND, BLAST_SOUND};
        let mut view = EntityView::new(3);
        let burst = Burst { position: DVec3::new(0.5, 80.0, 0.5), velocity: DVec3::new(0.0, 0.9, 0.0), colors: vec![4312372] };
        let mut snapshot = EntitySnapshot { firework_bursts: vec![burst.clone()], ..EntitySnapshot::default() };
        view.bursts(&mut snapshot);
        assert!(snapshot.firework_bursts.is_empty());
        assert_eq!(view.fireworks.len(), minecraft_terrain::firework_particles::BURST_SPARKS);
        assert_eq!(view.sounds.last().map(|(event, at, volume, _)| (event.as_str(), *at, *volume)), Some((BLAST_FAR_SOUND, burst.position, 20.0)));
        view.firework_camera = DVec3::new(0.5, 75.0, 0.5);
        let mut near = EntitySnapshot { firework_bursts: vec![burst], ..EntitySnapshot::default() };
        view.bursts(&mut near);
        assert_eq!(view.sounds.last().map(|(event, ..)| event.as_str()), Some(BLAST_SOUND));
    }

    #[test]
    fn a_heart_puff_draws_seven_hearts_on_the_mob() {
        use minecraft_terrain::ambient_particles::Kind;
        use minecraft_terrain::mob_particles::{MobPuff, PuffKind};
        let mut view = EntityView::new(11);
        let feet = DVec3::new(4.5, 64.0, -3.5);
        let puff = |kind| MobPuff { kind, count: 7, feet, width: 0.6, height: 0.85, lift: 0.5, range: 10 };
        let mut snapshot = EntitySnapshot { mob_puffs: vec![puff(PuffKind::Heart), puff(PuffKind::Smoke)], ..EntitySnapshot::default() };
        view.bursts(&mut snapshot);
        assert!(snapshot.mob_puffs.is_empty());
        let drawn: Vec<(Kind, DVec3)> = view.ambient.kinds_at().collect();
        let count = |kind| drawn.iter().filter(|(drawn, _)| *drawn == kind).count();
        assert_eq!((count(Kind::Heart), count(Kind::Smoke), drawn.len()), (7, 7, 14));
        for (_, at) in drawn {
            assert!((at.x - feet.x).abs() <= 0.6 && (at.z - feet.z).abs() <= 0.6, "{at}");
            assert!(at.y >= feet.y + 0.5 && at.y <= feet.y + 0.85 + 0.5, "{at}");
        }
    }

    #[test]
    fn a_ridden_strider_keeps_its_own_clock_while_the_host_is_frozen() {
        use minecraftoss_entities::strider::Strider;
        use minecraftoss_entities::world::Rider;
        let mut view = EntityView::new(5);
        let mut saddled = Strider::new(DVec3::new(0.5, 64.0, 0.5));
        saddled.saddled = true;
        let ridden = view.world.spawn_strider(saddled, true);
        let idle = view.world.spawn_strider(Strider::new(DVec3::new(6.5, 64.0, 0.5)), true);
        assert!(view.world.force_riding(Rider::Player(0), ridden));
        view.client.receive(server_mobs(&view.world));
        let normal = crate::tick_rate::ClientTiming { tick_seconds: 0.25, runs_normally: true };
        let frozen = crate::tick_rate::ClientTiming { runs_normally: false, ..normal };
        let quarter = normal.tick_seconds / 4.0;
        view.advance(quarter, normal, &[], DVec3::ZERO);
        assert_eq!(view.ride_partial(), view.partial());
        let ticks = |view: &EntityView, id: u64| view.client.get(id).unwrap().tick_count;
        let (ridden_ticks, idle_ticks) = (ticks(&view, ridden), ticks(&view, idle));
        let held = view.partial();
        let mut seen = Vec::new();
        for _ in 0..7 {
            assert!(!view.advance(quarter, frozen, &[], DVec3::ZERO));
            assert_eq!(view.partial(), held, "the level's partial tick stays put");
            seen.push(view.ride_partial());
        }
        assert_eq!(seen, [0.5, 0.75, 0.0, 0.25, 0.5, 0.75, 0.0]);
        assert_eq!(ticks(&view, ridden), ridden_ticks + 2, "the carrier ticks once a tick");
        assert_eq!(ticks(&view, idle), idle_ticks, "the empty strider stays frozen");
        view.advance(quarter, normal, &[], DVec3::ZERO);
        assert_eq!(view.ride_partial(), view.partial(), "they agree once the game runs again");
    }
}
