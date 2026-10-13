use std::sync::Arc;

use minecraft_commands::server::Profile;
use minecraft_commands::{Clientbound, PlayerId, Text};
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::game_mode::PlayerMode;
use minecraftoss_player::Block;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::{Inventory, ItemStack};

use crate::death::DeathWatch;
use crate::offhand::Blocking;
use crate::player_nbt::{ENDER_SLOTS, GlobalPos, Place, PlayerData, Respawn, SLOTS, Stats};
use crate::survival::Survival;
use crate::switch::{LoadingReason, Switch, loading_reason};

pub struct Player {
    pub uuid: PlayerUuid,
    pub id: PlayerId,
    pub name: String,
    pub dimension: Dimension,
    pub survival: Survival,
    pub mode: PlayerMode,
    pub death_watch: DeathWatch,
    pub inventory: Inventory,
    pub selected: usize,
    pub hand: Hand,
    pub steps: Steps,
    pub dead: bool,
    pub arrived: bool,
    pub place: Option<Place>,
    pub arrival: Option<Place>,
    pub ender: Vec<Option<ItemStack>>,
    pub respawn: Option<Respawn>,
    pub last_death: Option<GlobalPos>,
    pub portal_cooldown: i32,
    pub seen_credits: bool,
    pub end: crate::the_end::EndTravel,
    pub elytra: crate::elytra::Elytra,
    pub switching: Option<Switch>,
    pub respawning: Option<Dimension>,
    pub remote: bool,
    pub held: Option<Switch>,
    pub portal: crate::portal::Portal,
    pub follow: crate::guns::Follow,
    pub arsenal: crate::guns::Binding,
    pub menus: crate::menus::Menus,
    pub enchantment_seed: i32,
    pub riding: Option<crate::riding::Riding>,
    pub saved_vehicle: Option<minecraftoss_core::nbt::Tag>,
    pub reattach: Option<minecraftoss_core::nbt::Tag>,
    pub sleeping: Option<crate::sleep::Sleeping>,
    pub sleep_timer: i32,
    pub time_since_rest: i32,
    pub saved_in_bed: Option<(i32, i32, i32)>,
    pub tags: Vec<String>,
}

#[derive(Default)]
pub struct Hand {
    pub swing: Option<f32>,
    pub clock: f64,
    pub place_delay: u32,
    pub cooldowns: crate::cooldowns::Cooldowns,
    pub recent: Vec<((i32, i32, i32), Block, f64)>,
    pub attack_ticks: u32,
    pub attack_item: Option<String>,
    pub miss_time: u32,
    pub blocking: Option<Blocking>,
    pub charging: Option<crate::trident::Charging>,
    pub scoping: Option<crate::spyglass::Scoping>,
    pub swing_offhand: bool,
    pub use_by_key: bool,
    pub offhand_key_was: bool,
    pub creative_delay: u32,
    pub using_eye: Option<usize>,
    pub dig: crate::digging::Dig,
}

#[derive(Default)]
pub struct Steps {
    pub last: Option<[f64; 3]>,
    pub move_dist: f32,
    pub next_step: f32,
    pub air_peak: Option<f64>,
}

impl Player {
    pub fn new(
        profile: &Profile,
        mode: PlayerMode,
        survival: Survival,
        inventory: Inventory,
    ) -> Self {
        Self {
            uuid: PlayerUuid(profile.uuid.clone()),
            id: profile.id,
            name: profile.name.clone(),
            dimension: Dimension::Overworld,
            survival,
            mode,
            death_watch: DeathWatch::default(),
            inventory,
            selected: 0,
            hand: Hand::default(),
            steps: Steps::default(),
            dead: false,
            arrived: false,
            place: None,
            arrival: None,
            ender: vec![None; ENDER_SLOTS],
            respawn: None,
            last_death: None,
            portal_cooldown: 0,
            seen_credits: false,
            end: crate::the_end::EndTravel::default(),
            elytra: crate::elytra::Elytra::default(),
            switching: None,
            respawning: None,
            remote: false,
            held: None,
            portal: crate::portal::Portal::default(),
            follow: crate::guns::Follow::default(),
            arsenal: crate::guns::Binding::default(),
            menus: crate::menus::Menus::default(),
            enchantment_seed: crate::level::opener_key(&PlayerUuid(profile.uuid.clone())) as i32,
            riding: None,
            saved_vehicle: None,
            reattach: None,
            sleeping: None,
            sleep_timer: 0,
            time_since_rest: 0,
            saved_in_bed: None,
            tags: Vec::new(),
        }
    }

