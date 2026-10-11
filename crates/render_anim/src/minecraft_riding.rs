use minecraft_survival::entities::Entities;
use minecraft_survival::player::Steps;
use minecraft_survival::riding::Riding;

pub(crate) fn step(
    riding: &mut Option<Riding>,
    steps: &mut Steps,
    entities: &mut Entities,
    authority: &mut sim::SimWorld,
    id: sim::ClientId,
    origin: [f64; 3],
    present: bool,
) {
    for vehicle in entities.take_mounts() {
        let uuid = entities.vehicle_uuid(vehicle);
        *riding = Some(Riding { vehicle, uuid });
    }
    for (vehicle, spot) in entities.take_dismounts() {
        if riding.is_none_or(|ride| ride.vehicle != vehicle) {
            continue;
        }
        *riding = None;
        entities.leave_seat();
        authority.ride(id, None);
        if let Some(spot) = spot {
            authority.teleport(id, sim::voxel::to_map(origin, spot.to_array()));
        }
        steps.air_peak = None;
    }
    if !present {
        dismount(riding, Some(entities), authority, id);
    }
    if let Some(ride) = *riding
        && let Some(seat) = entities.ride_seat(ride.vehicle)
    {
        authority.ride(id, Some(sim::voxel::to_map(origin, seat.to_array())));
        steps.air_peak = None;
    }
}

pub(crate) fn dismount(
    riding: &mut Option<Riding>,
    entities: Option<&mut Entities>,
    authority: &mut sim::SimWorld,
    id: sim::ClientId,
) {
    if riding.take().is_none() {
        return;
    }
    if let Some(entities) = entities {
        entities.dismount();
    }
    authority.ride(id, None);
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
        dismount(&mut riding, None, &mut authority, sim::ClientId(1));
        assert_eq!(riding, None);
    }
}
