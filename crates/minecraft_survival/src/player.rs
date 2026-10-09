use minecraft_commands::PlayerId;
use minecraft_commands::server::Profile;
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::game_mode::PlayerMode;

use crate::death::DeathWatch;
use crate::survival::Survival;

pub struct Player {
    pub uuid: PlayerUuid,
    pub id: PlayerId,
    pub name: String,
    pub dimension: Dimension,
    pub survival: Survival,
    pub mode: PlayerMode,
    pub death_watch: DeathWatch,
}

impl Player {
    pub fn new(profile: &Profile, mode: PlayerMode, survival: Survival) -> Self {
        Self {
            uuid: PlayerUuid(profile.uuid.clone()),
            id: profile.id,
            name: profile.name.clone(),
            dimension: Dimension::Overworld,
            survival,
            mode,
            death_watch: DeathWatch::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::HOST;
    use minecraft_commands::CommandServer;
    use minecraft_terrain::game_mode::GameMode;

    #[test]
    fn a_joining_player_starts_in_the_overworld_with_the_survival_it_carries() {
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Steve");
        let mut carried = Survival::default();
        carried.observe(100, 100);
        carried.status.health = 7.0;
        let player = Player::new(
            chat.profile(HOST).unwrap(),
            PlayerMode::new(GameMode::Creative),
            carried,
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
    }
}
