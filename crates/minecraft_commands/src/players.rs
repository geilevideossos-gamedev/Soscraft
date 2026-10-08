use minecraftoss_player::GameMode;

pub const GAME_MODES: [GameMode; 4] = [
    GameMode::Survival,
    GameMode::Creative,
    GameMode::Adventure,
    GameMode::Spectator,
];

pub fn game_mode_named(name: &str) -> Option<GameMode> {
    GAME_MODES.into_iter().find(|mode| mode.name() == name)
}

pub fn game_mode_key(mode: GameMode) -> String {
    format!("gameMode.{}", mode.name())
}

pub fn offline_uuid(name: &str) -> String {
    let mut bytes = md5::compute(format!("OfflinePlayer:{name}").as_bytes()).0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_matches_vanilla() {
        assert_eq!(
            offline_uuid("Notch"),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        let uuid = offline_uuid("Steve");
        assert_eq!(uuid.len(), 36);
        assert_eq!(&uuid[14..15], "3");
        assert!(matches!(&uuid[19..20], "8" | "9" | "a" | "b"));
        assert_ne!(offline_uuid("steve"), uuid);
    }

    #[test]
    fn game_modes_take_their_vanilla_names_only() {
        for mode in GAME_MODES {
            assert_eq!(game_mode_named(mode.name()), Some(mode));
        }
        assert_eq!(
            GAME_MODES.map(GameMode::name),
            ["survival", "creative", "adventure", "spectator"]
        );
        assert_eq!(game_mode_key(GameMode::Creative), "gameMode.creative");
        assert_eq!(game_mode_named("Creative"), None);
        assert_eq!(game_mode_named("1"), None);
        assert_eq!(game_mode_named("hardcore"), None);
    }
}
