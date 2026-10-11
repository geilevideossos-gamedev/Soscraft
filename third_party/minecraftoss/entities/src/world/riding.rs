use super::*;
use crate::strider::{self, Strider};
use crate::strider_ai::StriderGoalEffect;

pub const NON_CONTROLLING_RIDERS: &[&str] = &[
    "minecraft:slime",
    "minecraft:magma_cube",
    "minecraft:sulfur_cube",
];
pub const PLAYER_VEHICLE_ATTACHMENT: f64 = 0.6;
pub const PLAYER_WIDTH: f32 = 0.6;
pub const PLAYER_DISMOUNT_HEIGHTS: [f32; 3] = [1.8, 1.5, 0.6];
pub const STEERING_ITEM: &str = "minecraft:warped_fungus_on_a_stick";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rider {
    Mob(u64),
    Player(u64),
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RideTick {
    rider: u64,
    vehicle: u64,
    target: Option<(i32, i32, i32)>,
}

pub fn eye_in_lava(world: &(impl World + ?Sized), position: DVec3, eye_height: f32) -> bool {
    let eye = position.y + f64::from(eye_height) - f64::from(0.111_111_11_f32);
    let pos = (
        position.x.floor() as i32,
        eye.floor() as i32,
        position.z.floor() as i32,
    );
    let Some(block) = world.block(pos).filter(|b| b.id == "minecraft:lava") else {
        return false;
    };
    let level = block
        .property("level")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    let amount = if level == 0 || level >= 8 {
        8
    } else {
        8 - level
    };
    let height = if strider::is_lava(world, (pos.0, pos.1 + 1, pos.2)) {
        1.0
    } else {
        f64::from(amount as f32 / 9.0)
    };
    f64::from(pos.1) + height > eye
}

pub fn escape_vector(vehicle_width: f32, rider_width: f32, yaw: f32) -> DVec3 {
    let distance =
        (f64::from(vehicle_width) + f64::from(rider_width) + f64::from(1.0e-5_f32)) / 2.0;
    let angle = f64::from(yaw * 0.017_453_292_f32);
    let x = -minecraftoss_player::mth::sin(angle);
    let z = minecraftoss_player::mth::cos(angle);
    let longest = x.abs().max(z.abs());
    DVec3::new(
        f64::from(x) * distance / f64::from(longest),
        0.0,
        f64::from(z) * distance / f64::from(longest),
    )
}

fn floor_height(world: &(impl World + ?Sized), pos: (i32, i32, i32)) -> f64 {
    let top = |boxes: Vec<[f64; 6]>| {
        boxes
            .iter()
            .map(|b| b[4])
            .fold(None, |m: Option<f64>, y| Some(m.map_or(y, |m| m.max(y))))
    };
    if let Some(top) = top(world.collision_boxes(pos)) {
        return top;
    }
    match top(world.collision_boxes((pos.0, pos.1 - 1, pos.2))) {
        Some(below) if below >= 1.0 => below - 1.0,
        _ => f64::NEG_INFINITY,
    }
}

pub fn dismount_spot(
    world: &(impl World + ?Sized),
    vehicle: &Strider,
    rider_width: f32,
    rider_heights: &[f32],
    yaw: f32,
) -> DVec3 {
    let body = &vehicle.body;
    let p = body.position;
    let top = p.y + f64::from(body.height);
    let bottom = p.y - 0.5;
    let mut columns: Vec<(i32, i32, i32)> = Vec::new();
    for turn in [0.0, -22.5, 22.5, -45.0, 45.0] {
        let offset = escape_vector(body.width, rider_width, yaw + turn);
        let mut y = top;
        while y > bottom {
            let pos = (
                (p.x + offset.x).floor() as i32,
                y.floor() as i32,
                (p.z + offset.z).floor() as i32,
            );
            if !columns.contains(&pos) {
                columns.push(pos);
            }
            y -= 1.0;
        }
    }
    for pos in columns {
        if world.fluid_is_lava(pos) {
            continue;
        }
        let floor = floor_height(world, pos);
        if !(floor.is_finite() && floor < 1.0) {
            continue;
        }
        let spot = DVec3::new(
            f64::from(pos.0) + 0.5,
            f64::from(pos.1) + floor,
            f64::from(pos.2) + 0.5,
        );
        let half = f64::from(rider_width) / 2.0;
        for &height in rider_heights {
            let min = spot - DVec3::new(half, 0.0, half);
            let max = spot + DVec3::new(half, f64::from(height), half);
            if minecraftoss_player::collision::no_block_collision(world, min, max) {
                return spot;
            }
        }
    }
    DVec3::new(p.x, top, p.z)
}

impl EntityWorld {
    pub fn passengers_of(&self, vehicle: u64) -> &[Rider] {
        self.passengers.get(&vehicle).map_or(&[], Vec::as_slice)
    }

    pub fn vehicle_of(&self, rider: Rider) -> Option<u64> {
        self.passengers
            .iter()
            .find(|(_, riders)| riders.contains(&rider))
            .map(|(&vehicle, _)| vehicle)
    }

    pub fn riding_mobs(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.passengers.iter().flat_map(|(&vehicle, riders)| {
            riders.iter().filter_map(move |rider| match rider {
                Rider::Mob(id) => Some((*id, vehicle)),
                Rider::Player(_) => None,
            })
        })
    }

    pub fn rides_together(&self, a: u64, b: u64) -> bool {
        let root = |mut id: u64| {
            while let Some(vehicle) = self.vehicle_of(Rider::Mob(id)) {
                id = vehicle;
            }
            id
        };
        (self.vehicle_of(Rider::Mob(a)).is_some() || self.vehicle_of(Rider::Mob(b)).is_some())
            && root(a) == root(b)
    }

    fn vehicle_strider(&self, vehicle: u64) -> Option<&StriderEntity> {
        self.striders.iter().find(|e| e.id == vehicle)
    }

    pub fn controlling_mob(&self, vehicle: u64) -> Option<u64> {
        let entity = self.vehicle_strider(vehicle)?;
        if entity.no_ai {
            return None;
        }
        match *self.passengers_of(vehicle).first()? {
            Rider::Mob(id) if !NON_CONTROLLING_RIDERS.contains(&self.entity_type(id)?) => {
                let steers = self.zombies.iter().any(|e| e.id == id && e.ai.is_some())
                    || self.striders.iter().any(|e| e.id == id);
                steers.then_some(id)
            }
            _ => None,
        }
    }

    pub fn controlling_player(&self, vehicle: u64) -> Option<u64> {
        let entity = self.vehicle_strider(vehicle)?;
        if !entity.strider.saddled {
            return None;
        }
        let Rider::Player(player) = *self.passengers_of(vehicle).first()? else {
            return None;
        };
        self.player_controls(player)?
            .holds(|item| item == STEERING_ITEM)
            .then_some(player)
    }

    pub fn boost_ride(&mut self, player: u64) -> bool {
        let Some(vehicle) = self.vehicle_of(Rider::Player(player)) else {
            return false;
        };
        if self.controlling_player(vehicle) != Some(player) {
            return false;
        }
        let Some(entity) = self.striders.iter_mut().find(|e| e.id == vehicle) else {
            return false;
        };
        entity.strider.steering.boost(&mut entity.random)
    }

    fn mob_vehicle_attachment(&self, id: u64) -> f64 {
        let offset = match self.entity_type(id) {
            Some(
                "minecraft:zombie"
                | "minecraft:husk"
                | "minecraft:drowned"
                | "minecraft:zombie_villager"
                | "minecraft:zombified_piglin"
                | "minecraft:skeleton"
                | "minecraft:stray"
                | "minecraft:bogged"
                | "minecraft:parched"
                | "minecraft:piglin"
                | "minecraft:piglin_brute",
            ) => f64::from(0.7_f32),
            Some("minecraft:wither_skeleton") => f64::from(0.875_f32),
            Some("minecraft:ghast") => -f64::from(0.5_f32),
            _ => 0.0,
        };
        let baby = self.zombies.iter().any(|e| e.id == id && e.zombie.baby);
        if baby { offset * 0.5 } else { offset }
    }

    pub fn seat_offset(&self, rider: Rider) -> Option<(u64, f64)> {
        let vehicle = self.vehicle_of(rider)?;
        let strider = &self.vehicle_strider(vehicle)?.strider;
        let attachment = if strider.age.baby() {
            strider::BABY_PASSENGER_ATTACHMENT
        } else {
            f64::from(strider.body.height)
        };
        let lift = match rider {
            Rider::Mob(id) => self.mob_vehicle_attachment(id),
            Rider::Player(_) => PLAYER_VEHICLE_ATTACHMENT,
        };
        Some((vehicle, attachment - lift))
    }

    pub fn seat(&self, rider: Rider) -> Option<DVec3> {
        let (vehicle, offset) = self.seat_offset(rider)?;
        let strider = &self.vehicle_strider(vehicle)?.strider;
        Some(strider.body.position + DVec3::new(0.0, offset, 0.0))
    }

    fn can_ride(&mut self, rider: Rider, vehicle: u64) -> bool {
        if rider == Rider::Mob(vehicle) || self.vehicle_strider(vehicle).is_none() {
            return false;
        }
        if let Rider::Mob(id) = rider {
            if self.body_mut(id).is_none() {
                return false;
            }
        }
        let mut up = Some(vehicle);
        while let Some(id) = up {
            if rider == Rider::Mob(id) {
                return false;
            }
            up = self.vehicle_of(Rider::Mob(id));
        }
        true
    }

    pub fn force_riding(&mut self, rider: Rider, vehicle: u64) -> bool {
        if !self.can_ride(rider, vehicle) {
            return false;
        }
        self.unlink(rider);
        self.passengers.entry(vehicle).or_default().push(rider);
        self.seat_mob(rider);
        true
    }

    pub fn start_riding(&mut self, rider: Rider, vehicle: u64, world: &impl World) -> bool {
        let Some(entity) = self.vehicle_strider(vehicle) else {
            return false;
        };
        let strider = &entity.strider;
        if !self.passengers_of(vehicle).is_empty()
            || eye_in_lava(world, strider.body.position, strider.eye_height())
        {
            return false;
        }
        self.force_riding(rider, vehicle)
    }

    fn unlink(&mut self, rider: Rider) -> Option<u64> {
        let vehicle = self.vehicle_of(rider)?;
        if let Some(riders) = self.passengers.get_mut(&vehicle) {
            riders.retain(|r| *r != rider);
            if riders.is_empty() {
                self.passengers.remove(&vehicle);
            }
        }
        Some(vehicle)
    }

    pub fn stop_riding(&mut self, rider: Rider, yaw: f32, world: &impl World) -> Option<DVec3> {
        let vehicle = self.vehicle_of(rider)?;
        let size = match rider {
            Rider::Mob(id) => {
                let body = self.body_mut(id)?;
                Some((body.width, body.height))
            }
            Rider::Player(_) => None,
        };
        let strider = self.vehicle_strider(vehicle)?.strider.clone();
        let spot = match size {
            Some((width, height)) => dismount_spot(world, &strider, width, &[height], yaw),
            None => dismount_spot(world, &strider, PLAYER_WIDTH, &PLAYER_DISMOUNT_HEIGHTS, yaw),
        };
        self.unlink(rider);
        if let Rider::Mob(id) = rider {
            if let Some(body) = self.body_mut(id) {
                body.position = spot;
                body.velocity = DVec3::ZERO;
                body.fall_distance = 0.0;
            }
            self.file_mob(id);
        }
        Some(spot)
    }

    fn file_mob(&mut self, id: u64) {
        if let Some(key) = self.order.iter().copied().find(|key| key.id() == id) {
            self.file_in_section(key);
        }
    }

    fn seat_mob(&mut self, rider: Rider) {
        let Rider::Mob(id) = rider else {
            return;
        };
        let Some(seat) = self.seat(rider) else {
            return;
        };
        if let Some(body) = self.body_mut(id) {
            body.position = seat;
            body.velocity = DVec3::ZERO;
            body.fall_distance = 0.0;
            body.on_ground = true;
        }
        self.file_mob(id);
    }

    pub(super) fn ride_order(&self, order: Vec<EntityKey>) -> Vec<EntityKey> {
        if self.passengers.is_empty() {
            return order;
        }
        let ticking: HashMap<u64, EntityKey> = order.iter().map(|key| (key.id(), *key)).collect();
        let mut out = Vec::with_capacity(order.len());
        for key in order {
            if self.vehicle_of(Rider::Mob(key.id())).is_none() {
                self.push_with_riders(key, &ticking, &mut out);
            }
        }
        out
    }

    fn push_with_riders(
        &self,
        key: EntityKey,
        ticking: &HashMap<u64, EntityKey>,
        out: &mut Vec<EntityKey>,
    ) {
        out.push(key);
        for rider in self.passengers_of(key.id()) {
            if let Rider::Mob(id) = rider {
                if let Some(&rider_key) = ticking.get(id) {
                    self.push_with_riders(rider_key, ticking, out);
                }
            }
        }
    }

    pub(super) fn begin_ride_tick(&mut self, id: u64) -> Option<RideTick> {
        let vehicle = self.vehicle_of(Rider::Mob(id))?;
        if let Some(body) = self.body_mut(id) {
            body.velocity = DVec3::ZERO;
        }
        let target = self
            .zombies
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.navigation.target_pos);
        Some(RideTick {
            rider: id,
            vehicle,
            target,
        })
    }

    pub(super) fn end_ride_tick(&mut self, tick: RideTick, world: &impl World) {
        if self.vehicle_of(Rider::Mob(tick.rider)) != Some(tick.vehicle) {
            return;
        }
        if self.controlling_mob(tick.vehicle) == Some(tick.rider) {
            let wanted = self
                .zombies
                .iter()
                .find(|e| e.id == tick.rider)
                .and_then(|e| {
                    let target = e.navigation.target_pos?;
                    (Some(target) != tick.target).then_some((target, e.navigation.speed_modifier))
                });
            if let Some((target, speed)) = wanted {
                let at = DVec3::new(
                    f64::from(target.0),
                    f64::from(target.1),
                    f64::from(target.2),
                );
                self.order_vehicle(
                    tick.vehicle,
                    &[StriderGoalEffect::Navigate { target: at, speed }],
                    world,
                );
            }
        }
        if let Some(body) = self.body_mut(tick.rider) {
            body.fall_distance = 0.0;
        }
        self.seat_mob(Rider::Mob(tick.rider));
    }

    pub(super) fn order_vehicle(
        &mut self,
        vehicle: u64,
        orders: &[StriderGoalEffect],
        world: &impl World,
    ) {
        let Some(entity) = self.striders.iter_mut().find(|e| e.id == vehicle) else {
            return;
        };
        let fluid = entity.fluid;
        for order in orders {
            match *order {
                StriderGoalEffect::Navigate { target, speed } => {
                    let _ = entity.strider.navigate_to(world, fluid, target, speed);
                }
                StriderGoalEffect::NavigateToEntity { target, speed } => {
                    let _ = entity
                        .strider
                        .navigate_to_entity(world, fluid, target, speed);
                }
                StriderGoalEffect::StopNavigation => entity.strider.navigation.stop(),
                _ => {}
            }
        }
    }

    pub(super) fn prune_riding(&mut self) {
        if self.passengers.is_empty() {
            return;
        }
        let alive: HashSet<u64> = self.positions().map(|(id, _)| id).collect();
        let dismounts = &mut self.player_dismounts;
        self.passengers.retain(|vehicle, riders| {
            riders.retain(|rider| match rider {
                Rider::Mob(id) => alive.contains(id),
                Rider::Player(_) => true,
            });
            if !alive.contains(vehicle) {
                for rider in riders.iter() {
                    if let Rider::Player(player) = rider {
                        dismounts.push((*player, *vehicle));
                    }
                }
            }
            alive.contains(vehicle) && !riders.is_empty()
        });
    }

    pub fn take_player_dismounts(&mut self) -> Vec<(u64, u64)> {
        std::mem::take(&mut self.player_dismounts)
    }

    pub fn dismount_player(
        &mut self,
        player: u64,
        yaw: f32,
        world: &impl World,
    ) -> Option<(u64, DVec3)> {
        let vehicle = self.vehicle_of(Rider::Player(player))?;
        let spot = self.stop_riding(Rider::Player(player), yaw, world)?;
        Some((vehicle, spot))
    }

    pub fn drop_player(&mut self, player: u64) -> Option<u64> {
        self.unlink(Rider::Player(player))
    }

    pub fn root_vehicle(&self, vehicle: u64) -> u64 {
        let mut root = vehicle;
        while let Some(up) = self.vehicle_of(Rider::Mob(root)) {
            root = up;
        }
        root
    }

    pub fn with_mob_riders(&self, vehicle: u64) -> Vec<u64> {
        let mut out = vec![vehicle];
        let mut next = 0;
        while next < out.len() {
            for rider in self.passengers_of(out[next]) {
                if let Rider::Mob(id) = rider {
                    out.push(*id);
                }
            }
            next += 1;
        }
        out
    }

    pub fn mob_with_uuid(&self, uuid: u128) -> Option<u64> {
        self.mob_ids().find(|&id| self.uuid_of(id) == uuid)
    }

    pub fn remove_mobs(&mut self, ids: &[u64]) {
        let gone = |id: u64| ids.contains(&id);
        self.bats.retain(|e| !gone(e.id));
        self.zombies.retain(|e| !gone(e.id));
        self.skeletons.retain(|e| !gone(e.id));
        self.creepers.retain(|e| !gone(e.id));
        self.spiders.retain(|e| !gone(e.id));
        self.slimes.retain(|e| !gone(e.id));
        self.endermen.retain(|e| !gone(e.id));
        self.witches.retain(|e| !gone(e.id));
        self.iron_golems.retain(|e| !gone(e.id));
        self.wolves.retain(|e| !gone(e.id));
        self.villagers.retain(|e| !gone(e.id));
        self.cows.retain(|e| !gone(e.id));
        self.sheep.retain(|e| !gone(e.id));
        self.pigs.retain(|e| !gone(e.id));
        self.chickens.retain(|e| !gone(e.id));
        self.ghasts.retain(|e| !gone(e.id));
        self.blazes.retain(|e| !gone(e.id));
        self.striders.retain(|e| !gone(e.id));
        self.order.retain(|key| !gone(key.id()));
        self.sections.retain_mobs(|id| !gone(id));
        self.passengers.retain(|vehicle, riders| {
            riders.retain(|rider| !matches!(rider, Rider::Mob(id) if gone(*id)));
            !gone(*vehicle) && !riders.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zombie::Zombie;
    use minecraftoss_player::Pos;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Blocks {
        blocks: BTreeMap<Pos, Block>,
    }

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.blocks.get(&pos).cloned()
        }
        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.blocks.insert(pos, block),
                None => self.blocks.remove(&pos),
            };
        }
    }

    fn lava() -> Block {
        Block::new("minecraft:lava").with("level", "0")
    }

    fn floor(id: &str, radius: i32) -> Blocks {
        let mut world = Blocks::default();
        for x in -radius..=radius {
            for z in -radius..=radius {
                world.blocks.insert((x, 63, z), Block::new(id));
            }
        }
        world
    }

    fn strider_at(entities: &mut EntityWorld, at: DVec3) -> u64 {
        entities.spawn_strider(Strider::new(at), false)
    }

    fn baby_strider(entities: &mut EntityWorld, at: DVec3) -> u64 {
        let mut baby = Strider::new(at);
        baby.age.set(crate::age::Age::BABY_START);
        entities.spawn_strider(baby, false)
    }

    fn zombified_piglin(entities: &mut EntityWorld, at: DVec3) -> u64 {
        let mut zombie = Zombie::new(at);
        zombie.kind = ZombieKind::ZombifiedPiglin;
        entities.spawn_zombie_active(zombie, 0.0)
    }

    fn feet(entities: &mut EntityWorld, id: u64) -> DVec3 {
        entities.body_mut(id).unwrap().position
    }

    #[test]
    fn a_baby_rides_an_adult_at_1_7_and_stays_warm_while_it_does() {
        let mut world = floor("minecraft:netherrack", 6);
        for x in -6..=6 {
            for z in -6..=6 {
                world.blocks.insert((x, 64, z), lava());
            }
        }
        let mut entities = EntityWorld::default();
        let adult = strider_at(&mut entities, DVec3::new(0.5, 64.5, 0.5));
        let baby = baby_strider(&mut entities, DVec3::new(3.5, 64.5, 0.5));
        assert!(entities.start_riding(Rider::Mob(baby), adult, &world));
        assert_eq!(entities.vehicle_of(Rider::Mob(baby)), Some(adult));
        assert_eq!(entities.controlling_mob(adult), Some(baby));
        for _ in 0..80 {
            entities.tick_with_players(&mut world, &[]);
            let under = feet(&mut entities, adult);
            let seat = feet(&mut entities, baby);
            assert!(
                (seat.y - under.y - f64::from(strider::HEIGHT)).abs() < 1.0e-9,
                "{seat} {under}"
            );
            assert_eq!((seat.x, seat.z), (under.x, under.z));
        }
        let baby_entity = entities.strider_mut(baby).unwrap();
        assert!(
            !baby_entity.strider.suffocating,
            "carried by a warm strider"
        );
        assert!(
            !baby_entity.fluid.in_lava(),
            "its own feet are out of the lava"
        );
        entities.strider_mut(adult).unwrap().strider.suffocating = true;
        let mut land = floor("minecraft:netherrack", 6);
        entities.tick_with_players(&mut land, &[]);
        entities.tick_with_players(&mut land, &[]);
        assert!(entities.strider_mut(baby).unwrap().strider.suffocating);
    }

    #[test]
    fn a_zombified_piglin_sits_at_1_0_and_steers_its_strider() {
        let mut world = floor("minecraft:netherrack", 12);
        let mut entities = EntityWorld::default();
        let strider = strider_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        let rider = zombified_piglin(&mut entities, DVec3::new(4.5, 64.0, 0.5));
        assert!(entities.start_riding(Rider::Mob(rider), strider, &world));
        let (vehicle, offset) = entities.seat_offset(Rider::Mob(rider)).unwrap();
        assert_eq!(vehicle, strider);
        assert!((offset - 1.0).abs() < 1.0e-6, "{offset}");
        assert_eq!(entities.controlling_mob(strider), Some(rider));
        for _ in 0..5 {
            entities.tick_with_players(&mut world, &[]);
        }
        assert!(
            entities
                .strider_mut(strider)
                .unwrap()
                .running_goals()
                .is_empty(),
            "its own goals pause"
        );
        let target = (8, 64, 0);
        entities.zombie_mut(rider).unwrap().navigation.target_pos = None;
        let tick = entities.begin_ride_tick(rider).unwrap();
        entities.zombie_mut(rider).unwrap().navigation.target_pos = Some(target);
        entities
            .zombie_mut(rider)
            .unwrap()
            .navigation
            .speed_modifier = 1.0;
        entities.end_ride_tick(tick, &world);
        let navigation = &entities.strider_mut(strider).unwrap().strider.navigation;
        assert_eq!(navigation.target_pos, Some(target));
        assert!(!navigation.is_done());
        let start = feet(&mut entities, strider);
        let mut farthest = start.x;
        for _ in 0..60 {
            entities.tick_with_players(&mut world, &[]);
            let under = feet(&mut entities, strider);
            let seat = feet(&mut entities, rider);
            assert!((seat.y - under.y - offset).abs() < 1.0e-9);
            farthest = farthest.max(under.x);
        }
        assert!(
            farthest > start.x + 0.5,
            "it walks where its rider wants: {farthest}"
        );
        let wanted = entities.zombie_mut(rider).unwrap().navigation.target_pos;
        let heading = entities
            .strider_mut(strider)
            .unwrap()
            .strider
            .navigation
            .target_pos;
        assert!(
            heading == Some(target) || heading == wanted,
            "{heading:?} {wanted:?}"
        );
        let census = entities.census();
        assert!(census.iter().all(|entry| entry.riding));
        assert!(
            census
                .iter()
                .find(|entry| entry.kind == "minecraft:zombified_piglin")
                .unwrap()
                .persistent
        );
    }

    #[test]
    fn a_strider_carries_one_passenger_and_none_with_its_eyes_in_lava() {
        let world = floor("minecraft:netherrack", 3);
        let mut entities = EntityWorld::default();
        let strider = strider_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        let first = zombified_piglin(&mut entities, DVec3::new(2.5, 64.0, 0.5));
        let second = baby_strider(&mut entities, DVec3::new(-2.5, 64.0, 0.5));
        assert!(entities.start_riding(Rider::Mob(first), strider, &world));
        assert!(!entities.start_riding(Rider::Mob(second), strider, &world));
        assert!(!entities.start_riding(Rider::Mob(strider), strider, &world));
        assert!(
            !entities.start_riding(Rider::Mob(strider), first, &world),
            "only striders carry"
        );
        let mut deep = floor("minecraft:netherrack", 3);
        for y in 64..67 {
            deep.blocks.insert((0, y, 0), lava());
        }
        let sunk = strider_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        assert!(eye_in_lava(
            &deep,
            DVec3::new(0.5, 64.0, 0.5),
            strider::EYE_HEIGHT
        ));
        assert!(!entities.start_riding(Rider::Mob(second), sunk, &deep));
        assert!(
            entities.force_riding(Rider::Mob(second), sunk),
            "a saved jockey loads anyway"
        );
        assert!(
            !entities.force_riding(Rider::Mob(sunk), second),
            "no riding in circles"
        );
        let mut ghast = crate::ghast::Ghast::new(DVec3::new(6.5, 64.0, 0.5));
        ghast.persistence_required = true;
        let ghast = entities.spawn_ghast(ghast, false);
        let carrier = strider_at(&mut entities, DVec3::new(-2.5, 64.0, -2.5));
        assert!(entities.start_riding(Rider::Mob(ghast), carrier, &world));
        assert_eq!(
            entities.controlling_mob(carrier),
            None,
            "a rider without a way to steer leaves the strider its own will"
        );
    }

    #[test]
    fn the_dismount_spot_is_forward_never_lava_and_on_top_as_a_last_resort() {
        let mut world = floor("minecraft:netherrack", 4);
        let mut entities = EntityWorld::default();
        let strider = strider_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        let vehicle = entities.strider_mut(strider).unwrap().strider.clone();
        let south = dismount_spot(
            &world,
            &vehicle,
            PLAYER_WIDTH,
            &PLAYER_DISMOUNT_HEIGHTS,
            0.0,
        );
        assert_eq!(south, DVec3::new(0.5, 64.0, 1.5), "yaw 0 looks south");
        let east = dismount_spot(
            &world,
            &vehicle,
            PLAYER_WIDTH,
            &PLAYER_DISMOUNT_HEIGHTS,
            -90.0,
        );
        assert_eq!(east, DVec3::new(1.5, 64.0, 0.5));
        for x in -4..=4 {
            for z in -4..=4 {
                world.blocks.insert((x, 63, z), lava());
            }
        }
        let over_lava = dismount_spot(
            &world,
            &vehicle,
            PLAYER_WIDTH,
            &PLAYER_DISMOUNT_HEIGHTS,
            0.0,
        );
        assert_eq!(
            over_lava,
            DVec3::new(0.5, 64.0 + f64::from(strider::HEIGHT), 0.5)
        );
        world
            .blocks
            .insert((0, 63, 1), Block::new("minecraft:netherrack"));
        let ledge = dismount_spot(
            &world,
            &vehicle,
            PLAYER_WIDTH,
            &PLAYER_DISMOUNT_HEIGHTS,
            0.0,
        );
        assert_eq!(ledge, DVec3::new(0.5, 64.0, 1.5));
        let rider = zombified_piglin(&mut entities, DVec3::new(3.5, 64.0, 0.5));
        assert!(entities.start_riding(Rider::Mob(rider), strider, &world));
        let spot = entities
            .stop_riding(Rider::Mob(rider), 0.0, &world)
            .unwrap();
        assert_eq!(spot, ledge);
        assert_eq!(feet(&mut entities, rider), ledge);
        assert_eq!(entities.vehicle_of(Rider::Mob(rider)), None);
    }

    #[test]
    fn killing_the_strider_lets_its_rider_down_where_it_sat() {
        let mut world = floor("minecraft:netherrack", 4);
        let mut entities = EntityWorld::default();
        let strider = strider_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        let rider = zombified_piglin(&mut entities, DVec3::new(3.5, 64.0, 0.5));
        assert!(entities.start_riding(Rider::Mob(rider), strider, &world));
        entities
            .strider_mut(strider)
            .unwrap()
            .hurt(100.0, DamageSourceKind::Generic);
        for _ in 0..10 {
            entities.tick_with_players(&mut world, &[]);
        }
        assert_eq!(
            entities.vehicle_of(Rider::Mob(rider)),
            Some(strider),
            "it stays on while the strider dies"
        );
        for _ in 0..20 {
            entities.tick_with_players(&mut world, &[]);
        }
        assert!(entities.strider_mut(strider).is_none());
        assert_eq!(entities.vehicle_of(Rider::Mob(rider)), None);
        assert!(entities.passengers.is_empty());
        for _ in 0..10 {
            entities.tick_with_players(&mut world, &[]);
        }
        let landed = feet(&mut entities, rider);
        assert!((landed.y - 64.0).abs() < 1.0e-9, "{landed}");
        assert!(
            landed.distance(DVec3::new(0.5, 64.0, 0.5)) < 2.0,
            "{landed}"
        );
    }

    fn ridden_strider(world: &Blocks, feet: f64, stick: bool, yaw: f32) -> (EntityWorld, u64) {
        let mut entities = EntityWorld::default();
        let mut strider = Strider::new(DVec3::new(0.5, feet, 0.5));
        strider.saddled = true;
        let id = entities.spawn_strider(strider, false);
        assert!(entities.start_riding(Rider::Player(0), id, world));
        entities.set_player_controls(&[crate::controls::PlayerControls {
            id: 0,
            yaw,
            main_hand: stick.then(|| STEERING_ITEM.to_owned()),
            ..Default::default()
        }]);
        (entities, id)
    }

    fn lava_sea(radius: i32) -> Blocks {
        let mut world = floor("minecraft:netherrack", radius);
        for x in -radius..=radius {
            for z in -radius..=radius {
                world.blocks.insert((x, 64, z), lava());
            }
        }
        world
    }

    fn steady_step(entities: &mut EntityWorld, world: &mut Blocks, id: u64) -> f64 {
        for _ in 0..90 {
            entities.tick_with_players(world, &[]);
        }
        let from = feet(entities, id);
        for _ in 0..10 {
            entities.tick_with_players(world, &[]);
        }
        feet(entities, id).distance(from) / 10.0
    }

    #[test]
    fn a_warm_ridden_strider_reaches_4_24_blocks_a_second_and_a_cold_one_1_78() {
        let mut sea = lava_sea(40);
        let (mut entities, id) = ridden_strider(&sea, 64.5, true, 0.0);
        let warm = steady_step(&mut entities, &mut sea, id);
        assert!(!entities.strider_mut(id).unwrap().strider.suffocating);
        let expected = 0.175 * 0.55 / (1.0 - 0.6 * 0.91);
        assert!((warm / expected - 1.0).abs() < 0.01, "{warm} {expected}");
        assert!((warm * 20.0 - 4.24).abs() < 0.05, "{}", warm * 20.0);
        assert!(
            feet(&mut entities, id).z > 15.0,
            "it walks where the rider looks"
        );
        let mut land = floor("minecraft:netherrack", 40);
        let (mut entities, id) = ridden_strider(&land, 64.0, true, 0.0);
        let cold = steady_step(&mut entities, &mut land, id);
        assert!(entities.strider_mut(id).unwrap().strider.suffocating);
        let expected = 0.175 * (1.0 - 0.34) * 0.35 / (1.0 - 0.6 * 0.91);
        assert!((cold / expected - 1.0).abs() < 0.01, "{cold} {expected}");
        assert!((cold * 20.0 - 1.78).abs() < 0.05, "{}", cold * 20.0);
    }

    #[test]
    fn it_turns_to_the_riders_yaw_at_once() {
        let mut sea = lava_sea(8);
        let (mut entities, id) = ridden_strider(&sea, 64.5, true, 90.0);
        assert_eq!(entities.controlling_player(id), Some(0));
        entities.tick_with_players(&mut sea, &[]);
        let entity = entities.strider_mut(id).unwrap();
        assert_eq!(entity.strider.yaw, 90.0);
        assert_eq!(entity.body_rotation.body_yaw, 90.0);
        assert_eq!(entity.look_control.head_yaw, 90.0);
        let moved = entity.strider.body.position;
        assert!(moved.x < 0.5 && (moved.z - 0.5).abs() < 1.0e-6, "{moved}");
    }

    #[test]
    fn without_the_stick_the_rider_does_not_steer() {
        let mut sea = lava_sea(8);
        let (mut entities, id) = ridden_strider(&sea, 64.5, false, 90.0);
        assert_eq!(entities.controlling_player(id), None);
        assert!(!entities.boost_ride(0));
        entities.tick_with_players(&mut sea, &[]);
        assert_ne!(entities.strider_mut(id).unwrap().strider.yaw, 90.0);
        entities.set_player_controls(&[crate::controls::PlayerControls {
            id: 0,
            offhand: Some(STEERING_ITEM.to_owned()),
            ..Default::default()
        }]);
        assert_eq!(
            entities.controlling_player(id),
            Some(0),
            "the off hand steers too"
        );
        entities.strider_mut(id).unwrap().strider.saddled = false;
        assert_eq!(entities.controlling_player(id), None);
    }

    #[test]
    fn boost_lasts_140_to_980_ticks_and_a_second_boost_fails_meanwhile() {
        let mut sea = lava_sea(4);
        let (mut entities, id) = ridden_strider(&sea, 64.5, true, 0.0);
        assert!(entities.boost_ride(0));
        assert!(!entities.boost_ride(0), "one boost at a time");
        assert!(!entities.boost_ride(1), "only the rider boosts");
        let total = entities
            .strider_mut(id)
            .unwrap()
            .strider
            .steering
            .boost_time_total;
        assert!((140..=980).contains(&total), "{total}");
        let mut ticks = 0;
        let mut fastest = 0.0_f32;
        while entities.strider_mut(id).unwrap().strider.steering.boosting {
            let entity = entities.strider_mut(id).unwrap();
            entity.strider.body.position = DVec3::new(0.5, 64.5, 0.5);
            fastest = fastest.max(entity.ridden_speed());
            entities.tick_with_players(&mut sea, &[]);
            ticks += 1;
        }
        assert_eq!(ticks, total + 2);
        let base = (0.175_f64 * f64::from(0.55_f32)) as f32;
        assert!((fastest / base - 2.15).abs() < 0.01, "{fastest} {base}");
        assert!(
            entities.boost_ride(0),
            "a new boost after the last one ends"
        );
    }
}