    pub fn close_menu(&mut self) -> Vec<ItemStack> {
        crate::menus::close(self)
    }

    pub fn arrive(&mut self, place: Place) {
        self.dimension = place.dimension;
        self.place = Some(place);
        self.arrival = Some(place);
    }

    pub fn switch_to(&mut self, to: Dimension, arrival: Option<Place>, reason: LoadingReason) {
        let switch = Switch::new(self.dimension, to, arrival, reason);
        self.dimension = to;
        self.arrival = switch.arrival;
        self.switching = Some(switch);
    }

    pub fn travel_to(&mut self, to: Dimension, arrival: Option<Place>, reason: LoadingReason) {
        if self.remote {
            self.held = Some(Switch::new(self.dimension, to, arrival, reason));
        } else {
            self.switch_to(to, arrival, reason);
        }
    }

    pub fn teleport_into(&mut self, place: Place) -> bool {
        if place.dimension == self.dimension {
            return false;
        }
        let reason = loading_reason(self.dead, self.dimension, place.dimension);
        self.travel_to(place.dimension, Some(place), reason);
        true
    }

    pub fn set_remote(&mut self, remote: bool) {
        self.remote = remote;
        if let Some(switch) = self.held.take_if(|_| !remote) {
            self.switch_to(switch.to, switch.arrival, switch.reason);
        }
    }

    pub fn set_respawn(&mut self, respawn: Option<Respawn>) -> bool {
        let moved = respawn.is_some_and(|new| self.respawn.map(|old| old.at) != Some(new.at));
        self.respawn = respawn;
        moved
    }

    pub fn arrive_at_respawn(
        &mut self,
        loaded: &[Dimension],
        open: impl Fn((i32, i32, i32)) -> Option<bool>,
        bed: impl FnOnce(&Respawn) -> Option<Place>,
    ) -> bool {
        let Some(respawn) = self.respawn else {
            return false;
        };
        if !loaded.contains(&respawn.at.dimension) {
            return false;
        }
        match respawn_place(&respawn, open, bed) {
            Some(place) => {
                self.arrive(place);
                false
            }
            None => {
                self.respawn = None;
                true
            }
        }
    }

    pub fn observe_life(&mut self, pm_type: i32) -> bool {
        let (in_world, died) = crate::death::life_state(pm_type);
        let respawned = in_world && self.dead;
        if died && !self.dead {
            self.last_death = self.place.map(|place| GlobalPos {
                dimension: place.dimension,
                pos: (
                    place.feet[0].floor() as i32,
                    place.feet[1].floor() as i32,
                    place.feet[2].floor() as i32,
                ),
            });
        }
        if died {
            self.dead = true;
        } else if in_world {
            self.dead = false;
            self.arrived = true;
        }
        respawned
    }

    pub fn data(&self) -> PlayerData {
        let mut inventory = self.inventory.clone();
        let mut loose: Vec<ItemStack> = inventory
            .crafting
            .iter_mut()
            .chain(inventory.workbench.iter_mut())
            .chain(std::iter::once(&mut inventory.cursor))
            .filter_map(Option::take)
            .collect();
        if let Some(crate::menus::Menu {
            open: crate::menus::Open::Bench(_, bench),
            ..
        }) = self.menus.open.as_ref()
        {
            loose.extend(bench.cells.iter().flatten().cloned());
        }
        for stack in loose {
            let _ = inventory.add_item(stack, self.selected);
        }
        let mut status = self.survival.status;
        if self.dead {
            status.health = 0.0;
        }
        PlayerData {
            mode: Some(self.mode),
            inventory: Some(inventory.slots),
            selected: self.selected,
            stats: Some(Stats {
                status,
                fire: self.survival.fire_ticks(),
                score: self.survival.score,
                frozen: self.survival.ticks_frozen(),
            }),
            place: self.place,
            uuid: Some(self.uuid.0.clone()),
            ender: Some(self.ender.clone()),
            respawn: self.respawn,
            last_death: self.last_death,
            portal_cooldown: Some(self.portal_cooldown),
            seen_credits: Some(self.seen_credits),
            root_vehicle: crate::riding::root_vehicle(
                self.riding.is_some(),
                self.saved_vehicle.as_ref(),
                self.reattach.as_ref(),
            ),
            sleep_timer: Some(self.sleep_timer),
            sleeping_pos: self
                .sleeping
                .map(|sleeping| sleeping.head)
                .or(self.saved_in_bed),
            time_since_rest: Some(self.time_since_rest),
            tags: Some(self.tags.clone()),
        }
    }
}

