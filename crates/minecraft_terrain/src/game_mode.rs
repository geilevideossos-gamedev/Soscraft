pub use minecraftoss_player::GameMode;

pub const MODES: [GameMode; 4] = [
    GameMode::Survival,
    GameMode::Creative,
    GameMode::Adventure,
    GameMode::Spectator,
];

pub fn id(mode: GameMode) -> i32 {
    match mode {
        GameMode::Survival => 0,
        GameMode::Creative => 1,
        GameMode::Adventure => 2,
        GameMode::Spectator => 3,
    }
}

pub fn from_id(id: i32) -> Option<GameMode> {
    MODES.into_iter().find(|&mode| self::id(mode) == id)
}

pub fn parse(name: &str) -> Option<GameMode> {
    MODES.into_iter().find(|mode| mode.name() == name)
}

pub fn translation_key(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Survival => "gameMode.survival",
        GameMode::Creative => "gameMode.creative",
        GameMode::Adventure => "gameMode.adventure",
        GameMode::Spectator => "gameMode.spectator",
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Abilities {
    pub invulnerable: bool,
    pub flying: bool,
    pub may_fly: bool,
    pub instabuild: bool,
    pub may_build: bool,
    pub flying_speed: f32,
    pub walking_speed: f32,
}

impl Default for Abilities {
    fn default() -> Self {
        Self {
            invulnerable: false,
            flying: false,
            may_fly: false,
            instabuild: false,
            may_build: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }
    }
}

impl Abilities {
    pub fn update_for(&mut self, mode: GameMode) {
        match mode {
            GameMode::Creative => {
                self.may_fly = true;
                self.instabuild = true;
                self.invulnerable = true;
            }
            GameMode::Spectator => {
                self.may_fly = true;
                self.instabuild = false;
                self.invulnerable = true;
                self.flying = true;
            }
            GameMode::Survival | GameMode::Adventure => {
                self.may_fly = false;
                self.instabuild = false;
                self.invulnerable = false;
                self.flying = false;
            }
        }
        self.may_build = !matches!(mode, GameMode::Adventure | GameMode::Spectator);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerMode {
    pub mode: GameMode,
    pub previous: Option<GameMode>,
    pub abilities: Abilities,
}

impl Default for PlayerMode {
    fn default() -> Self {
        Self::new(GameMode::Survival)
    }
}

impl PlayerMode {
    pub fn new(mode: GameMode) -> Self {
        let mut abilities = Abilities::default();
        abilities.update_for(mode);
        Self {
            mode,
            previous: None,
            abilities,
        }
    }

    pub fn set(&mut self, mode: GameMode) -> bool {
        if mode == self.mode {
            return false;
        }
        self.previous = Some(self.mode);
        self.mode = mode;
        self.abilities.update_for(mode);
        true
    }

    pub fn invulnerable(&self) -> bool {
        self.abilities.invulnerable
    }

    pub fn consumes_items(&self) -> bool {
        !self.abilities.instabuild
    }

    pub fn instant_break(&self) -> bool {
        self.abilities.instabuild
    }

    pub fn may_build(&self) -> bool {
        self.abilities.may_build
    }

    pub fn interacts(&self) -> bool {
        self.mode != GameMode::Spectator
    }

    pub fn drops_from_breaking(&self) -> bool {
        !self.abilities.instabuild
    }

    pub fn shows_vitals(&self) -> bool {
        self.mode.is_survival()
    }

    pub fn shows_hotbar(&self) -> bool {
        self.mode != GameMode::Spectator
    }

    pub fn attackable_by_mobs(&self) -> bool {
        !self.abilities.invulnerable
    }

    pub fn noclip(&self) -> bool {
        self.mode == GameMode::Spectator
    }

    pub fn picks_up(&self) -> bool {
        self.mode != GameMode::Spectator
    }
}

pub fn can_destroy_in_creative(item: Option<&str>) -> bool {
    let Some(item) = item else {
        return true;
    };
    let path = item.strip_prefix("minecraft:").unwrap_or(item);
    !(path.ends_with("_sword") || path == "trident" || path == "mace")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(abilities: Abilities) -> [bool; 5] {
        [
            abilities.invulnerable,
            abilities.flying,
            abilities.may_fly,
            abilities.instabuild,
            abilities.may_build,
        ]
    }

    #[test]
    fn abilities_follow_game_type_update_player_abilities() {
        let of = |mode| flags(PlayerMode::new(mode).abilities);
        assert_eq!(of(GameMode::Survival), [false, false, false, false, true]);
        assert_eq!(of(GameMode::Creative), [true, false, true, true, true]);
        assert_eq!(of(GameMode::Adventure), [false, false, false, false, false]);
        assert_eq!(of(GameMode::Spectator), [true, true, true, false, false]);
        let abilities = PlayerMode::new(GameMode::Creative).abilities;
        assert_eq!(
            (abilities.flying_speed, abilities.walking_speed),
            (0.05, 0.1)
        );
    }

    #[test]
    fn creative_keeps_flying_and_survival_clears_it() {
        let mut player = PlayerMode::new(GameMode::Creative);
        player.abilities.flying = true;
        player.abilities.update_for(GameMode::Creative);
        assert!(player.abilities.flying);
        player.set(GameMode::Survival);
        assert!(!player.abilities.flying);
        player.set(GameMode::Creative);
        assert!(!player.abilities.flying);
        player.set(GameMode::Spectator);
        assert!(player.abilities.flying);
        player.set(GameMode::Creative);
        assert!(player.abilities.flying);
    }

    #[test]
    fn setting_the_same_mode_changes_nothing_and_a_change_remembers_the_last() {
        let mut player = PlayerMode::default();
        assert_eq!(player.mode, GameMode::Survival);
        assert_eq!(player.previous, None);
        assert!(!player.set(GameMode::Survival));
        assert_eq!(player.previous, None);
        assert!(player.set(GameMode::Creative));
        assert_eq!(
            (player.mode, player.previous),
            (GameMode::Creative, Some(GameMode::Survival))
        );
        assert!(!player.set(GameMode::Creative));
        assert_eq!(player.previous, Some(GameMode::Survival));
        assert!(player.set(GameMode::Spectator));
        assert_eq!(player.previous, Some(GameMode::Creative));
    }

    #[test]
    fn ids_and_names_round_trip_and_unknown_ones_are_refused() {
        for mode in MODES {
            assert_eq!(from_id(id(mode)), Some(mode));
            assert_eq!(parse(mode.name()), Some(mode));
            assert_eq!(translation_key(mode), format!("gameMode.{}", mode.name()));
        }
        assert_eq!(
            MODES.map(id),
            [0, 1, 2, 3],
            "survival, creative, adventure, spectator"
        );
        assert_eq!(from_id(4), None);
        assert_eq!(from_id(-1), None);
        assert_eq!(parse("hardcore"), None);
        assert_eq!(parse("Creative"), None);
        assert_eq!(parse("1"), None);
    }

    #[test]
    fn creative_and_spectator_are_safe_from_mobs_and_only_survival_modes_show_vitals() {
        for mode in MODES {
            let player = PlayerMode::new(mode);
            let safe = matches!(mode, GameMode::Creative | GameMode::Spectator);
            assert_eq!(player.invulnerable(), safe, "{mode:?}");
            assert_eq!(player.attackable_by_mobs(), !safe, "{mode:?}");
            assert_eq!(player.shows_vitals(), !safe, "{mode:?}");
            let spectator = mode == GameMode::Spectator;
            assert_eq!(player.picks_up(), !spectator, "{mode:?}");
            assert_eq!(player.noclip(), spectator, "{mode:?}");
            assert_eq!(player.shows_hotbar(), !spectator, "{mode:?}");
            assert_eq!(player.interacts(), !spectator, "{mode:?}");
        }
    }

    #[test]
    fn swords_tridents_and_maces_cannot_break_blocks_in_creative() {
        assert!(!can_destroy_in_creative(Some("minecraft:diamond_sword")));
        assert!(!can_destroy_in_creative(Some("wooden_sword")));
        assert!(!can_destroy_in_creative(Some("minecraft:trident")));
        assert!(!can_destroy_in_creative(Some("minecraft:mace")));
        assert!(can_destroy_in_creative(Some("minecraft:diamond_pickaxe")));
        assert!(can_destroy_in_creative(Some("minecraft:iron_spear")));
        assert!(can_destroy_in_creative(Some("minecraft:stone")));
        assert!(can_destroy_in_creative(None));
    }
}
