use minecraft_survival::death::DamageCause;
use minecraft_survival::elytra::{Flight, can_glide, fall_peak};
use minecraft_survival::player::Player;
use minecraft_terrain::pack::PackStack;
use minecraftoss_player::survival::Armor;
use playerstate_iw4::PlayerState;
use playerstate_iw4::pm_flags::MC_GLIDING;

use crate::minecraft_sounds::Sounds;

pub(crate) fn step(
    player: &mut Player,
    authority: &mut sim::SimWorld,
    id: sim::ClientId,
    origin: [f64; 3],
    seconds: f64,
    sounds: Option<&mut Sounds>,
    packs: &PackStack,
) {
    let ps = authority
        .player(id)
        .copied()
        .filter(|ps| ps.pm_type == 0 && !player.dead);
    let may = ps.is_some() && player.mode.interacts() && can_glide(&player.inventory);
    authority.set_may_glide(id, may);
    let flight = ps.map_or_else(Flight::default, |ps| flight(&ps, may));
    if let Some(ps) = ps {
        let feet = sim::voxel::to_block(origin, ps.origin);
        player.steps.air_peak = fall_peak(player.steps.air_peak, feet[1], flight);
    }
    let wears = player.mode.consumes_items();
    let glided = player
        .elytra
        .step(flight, seconds, &mut player.inventory, wears);
    let Some(damage) = glided.wall_damage else {
        return;
    };
    if !player
        .survival
        .hurt(damage, Armor::default(), None, DamageCause::FlyIntoWall)
    {
        return;
    }
    if let Some(sounds) = sounds {
        let fall = minecraft_survival::survival::fall_sound(damage as i32);
        sounds.play(packs, fall, None, 1.0, 1.0);
        let voice = (sounds.random() - sounds.random()) * 0.2 + 1.0;
        sounds.play(packs, "minecraft:entity.player.hurt", None, 1.0, voice);
    }
}

fn flight(ps: &PlayerState, may: bool) -> Flight {
    let per_tick = f64::from(sim::voxel::BLOCK) * 20.0;
    let [x, y, z] = ps.velocity.map(f64::from);
    Flight {
        gliding: may && ps.pm_flags & MC_GLIDING != 0,
        motion: [x / per_tick, z / per_tick, -y / per_tick],
    }
}

pub(crate) fn launch_rocket(
    authority: &mut sim::SimWorld,
    id: sim::ClientId,
    boost_ticks: i32,
    swing: &mut Option<f32>,
    spend: impl FnOnce(),
) {
    if authority.launch_rocket(id, boost_ticks) {
        *swing = Some(0.0);
        spend();
    }
}