pub fn respawn_place(
    respawn: &Respawn,
    open: impl Fn((i32, i32, i32)) -> Option<bool>,
    bed: impl FnOnce(&Respawn) -> Option<Place>,
) -> Option<Place> {
    if let Some(place) = bed(respawn) {
        return Some(place);
    }
    let (x, y, z) = respawn.at.pos;
    if !respawn.forced || open((x, y, z)) == Some(false) || open((x, y + 1, z)) == Some(false) {
        return None;
    }
    Some(Place::standing(
        respawn.at.dimension,
        [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5],
        respawn.yaw,
        respawn.pitch,
    ))
}

pub fn missing_respawn_message() -> Clientbound {
    Clientbound::System {
        text: Text::translate("block.minecraft.spawn.not_valid", Vec::new()),
        overlay: false,
    }
}

pub fn joining_inventory(
    recipes: Arc<RecipeBook>,
    saved: Option<Vec<Option<ItemStack>>>,
    kit: Option<&str>,
) -> Inventory {
    let Some(slots) = saved else {
        return starting_inventory(recipes, kit);
    };
    let mut inventory = starting_inventory(recipes, None);
    inventory.slots = slots;
    inventory.slots.resize(SLOTS, None);
    if kit.is_some() {
        diag::info!(
            World,
            "Minecraft kit skipped: the player file already holds an inventory"
        );
    }
    inventory
}

pub fn starting_inventory(recipes: Arc<RecipeBook>, kit: Option<&str>) -> Inventory {
    let mut inventory = Inventory::default();
    inventory.recipes = recipes;
    if let Some(kit) = kit {
        let given = minecraft_terrain::crafting::give_kit(&mut inventory, kit);
        diag::info!(
            World,
            "Minecraft kit: {given} stacks from IW4L_MINECRAFT_KIT"
        );
    }
    inventory
}

