use minecraft_survival::entities::Entities;
use minecraft_survival::player::Steps;
use minecraft_survival::riding::Riding;
use minecraftoss_core::nbt::Tag;

#[allow(clippy::too_many_arguments)]
pub(crate) fn step(
    riding: &mut Option<Riding>,
    reattach: &mut Option<Tag>,
    steps: &mut Steps,
    entities: &mut Entities,
    authority: &mut sim::SimWorld,
    (id, player): (sim::ClientId, u64),
    origin: [f64; 3],
    present: bool,
) {
    for vehicle in entities.take_mounts(player) {
        let uuid = entities.vehicle_uuid(vehicle);
        *riding = Some(Riding { vehicle, uuid });
    }
    for (vehicle, spot) in entities.take_dismounts(player) {
        if riding.is_none_or(|ride| ride.vehicle != vehicle) {
            continue;
        }
        *riding = None;
        entities.leave_seat(player);
        authority.ride(id, None);
        if let Some(spot) = spot {
            authority.teleport(id, sim::voxel::to_map(origin, spot.to_array()));
        }
        steps.air_peak = None;
    }
    if !present {
        dismount(riding, Some(entities), authority, (id, player));
    } else if let Some(root) = reattach.take() {
        entities.reattach(player, root);
    }
    if let Some(ride) = *riding
        && let Some(seat) = entities.ride_seat(player, ride.vehicle)
    {
        authority.ride(id, Some(sim::voxel::to_map(origin, seat.to_array())));
        steps.air_peak = None;
    }
}

pub(crate) fn dismount(
    riding: &mut Option<Riding>,
    entities: Option<&mut Entities>,
    authority: &mut sim::SimWorld,
    (id, player): (sim::ClientId, u64),
) {
    if riding.take().is_none() {
        return;
    }
    if let Some(entities) = entities {
        entities.dismount(player);
    }
    authority.ride(id, None);
}

pub(crate) fn ride_input(authority: &sim::SimWorld, client: sim::ClientId) -> (f32, f32, bool) {
    let [forward, right] = authority.old_moves(client).unwrap_or([0, 0]);
    let jumping = authority
        .old_buttons(client)
        .is_some_and(|held| held & playerstate_iw4::buttons::JUMP != 0);
    let (x, z) = (-f32::from(right) / 127.0, f32::from(forward) / 127.0);
    let length = (x * x + z * z).sqrt();
    let (x, z) = if length > 1.0 {
        (x / length, z / length)
    } else {
        (x, z)
    };
    (z * 0.98, x * 0.98, jumping)
}

pub(crate) fn counts_steps(alive: bool, noclip: bool, riding: bool) -> bool {
    alive && !noclip && !riding
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rider_counts_no_steps_and_no_fall() {
        assert!(counts_steps(true, false, false));
        assert!(!counts_steps(true, false, true));
        assert!(!counts_steps(false, false, false));
        assert!(!counts_steps(true, true, false));
        let mut riding = Some(Riding {
            vehicle: 4,
            uuid: 9,
        });
        let mut authority = sim::SimWorld::default();
        dismount(&mut riding, None, &mut authority, (sim::ClientId(1), 1));
        assert_eq!(riding, None);
    }

    #[test]
    fn the_keys_steer_like_a_minecraft_player_s_input() {
        let mut authority = sim::SimWorld::default();
        let client = sim::ClientId(3);
        assert_eq!(ride_input(&authority, client), (0.0, 0.0, false));
        authority.set_old_moves(client, [127, 0]);
        authority.set_old_buttons(client, playerstate_iw4::buttons::JUMP);
        assert_eq!(ride_input(&authority, client), (0.98, 0.0, true));
        authority.set_old_moves(client, [127, 127]);
        authority.set_old_buttons(client, 0);
        let (forward, strafe, jumping) = ride_input(&authority, client);
        assert!((forward - 0.98 / 2f32.sqrt()).abs() < 1e-6 && (strafe + 0.98 / 2f32.sqrt()).abs() < 1e-6 && !jumping);
    }
}
