use glam::DVec3;
use minecraft_net::ToClient;
use minecraft_terrain::ender_dragon::DRAGON_DEATH_SOUND;
use minecraft_terrain::mob_tracking::{tracking_range, tracks};

use crate::actions::Effect;

pub const SOUND_RANGE: f64 = 16.0;
pub const EVENT_RANGE: f64 = 64.0;
pub const EXPLOSION_RANGE: f64 = 64.0;
pub const BLOCK_EVENT_RANGE: f64 = 64.0;
pub const TOTEM_EVENT: u8 = 35;
pub const SPLASH_POTION_EVENT: i32 = 2002;
pub const INSTANT_POTION_EVENT: i32 = 2007;
pub const ITEM_PICKUP_SOUND: &str = "minecraft:entity.item.pickup";
pub const ORB_PICKUP_SOUND: &str = "minecraft:entity.experience_orb.pickup";
pub const LEVEL_UP_SOUND: &str = "minecraft:entity.player.levelup";
pub const LIMIT: usize = 4096;
const ACTOR_ONLY: [&str; 3] = [
    "minecraft:ui.hud.bubble_pop",
    "minecraft:block.portal.trigger",
    "minecraft:block.portal.travel",
];

#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    Sound {
        event: String,
        at: [f64; 3],
        volume: f32,
        pitch: f32,
    },
    Event {
        event: i32,
        pos: [i32; 3],
        data: i32,
    },
    Explosion {
        center: [f64; 3],
        radius: f32,
        blocks: u32,
        interacts: bool,
    },
    Block {
        pos: [i32; 3],
        kind: u8,
        param: i32,
    },
    Player {
        client: u32,
        event: u8,
        at: [f64; 3],
    },
}

impl Heard {
    pub fn sound(event: &str, at: [f64; 3], volume: f32, pitch: f32) -> Self {
        Self::Sound {
            event: event.to_owned(),
            at,
            volume,
            pitch,
        }
    }

    fn at(&self) -> [f64; 3] {
        let centre = |pos: &[i32; 3]| pos.map(|k| f64::from(k) + 0.5);
        match self {
            Self::Sound { at, .. } | Self::Player { at, .. } => *at,
            Self::Explosion { center, .. } => *center,
            Self::Event { pos, .. } | Self::Block { pos, .. } => centre(pos),
        }
    }

    fn range(&self) -> f64 {
        match self {
            Self::Sound { event, .. } if event == DRAGON_DEATH_SOUND => f64::INFINITY,
            Self::Sound { volume, .. } => SOUND_RANGE * f64::from(volume.max(1.0)),
            Self::Event { .. } => EVENT_RANGE,
            Self::Explosion { .. } => EXPLOSION_RANGE,
            Self::Block { .. } => BLOCK_EVENT_RANGE,
            Self::Player { .. } => f64::INFINITY,
        }
    }

    pub fn reaches(&self, feet: [f64; 3], view_distance: i32) -> bool {
        let at = self.at();
        if let Self::Player { .. } = self {
            let range = tracking_range("minecraft:player");
            return tracks(range, view_distance, feet, DVec3::from_array(at));
        }
        let range = self.range();
        let distance: f64 = (0..3).map(|k| (at[k] - feet[k]).powi(2)).sum();
        range.is_infinite() || distance < range * range
    }