pub fn hold_off_death(
    survival: &mut Survival,
    inventory: &mut Inventory,
    selected: usize,
    struck: bool,
) -> bool {
    (struck || survival.status.health <= 0.0) && survival.protect(inventory, selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::HOST;
    use minecraft_commands::CommandServer;
    use minecraft_terrain::game_mode::GameMode;
    use minecraftoss_player::inventory::ItemStack;

    #[test]
    fn a_joining_player_starts_in_the_overworld_with_the_survival_it_carries() {
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Steve");
        let mut carried = Survival::default();
        carried.observe(100, 100);
        carried.status.health = 7.0;
        let mut inventory = Inventory::default();
        inventory.slots[3] = Some(ItemStack::new("minecraft:stone", 5));
        let player = Player::new(
            chat.profile(HOST).unwrap(),
            PlayerMode::new(GameMode::Creative),
            carried,
            inventory,
        );
        assert_eq!(
            player.uuid,
            PlayerUuid(minecraft_commands::offline_uuid("Steve"))
        );
        assert_eq!(player.id, HOST);
        assert_eq!(player.name, "Steve");
        assert_eq!(player.dimension, Dimension::Overworld);
        assert_eq!(player.survival.status.health, 7.0);
        assert_eq!(player.mode.mode, GameMode::Creative);
        assert!(!player.death_watch.switched_teams());
        assert_eq!(
            player.inventory.slots[3].as_ref().map(|stack| stack.count),
            Some(5)
        );
        assert_eq!(player.selected, 0);
    }

    #[test]
    fn a_joining_player_has_no_hand_or_fall_timers_running() {
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Alex");
        let player = Player::new(
            chat.profile(HOST).unwrap(),
            PlayerMode::new(GameMode::Survival),
            Survival::default(),
            Inventory::default(),
        );
        assert!(player.hand.cooldowns.is_empty());
        assert_eq!(player.hand.place_delay, 0);
        assert!(player.hand.blocking.is_none());
        assert!(player.hand.swing.is_none());
        assert!(player.hand.recent.is_empty());
        assert!(player.steps.last.is_none());
        assert!(player.steps.air_peak.is_none());
    }

    #[test]
    fn the_kit_fills_the_backpack_and_the_inventory_uses_the_world_recipes() {
        let recipes = Arc::new(RecipeBook::default());
        let empty = starting_inventory(recipes.clone(), None);
        assert!(Arc::ptr_eq(&empty.recipes, &recipes));
        assert!(empty.slots.iter().all(Option::is_none));
        let kit = starting_inventory(recipes.clone(), Some("cobblestone*70,diamond_sword"));
        assert!(Arc::ptr_eq(&kit.recipes, &recipes));
        let stacks: Vec<(usize, &str, u8)> = kit
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, stack)| stack.as_ref().map(|s| (slot, s.id.as_str(), s.count)))
            .collect();
        assert_eq!(
            stacks,
            vec![
                (9, "minecraft:cobblestone", 64),
                (10, "minecraft:cobblestone", 6),
                (11, "minecraft:diamond_sword", 1)
            ]
        );
    }

    #[test]
    fn a_saved_inventory_comes_back_in_its_slots_with_the_world_recipes() {
        let recipes = Arc::new(RecipeBook::default());
        let mut slots = vec![None; SLOTS];
        slots[4] = Some(ItemStack::new("minecraft:bread", 6));
        slots[39] = Some(ItemStack::new("minecraft:iron_helmet", 1));
        slots[40] = Some(ItemStack::new("minecraft:shield", 1));
        let inventory = joining_inventory(recipes.clone(), Some(slots.clone()), None);
        assert!(Arc::ptr_eq(&inventory.recipes, &recipes));
        assert_eq!(inventory.slots, slots);
        let short = joining_inventory(recipes.clone(), Some(vec![None; 36]), None);
        assert_eq!(short.slots.len(), SLOTS);
        let fresh = joining_inventory(recipes, None, Some("dirt"));
        assert_eq!(
            fresh.slots[9].as_ref().map(|stack| stack.id.as_str()),
            Some("minecraft:dirt")
        );
    }

    #[test]
    fn the_kit_only_goes_to_a_player_without_a_saved_inventory() {
        let recipes = Arc::new(RecipeBook::default());
        let mut slots = vec![None; SLOTS];
        slots[0] = Some(ItemStack::new("minecraft:bread", 2));
        let saved = joining_inventory(recipes.clone(), Some(slots.clone()), Some("dirt*3"));
        assert_eq!(saved.slots, slots);
        let emptied = joining_inventory(recipes.clone(), Some(vec![None; SLOTS]), Some("dirt*3"));
        assert!(emptied.slots.iter().all(Option::is_none));
        let new = joining_inventory(recipes, None, Some("dirt*3"));
        assert_eq!(
            new.slots[9]
                .as_ref()
                .map(|stack| (stack.id.as_str(), stack.count)),
            Some(("minecraft:dirt", 3))
        );
    }

    #[test]
    fn a_rider_never_uses_a_portal() {
        let mut player = steve();
        assert!(crate::portal::may_use_portals(&player));
        player.riding = Some(crate::riding::Riding {
            vehicle: 3,
            uuid: 7,
        });
        assert!(!crate::portal::may_use_portals(&player));
        player.riding = None;
        player.dead = true;
        assert!(!crate::portal::may_use_portals(&player));
    }

    fn steve() -> Player {
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Steve");
        Player::new(
            chat.profile(HOST).unwrap(),
            PlayerMode::new(GameMode::Survival),
            Survival::default(),
            Inventory::default(),
        )
    }

    #[test]
    fn a_player_arrives_when_first_alive_and_is_dead_until_respawned() {
        let mut player = steve();
        assert!(!player.arrived && !player.dead);
        player.observe_life(playerstate_iw4::PM_TYPE_SPECTATOR);
        assert!(!player.arrived && !player.dead);
        player.observe_life(0);
        assert!(player.arrived && !player.dead);
        player.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        assert!(player.dead);
        player.observe_life(playerstate_iw4::PM_TYPE_INTERMISSION);
        assert!(player.dead);
        player.observe_life(0);
        assert!(!player.dead);
        player.observe_life(playerstate_iw4::PM_TYPE_INTERMISSION);
        assert!(!player.dead);
    }

    fn spawn_point(pos: (i32, i32, i32), forced: bool) -> Respawn {
        Respawn {
            at: GlobalPos {
                dimension: Dimension::Overworld,
                pos,
            },
            yaw: -90.0,
            pitch: 10.0,
            forced,
        }
    }

    #[test]
    fn a_death_is_remembered_where_the_player_last_stood_and_a_respawn_is_reported() {
        let mut player = steve();
        player.place = Some(Place::standing(
            Dimension::Overworld,
            [10.7, 63.0, -4.2],
            0.0,
            0.0,
        ));
        assert!(!player.observe_life(0));
        assert!(!player.observe_life(playerstate_iw4::PM_TYPE_DEAD));
        assert_eq!(
            player.last_death,
            Some(GlobalPos {
                dimension: Dimension::Overworld,
                pos: (10, 63, -5)
            })
        );
        player.place = None;
        assert!(!player.observe_life(playerstate_iw4::PM_TYPE_DEAD));
        assert!(player.last_death.is_some());
        assert!(player.observe_life(0));
        assert!(!player.observe_life(0));
        let data = player.data();
        assert_eq!(data.last_death, player.last_death);
    }

    #[test]
    fn setting_a_respawn_point_tells_whether_it_moved() {
        let mut player = steve();
        assert!(player.set_respawn(Some(spawn_point((1, 64, 1), true))));
        assert!(!player.set_respawn(Some(spawn_point((1, 64, 1), false))));
        assert!(player.set_respawn(Some(spawn_point((2, 64, 1), true))));
        assert!(!player.set_respawn(None));
        assert_eq!(player.respawn, None);
        player.set_respawn(Some(spawn_point((3, 70, 3), true)));
        assert_eq!(player.data().respawn, Some(spawn_point((3, 70, 3), true)));
    }

    #[test]
    fn a_forced_respawn_point_needs_two_open_blocks_and_a_bed_needs_a_stand_up_spot() {
        let point = spawn_point((4, 64, -8), true);
        let placed = respawn_place(&point, |_| Some(true), |_| None).unwrap();
        assert_eq!(placed.feet, [4.5, 64.0, -7.5]);
        assert_eq!((placed.yaw, placed.pitch), (-90.0, 10.0));
        assert!(placed.on_ground);
        assert_eq!(
            respawn_place(&point, |_| None, |_| None).map(|p| p.feet),
            Some([4.5, 64.0, -7.5])
        );
        assert_eq!(
            respawn_place(&point, |pos| Some(pos.1 != 65), |_| None),
            None
        );
        assert_eq!(
            respawn_place(&point, |pos| Some(pos.1 != 64), |_| None),
            None
        );
        let bed = spawn_point((4, 64, -8), false);
        assert_eq!(respawn_place(&bed, |_| Some(true), |_| None), None);
        let beside = Place::standing(Dimension::Overworld, [5.5, 64.0, -7.5], 90.0, 0.0);
        assert_eq!(
            respawn_place(&bed, |_| Some(true), |_| Some(beside)),
            Some(beside)
        );
    }

    #[test]
    fn a_respawn_goes_to_the_point_or_clears_a_missing_one() {
        let mut player = steve();
        let loaded = [Dimension::Overworld];
        assert!(!player.arrive_at_respawn(&loaded, |_| Some(true), |_| None));
        assert_eq!(player.arrival, None);
        player.set_respawn(Some(spawn_point((4, 64, -8), true)));
        assert!(!player.arrive_at_respawn(&loaded, |_| Some(true), |_| None));
        assert_eq!(
            player.arrival.map(|place| place.feet),
            Some([4.5, 64.0, -7.5])
        );
        assert_eq!(player.place, player.arrival);
        assert!(player.respawn.is_some());
        player.arrival = None;
        assert!(player.arrive_at_respawn(&loaded, |_| Some(false), |_| None));
        assert_eq!(player.respawn, None);
        assert_eq!(player.arrival, None);
        let mut nether = spawn_point((4, 64, -8), true);
        nether.at.dimension = Dimension::Nether;
        player.set_respawn(Some(nether));
        assert!(!player.arrive_at_respawn(&loaded, |_| Some(false), |_| None));
        assert_eq!(player.respawn, Some(nether));
        assert_eq!(player.arrival, None);
        assert_eq!(
            missing_respawn_message(),
            Clientbound::System {
                text: Text::translate("block.minecraft.spawn.not_valid", Vec::new()),
                overlay: false,
            }
        );
    }

    #[test]
    fn a_switching_player_keeps_where_it_stood_until_it_arrives() {
        let mut player = steve();
        let home = Place::standing(Dimension::Overworld, [800.5, 70.0, -1600.5], 45.0, 0.0);
        player.arrive(home);
        player.arrival = None;
        let roof = Place::standing(Dimension::Nether, [100.5, 128.0, -200.5], 45.0, 0.0);
        player.switch_to(Dimension::Nether, Some(roof), LoadingReason::NetherPortal);
        assert_eq!(player.dimension, Dimension::Nether);
        assert_eq!(player.arrival, Some(roof));
        assert_eq!(player.place, Some(home), "where to go back to if the Nether fails to load");
        let switch = player.switching.expect("switching");
        assert_eq!((switch.from, switch.to, switch.reason), (Dimension::Overworld, Dimension::Nether, LoadingReason::NetherPortal));
        assert!(!switch.left && !switch.respawn);
        player.switch_to(Dimension::End, Some(roof), LoadingReason::EndPortal);
        assert_eq!(player.arrival, None, "an arrival in another dimension is not taken");
        assert_eq!(player.switching.map(|switch| switch.from), Some(Dimension::Nether));
    }

    #[test]
    fn a_teleport_into_another_dimension_travels_there() {
        let mut player = steve();
        let home = Place::standing(Dimension::Overworld, [800.5, 70.0, -1600.5], 45.0, 0.0);
        player.arrive(home);
        player.arrival = None;
        let near = Place::standing(Dimension::Overworld, [10.5, 64.0, 10.5], 0.0, 0.0);
        assert!(!player.teleport_into(near), "the same dimension is a plain teleport");
        assert!(player.switching.is_none());
        let roof = Place::standing(Dimension::Nether, [100.5, 70.0, -200.5], 45.0, 0.0);
        assert!(player.teleport_into(roof));
        assert_eq!(player.dimension, Dimension::Nether);
        assert_eq!(player.arrival, Some(roof));
        let switch = player.switching.expect("travelling");
        assert_eq!((switch.from, switch.reason), (Dimension::Overworld, LoadingReason::NetherPortal));
        player.switching = None;
        player.set_remote(true);
        let end = Place::standing(Dimension::End, [100.5, 49.0, 0.5], 0.0, 0.0);
        assert!(player.teleport_into(end));
        assert_eq!(player.dimension, Dimension::Nether, "held until the killstreak ends");
        player.set_remote(false);
        assert_eq!(player.switching.map(|switch| (switch.from, switch.to, switch.reason)), Some((Dimension::Nether, Dimension::End, LoadingReason::NetherPortal)));
    }

    #[test]
    fn travel_waits_while_the_player_is_in_a_remote_killstreak() {
        let mut player = steve();
        let home = Place::standing(Dimension::Overworld, [800.5, 70.0, -1600.5], 45.0, 0.0);
        player.arrive(home);
        let roof = Place::standing(Dimension::Nether, [100.5, 128.0, -200.5], 45.0, 0.0);
        player.set_remote(true);
        player.travel_to(Dimension::Nether, Some(roof), LoadingReason::NetherPortal);
        assert_eq!(player.dimension, Dimension::Overworld, "still flying the killstreak");
        assert!(player.switching.is_none());
        player.set_remote(true);
        assert!(player.switching.is_none() && player.held.is_some());
        player.set_remote(false);
        assert_eq!(player.dimension, Dimension::Nether);
        assert_eq!(player.arrival, Some(roof));
        let switch = player.switching.expect("the held switch goes on");
        assert_eq!((switch.from, switch.to, switch.reason), (Dimension::Overworld, Dimension::Nether, LoadingReason::NetherPortal));
        assert!(player.held.is_none());
        player.switching = None;
        player.travel_to(Dimension::Overworld, None, LoadingReason::NetherPortal);
        assert_eq!(player.switching.map(|switch| switch.to), Some(Dimension::Overworld), "no killstreak, no wait");
        player.switching = None;
        player.set_remote(true);
        player.switch_to(Dimension::End, None, LoadingReason::Other);
        assert_eq!(player.dimension, Dimension::End, "a respawn never waits");
    }

    #[test]
    fn a_player_saves_its_stats_and_a_dead_one_saves_no_health() {
        let mut player = steve();
        player.survival.observe(100, 100);
        player.survival.status.health = 9.0;
        player.survival.status.experience_level = 4;
        player.survival.score = 33;
        let stats = player.data().stats.unwrap();
        assert_eq!(stats.status.health, 9.0);
        assert_eq!(stats.status.experience_level, 4);
        assert_eq!(stats.score, 33);
        assert_eq!(stats.fire, player.survival.fire_ticks());
        player.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        assert_eq!(player.data().stats.unwrap().status.health, 0.0);
        assert_eq!(player.survival.status.health, 9.0);
    }

    #[test]
    fn a_player_saves_its_mode_and_slots_with_loose_stacks_put_back() {
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Steve");
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:stone", 5));
        inventory.slots[40] = Some(ItemStack::new("minecraft:shield", 1));
        inventory.cursor = Some(ItemStack::new("minecraft:stone", 3));
        inventory.crafting[1] = Some(ItemStack::new("minecraft:oak_planks", 2));
        inventory.workbench[8] = Some(ItemStack::new("minecraft:stick", 4));
        let mut player = Player::new(
            chat.profile(HOST).unwrap(),
            PlayerMode::new(GameMode::Creative),
            Survival::default(),
            inventory,
        );
        player.selected = 6;
        let data = player.data();
        assert_eq!(data.mode, Some(PlayerMode::new(GameMode::Creative)));
        assert_eq!(data.selected, 6);
        let slots = data.inventory.unwrap();
        assert_eq!(slots.len(), SLOTS);
        let held = |id: &str| -> u32 {
            slots
                .iter()
                .flatten()
                .filter(|stack| stack.id == id)
                .map(|stack| u32::from(stack.count))
                .sum()
        };
        assert_eq!(held("minecraft:stone"), 8);
        assert_eq!(held("minecraft:oak_planks"), 2);
        assert_eq!(held("minecraft:stick"), 4);
        assert_eq!(
            slots[40].as_ref().map(|stack| stack.id.as_str()),
            Some("minecraft:shield")
        );
        assert!(player.inventory.cursor.is_some());
        assert!(player.inventory.crafting[1].is_some());
    }

    #[test]
    fn a_totem_holds_off_death_only_when_the_player_would_die() {
        let mut survival = Survival::default();
        survival.observe(100, 100);
        let mut inventory = Inventory::default();
        inventory.slots[40] = Some(ItemStack::new(crate::offhand::TOTEM, 1));
        assert!(!hold_off_death(&mut survival, &mut inventory, 0, false));
        assert!(inventory.slots[40].is_some());
        survival.status.health = 0.0;
        assert!(hold_off_death(&mut survival, &mut inventory, 0, false));
        assert_eq!(survival.status.health, 1.0);
        assert!(inventory.slots[40].is_none());
        survival.status.health = 0.0;
        assert!(!hold_off_death(&mut survival, &mut inventory, 0, false));
    }

    #[test]
    fn a_totem_pop_from_the_game_uses_the_held_totem_at_any_health() {
        let mut survival = Survival::default();
        survival.observe(100, 100);
        let mut inventory = Inventory::default();
        inventory.slots[2] = Some(ItemStack::new(crate::offhand::TOTEM, 1));
        assert!(!hold_off_death(&mut survival, &mut inventory, 0, true));
        assert!(hold_off_death(&mut survival, &mut inventory, 2, true));
        assert!(inventory.slots[2].is_none());
        assert_eq!(survival.status.health, 1.0);
    }
}