    pub fn message(&self) -> ToClient {
        match self.clone() {
            Self::Sound {
                event,
                at,
                volume,
                pitch,
            } => ToClient::Sound {
                event,
                pos: at,
                volume,
                pitch,
                seed: 0,
            },
            Self::Event { event, pos, data } => ToClient::LevelEvent { event, pos, data },
            Self::Explosion {
                center,
                radius,
                blocks,
                interacts,
            } => ToClient::Explosion {
                center,
                radius,
                blocks,
                interacts,
            },
            Self::Block { pos, kind, param } => ToClient::BlockEvent { pos, kind, param },
            Self::Player { client, event, .. } => ToClient::PlayerEvent { client, event },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Said {
    pub heard: Heard,
    pub only: Option<u32>,
}

impl Said {
    pub fn all(heard: Heard) -> Self {
        Self { heard, only: None }
    }

    pub fn to(client: u32, heard: Heard) -> Self {
        Self {
            heard,
            only: Some(client),
        }
    }

    pub fn reaches(&self, client: u32, feet: [f64; 3], view_distance: i32) -> bool {
        self.only.is_none_or(|only| only == client) && self.heard.reaches(feet, view_distance)
    }
}

#[derive(Debug, Default)]
pub struct HeardLog {
    said: Option<Vec<Said>>,
}

impl HeardLog {
    pub fn push(&mut self, said: Said) {
        if let Some(log) = self.said.as_mut().filter(|log| log.len() < LIMIT) {
            log.push(said);
        }
    }

    pub fn listening(&self) -> bool {
        self.said.is_some()
    }

    pub fn listen(&mut self, on: bool) -> Vec<Said> {
        let next = on.then(Vec::new);
        std::mem::replace(&mut self.said, next).unwrap_or_else(Vec::new)
    }
}

pub fn actor_only(event: &str) -> bool {
    ACTOR_ONLY.contains(&event)
}

pub fn played(
    event: &str,
    at: Option<[f64; 3]>,
    (volume, pitch): (f32, f32),
    actor: u32,
    eye: [f64; 3],
) -> Said {
    let heard = Heard::sound(event, at.unwrap_or(eye), volume, pitch);
    if actor_only(event) {
        Said::to(actor, heard)
    } else {
        Said::all(heard)
    }
}

pub fn of_effects(
    effects: &[Effect],
    actor: u32,
    eye: [f64; 3],
    mut random: impl FnMut() -> f32,
) -> Vec<Said> {
    let mut said = Vec::new();
    for effect in effects {
        match effect {
            Effect::Sound {
                event,
                at,
                volume,
                pitch,
                spread,
            } => {
                let pitch = if *spread == 0.0 {
                    *pitch
                } else {
                    pitch + random() * spread
                };
                said.push(played(event, *at, (*volume, pitch), actor, eye));
            }
            Effect::Dig {
                pos,
                sound: Some((event, volume, pitch)),
                ..
            } => {
                let at = [pos.0, pos.1, pos.2].map(|k| f64::from(k) + 0.5);
                said.push(Said::to(actor, Heard::sound(event, at, *volume, *pitch)));
            }
            Effect::Dig { sound: None, .. }
            | Effect::Crumbs { .. }
            | Effect::Chat(_)
            | Effect::Overlay(_) => {}
        }
    }
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEET: [f64; 3] = [0.0, 64.0, 0.0];

    fn sound_at(x: f64, volume: f32) -> Heard {
        Heard::sound("minecraft:block.stone.place", [x, 64.0, 0.0], volume, 1.0)
    }

    #[test]
    fn a_sound_reaches_sixteen_blocks_times_its_volume_and_never_less_than_sixteen() {
        assert!(sound_at(15.9, 1.0).reaches(FEET, 8));
        assert!(!sound_at(16.0, 1.0).reaches(FEET, 8));
        assert!(
            sound_at(15.9, 0.15).reaches(FEET, 8),
            "quiet sounds still reach 16"
        );
        assert!(!sound_at(16.1, 0.15).reaches(FEET, 8));
        assert!(
            sound_at(63.9, 4.0).reaches(FEET, 8),
            "an explosion's 4 reaches 64"
        );
        assert!(!sound_at(64.0, 4.0).reaches(FEET, 8));
        let up = Heard::sound("minecraft:block.stone.place", [0.0, 80.5, 0.0], 1.0, 1.0);
        assert!(!up.reaches(FEET, 8), "the distance is three dimensional");
    }

    #[test]
    fn level_events_block_events_and_explosions_reach_sixty_four_blocks() {
        let event = |x| Heard::Event {
            event: 1501,
            pos: [x, 64, 0],
            data: 0,
        };
        assert!(event(62).reaches(FEET, 8));
        assert!(!event(64).reaches(FEET, 8));
        let lid = |x| Heard::Block {
            pos: [x, 64, 0],
            kind: 1,
            param: 1,
        };
        assert!(lid(62).reaches(FEET, 8));
        assert!(!lid(64).reaches(FEET, 8));
        let blast = |x| Heard::Explosion {
            center: [x, 64.0, 0.0],
            radius: 4.0,
            blocks: 10,
            interacts: true,
        };
        assert!(blast(63.9).reaches(FEET, 8));
        assert!(!blast(64.1).reaches(FEET, 8));
    }

    #[test]
    fn the_dragon_dies_for_everyone_and_a_totem_pops_for_those_tracking_the_player() {
        let death = Heard::sound(DRAGON_DEATH_SOUND, [5000.0, 64.0, 0.0], 5.0, 1.0);
        assert!(death.reaches(FEET, 8));
        let totem = |x| Heard::Player {
            client: 2,
            event: TOTEM_EVENT,
            at: [x, 64.0, 0.0],
        };
        assert!(totem(127.0).reaches(FEET, 8), "within 8 chunks");
        assert!(!totem(129.0).reaches(FEET, 8));
        assert!(
            totem(129.0).reaches(FEET, 12),
            "a longer view tracks further"
        );
        assert!(
            !totem(513.0).reaches(FEET, 64),
            "players track 32 chunks at most"
        );
    }

    #[test]
    fn a_said_line_for_one_player_reaches_only_them() {
        let said = Said::to(3, sound_at(1.0, 1.0));
        assert!(said.reaches(3, FEET, 8));
        assert!(!said.reaches(4, FEET, 8));
        assert!(Said::all(sound_at(1.0, 1.0)).reaches(4, FEET, 8));
    }

    #[test]
    fn the_log_keeps_lines_only_while_someone_listens() {
        let mut log = HeardLog::default();
        log.push(Said::all(sound_at(0.0, 1.0)));
        assert!(!log.listening());
        assert!(log.listen(true).is_empty(), "nothing was kept before");
        log.push(Said::all(sound_at(0.0, 1.0)));
        assert_eq!(log.listen(true).len(), 1);
        assert!(log.listen(true).is_empty());
        log.push(Said::all(sound_at(0.0, 1.0)));
        assert_eq!(log.listen(false).len(), 1);
        log.push(Said::all(sound_at(0.0, 1.0)));
        assert!(log.listen(true).is_empty(), "the log stopped");
        for _ in 0..LIMIT + 10 {
            log.push(Said::all(sound_at(0.0, 1.0)));
        }
        assert_eq!(log.listen(true).len(), LIMIT);
    }

    #[test]
    fn listener_sounds_come_from_the_eye_and_dig_sounds_go_to_the_digger_only() {
        let eye = [3.0, 65.6, 4.0];
        let effects = vec![
            Effect::Sound {
                event: "minecraft:entity.generic.eat".to_owned(),
                at: None,
                volume: 0.5,
                pitch: 1.0,
                spread: 0.0,
            },
            Effect::Sound {
                event: "minecraft:block.wood.place".to_owned(),
                at: Some([1.5, 64.5, 1.5]),
                volume: 1.0,
                pitch: 0.8,
                spread: 0.4,
            },
            Effect::Dig {
                pos: (2, 63, 2),
                face: minecraftoss_player::Face::Up,
                block: minecraft_terrain::scene::Block {
                    id: minecraft_terrain::pack::ResourceId::parse("minecraft:stone").unwrap(),
                    properties: Default::default(),
                },
                sound: Some(("minecraft:block.stone.hit".to_owned(), 0.25, 0.5)),
            },
            Effect::Overlay("block.minecraft.bed.no_sleep".to_owned()),
        ];
        let said = of_effects(&effects, 7, eye, || 0.5);
        assert_eq!(
            said,
            vec![
                Said::all(Heard::sound("minecraft:entity.generic.eat", eye, 0.5, 1.0)),
                Said::all(Heard::sound(
                    "minecraft:block.wood.place",
                    [1.5, 64.5, 1.5],
                    1.0,
                    1.0
                )),
                Said::to(
                    7,
                    Heard::sound("minecraft:block.stone.hit", [2.5, 63.5, 2.5], 0.25, 0.5)
                ),
            ]
        );
    }

    #[test]
    fn bubbles_and_portal_noises_stay_with_the_player_who_made_them() {
        let eye = [0.0, 65.6, 0.0];
        let bubble = played("minecraft:ui.hud.bubble_pop", None, (1.0, 1.0), 4, eye);
        assert_eq!(bubble.only, Some(4));
        let travel = played("minecraft:block.portal.travel", None, (0.25, 1.0), 4, eye);
        assert_eq!(travel.only, Some(4));
        let hurt = played("minecraft:entity.player.hurt", None, (1.0, 1.0), 4, eye);
        assert_eq!(
            hurt,
            Said::all(Heard::sound("minecraft:entity.player.hurt", eye, 1.0, 1.0))
        );
    }

    #[test]
    fn each_line_becomes_its_vanilla_message() {
        assert_eq!(
            sound_at(1.0, 0.5).message(),
            ToClient::Sound {
                event: "minecraft:block.stone.place".to_owned(),
                pos: [1.0, 64.0, 0.0],
                volume: 0.5,
                pitch: 1.0,
                seed: 0,
            }
        );
        let totem = Heard::Player {
            client: 2,
            event: TOTEM_EVENT,
            at: FEET,
        };
        assert_eq!(
            totem.message(),
            ToClient::PlayerEvent {
                client: 2,
                event: TOTEM_EVENT
            }
        );
        let lid = Heard::Block {
            pos: [1, 2, 3],
            kind: 1,
            param: 2,
        };
        assert_eq!(
            lid.message(),
            ToClient::BlockEvent {
                pos: [1, 2, 3],
                kind: 1,
                param: 2
            }
        );
    }
}
