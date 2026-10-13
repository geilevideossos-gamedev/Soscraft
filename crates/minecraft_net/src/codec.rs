use std::fmt;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::messages::ChunkBlob;

pub const RELIABLE_LIMIT: usize = 4 * 1024 * 1024;
pub const BULK_LIMIT: usize = 256 * 1024;
pub const STATE_LIMIT: usize = 256 * 1024;
pub const BATCH_CUT: usize = 128 * 1024;
pub const BATCH_MAGIC: [u8; 2] = *b"MC";
pub const BATCH_VERSION: u8 = 1;
const BATCH_HEADER: usize = 2 + 1 + 4 + 2;
const CHUNK_HEADER: usize = 4 + 4 + 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Reliable,
    Bulk,
    State,
}

impl Lane {
    pub const fn limit(self) -> usize {
        match self {
            Self::Reliable => RELIABLE_LIMIT,
            Self::Bulk => BULK_LIMIT,
            Self::State => STATE_LIMIT,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodecError {
    TooLarge { len: usize, limit: usize },
    Trailing { extra: usize },
    Malformed(String),
    BadMagic,
    BadVersion(u8),
    TooManyChunks(usize),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { len, limit } => {
                write!(f, "{len} bytes is over the lane's limit of {limit}")
            }
            Self::Trailing { extra } => write!(f, "{extra} bytes left over after the message"),
            Self::Malformed(error) => write!(f, "malformed message: {error}"),
            Self::BadMagic => write!(f, "not a Minecraft chunk batch"),
            Self::BadVersion(version) => write!(f, "chunk batch version {version} is unknown"),
            Self::TooManyChunks(count) => write!(f, "{count} chunks do not fit in one batch"),
        }
    }
}

impl std::error::Error for CodecError {}

fn fits(len: usize, limit: usize) -> Result<(), CodecError> {
    if len > limit {
        return Err(CodecError::TooLarge { len, limit });
    }
    Ok(())
}

pub fn encode<T: Serialize>(lane: Lane, value: &T) -> Result<Vec<u8>, CodecError> {
    let bytes =
        postcard::to_stdvec(value).map_err(|error| CodecError::Malformed(error.to_string()))?;
    fits(bytes.len(), lane.limit())?;
    Ok(bytes)
}

pub fn decode<T: DeserializeOwned>(lane: Lane, bytes: &[u8]) -> Result<T, CodecError> {
    fits(bytes.len(), lane.limit())?;
    let (value, rest) = postcard::take_from_bytes(bytes)
        .map_err(|error| CodecError::Malformed(error.to_string()))?;
    if !rest.is_empty() {
        return Err(CodecError::Trailing { extra: rest.len() });
    }
    Ok(value)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChunkBatchBlob {
    pub seq: u32,
    pub chunks: Vec<([i32; 2], ChunkBlob)>,
}

impl ChunkBatchBlob {
    pub fn encoded_len(&self) -> usize {
        BATCH_HEADER
            + self
                .chunks
                .iter()
                .map(|(_, chunk)| CHUNK_HEADER + chunk.0.len())
                .sum::<usize>()
    }

    pub fn encode(&self) -> Result<Vec<u8>, CodecError> {
        let count = u16::try_from(self.chunks.len())
            .map_err(|_| CodecError::TooManyChunks(self.chunks.len()))?;
        let len = self.encoded_len();
        fits(len, BULK_LIMIT)?;
        let mut out = Vec::with_capacity(len);
        out.extend_from_slice(&BATCH_MAGIC);
        out.push(BATCH_VERSION);
        out.extend_from_slice(&self.seq.to_be_bytes());
        out.extend_from_slice(&count.to_be_bytes());
        for ([x, z], chunk) in &self.chunks {
            let size = u32::try_from(chunk.0.len()).map_err(|_| CodecError::TooLarge {
                len: chunk.0.len(),
                limit: BULK_LIMIT,
            })?;
            out.extend_from_slice(&x.to_be_bytes());
            out.extend_from_slice(&z.to_be_bytes());
            out.extend_from_slice(&size.to_be_bytes());
            out.extend_from_slice(&chunk.0);
        }
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        fits(bytes.len(), BULK_LIMIT)?;
        let mut reader = Reader { bytes };
        if reader.take(2)? != BATCH_MAGIC {
            return Err(CodecError::BadMagic);
        }
        let version = reader.take(1)?[0];
        if version != BATCH_VERSION {
            return Err(CodecError::BadVersion(version));
        }
        let seq = reader.u32()?;
        let count = usize::from(u16::from_be_bytes(reader.array()?));
        let mut chunks = Vec::with_capacity(count.min(bytes.len() / CHUNK_HEADER));
        for _ in 0..count {
            let x = i32::from_be_bytes(reader.array()?);
            let z = i32::from_be_bytes(reader.array()?);
            let size = reader.u32()? as usize;
            chunks.push(([x, z], ChunkBlob(reader.take(size)?.to_vec())));
        }
        if !reader.bytes.is_empty() {
            return Err(CodecError::Trailing {
                extra: reader.bytes.len(),
            });
        }
        Ok(Self { seq, chunks })
    }

    pub fn peek_seq(bytes: &[u8]) -> Option<u32> {
        let header = bytes.get(..BATCH_HEADER)?;
        (header[..2] == BATCH_MAGIC && header[2] == BATCH_VERSION)
            .then(|| u32::from_be_bytes([header[3], header[4], header[5], header[6]]))
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        if self.bytes.len() < len {
            return Err(CodecError::Malformed(format!(
                "chunk batch ends {} bytes early",
                len - self.bytes.len()
            )));
        }
        let (head, rest) = self.bytes.split_at(len);
        self.bytes = rest;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        let mut out = [0; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_be_bytes(self.array()?))
    }
}

#[cfg(test)]
mod tests {
    use minecraft_commands::Text;
    use minecraft_commands::protocol::{Clientbound, PlayerId, Serverbound};

    use super::*;
    use crate::messages::*;
    use crate::{Dimension, PlayerUuid};

    fn block() -> Target {
        Target::Block {
            pos: [1, -60, 3],
            face: 4,
            hit: [0.5, 0.25, 1.0],
        }
    }

    fn server_index(message: &ToServer) -> usize {
        match message {
            ToServer::Hello { .. } => 0,
            ToServer::Ack => 1,
            ToServer::ChunkBatchReceived { .. } => 2,
            ToServer::Hand(_) => 3,
            ToServer::SelectSlot(_) => 4,
            ToServer::Click(_) => 5,
            ToServer::RenameItem(_) => 6,
            ToServer::CloseMenu { .. } => 7,
            ToServer::Chat(_) => 8,
            ToServer::CreditsDone => 9,
            ToServer::Lectern { .. } => 10,
            ToServer::LeaveBed => 11,
            ToServer::EditBook { .. } => 12,
            ToServer::CreativeSlot { .. } => 13,
        }
    }

    fn client_index(message: &ToClient) -> usize {
        match message {
            ToClient::Login(_) => 0,
            ToClient::Rejected { .. } => 1,
            ToClient::ChunkBatch { .. } => 2,
            ToClient::ChunkForget { .. } => 3,
            ToClient::BlockUpdates { .. } => 4,
            ToClient::BlockEntity { .. } => 5,
            ToClient::BlockEvent { .. } => 6,
            ToClient::Crack { .. } => 7,
            ToClient::LevelEvent { .. } => 8,
            ToClient::Sound { .. } => 9,
            ToClient::Explosion { .. } => 10,
            ToClient::MobAdd { .. } => 11,
            ToClient::MobLook { .. } => 12,
            ToClient::MobRemove { .. } => 13,
            ToClient::Player(_) => 14,
            ToClient::MenuOpen(_) => 15,
            ToClient::MenuSlots { .. } => 16,
            ToClient::MenuClose { .. } => 17,
            ToClient::Equipment { .. } => 18,
            ToClient::Chat(_) => 19,
            ToClient::PlayerList { .. } => 20,
            ToClient::Time { .. } => 21,
            ToClient::Weather { .. } => 22,
            ToClient::Dimension { .. } => 23,
            ToClient::BossBar(_) => 24,
            ToClient::KilledBy { .. } => 25,
            ToClient::Credits => 26,
            ToClient::PlayerEvent { .. } => 27,
            ToClient::Bursts { .. } => 28,
            ToClient::Lectern(_) => 29,
            ToClient::Sleeping { .. } => 30,
            ToClient::OpenBook { .. } => 31,
        }
    }

    fn event(seq: u32, kind: HandEventKind, target: Target) -> HandEvent {
        HandEvent { seq, kind, target }
    }

    fn full_book() -> Vec<String> {
        (0..100)
            .map(|page| {
                format!("Page {page} \u{e9}\n")
                    .repeat(200)
                    .chars()
                    .take(1024)
                    .collect()
            })
            .collect()
    }

    fn to_server() -> Vec<ToServer> {
        vec![
            ToServer::hello(8, 0xdead_beef_cafe),
            ToServer::Ack,
            ToServer::ChunkBatchReceived { seq: 77 },
            ToServer::Hand(vec![
                event(1, HandEventKind::AttackPressed, block()),
                event(2, HandEventKind::AttackReleased, Target::Miss),
                event(3, HandEventKind::UsePressed, Target::Mob { id: u64::MAX }),
                event(4, HandEventKind::UseReleased, Target::Player { client: 2 }),
                event(5, HandEventKind::SwapHands, Target::Miss),
                event(6, HandEventKind::Drop { whole_stack: true }, Target::Miss),
            ]),
            ToServer::SelectSlot(8),
            ToServer::Click(Click {
                menu: 100,
                state: 0x7fff,
                action: MenuAction::Spread {
                    slots: vec![MenuSlot::Inventory(40), MenuSlot::Container(53)],
                    right: true,
                },
            }),
            ToServer::Click(Click {
                menu: 0,
                state: 1,
                action: MenuAction::Swap {
                    slot: MenuSlot::Crafting(3),
                    hotbar: 40,
                },
            }),
            ToServer::RenameItem("Pick \u{f6}".to_string()),
            ToServer::CloseMenu { menu: 3 },
            ToServer::Chat(Serverbound::Command("time set day".to_string())),
            ToServer::CreditsDone,
            ToServer::Lectern {
                pos: [4, 70, -9],
                button: Some(102),
            },
            ToServer::Lectern {
                pos: [4, 70, -9],
                button: None,
            },
            ToServer::LeaveBed,
            ToServer::EditBook {
                slot: 40,
                pages: full_book(),
                title: Some("Diary".to_string()),
            },
            ToServer::EditBook {
                slot: 0,
                pages: Vec::new(),
                title: None,
            },
            ToServer::CreativeSlot {
                slot: CreativeTarget::Menu(36),
                stack: Some(Nbt(vec![10, 0, 0, 8, 0, 2, b'i', b'd', 0])),
            },
            ToServer::CreativeSlot {
                slot: CreativeTarget::Carried,
                stack: None,
            },
            ToServer::CreativeSlot {
                slot: CreativeTarget::Drop,
                stack: Some(Nbt(vec![10, 0, 0, 0])),
            },
        ]
    }

    fn player_view() -> PlayerView {
        let mut view = PlayerView {
            health: 13.5,
            absorption: 4.0,
            food: 17,
            saturation: 2.5,
            air: -20,
            fire_ticks: 80,
            frozen: 140,
            armor: 15,
            xp_level: 30,
            xp_progress: 0.75,
            xp_total: 1395,
            game_mode: GameMode::Creative,
            abilities: Abilities {
                flying: true,
                may_fly: true,
                instabuild: true,
                ..Abilities::default()
            },
            effects: vec![EffectView {
                kind: "minecraft:speed".to_string(),
                amplifier: 1,
                duration: -1,
                ambient: true,
            }],
            selected: 4,
            cooldowns: vec![CooldownView {
                group: "minecraft:ender_pearl".to_string(),
                remaining: 12,
                total: 20,
            }],
            using: Some(UsingView {
                hand: Hand::Off,
                ticks: 9,
            }),
            ..PlayerView::default()
        };
        view.slots[0] = Some(Nbt(vec![10, 0, 0, 8, 0, 2, b'i', b'd', 0]));
        view.slots[40] = Some(Nbt(vec![0]));
        view
    }

    fn menu(menu: u8, kind: MenuKind, title: Text, extras: MenuExtras) -> ToClient {
        ToClient::MenuOpen(MenuView {
            menu,
            kind,
            title,
            state: 1,
            slots: vec![None, Some(Nbt(vec![3]))],
            carried: Some(Nbt(vec![1])),
            data: vec![(0, 5), (9, -1)],
            extras,
        })
    }

    fn boss(operation: BossBarOperation) -> ToClient {
        ToClient::BossBar(BossBarUpdate { id: 1, operation })
    }

    fn to_client() -> Vec<ToClient> {
        let helmet = GearItem {
            id: "minecraft:leather_helmet".to_string(),
            glint: true,
            dye: Some(0xA06540),
            potion: None,
        };
        vec![
            ToClient::Login(Login {
                client: 1,
                uuid: PlayerUuid("b50ad385-829d-3141-a216-7e7d7539ba7f".to_string()),
                seed: i64::MIN,
                dimension: Dimension::Nether,
                origins: vec![
                    (Dimension::Overworld, [12.0, 64.0, -7.0]),
                    (Dimension::Nether, [1.5, 1314.0, -0.875]),
                    (Dimension::End, [0.0, -1140.0, 0.0]),
                ],
                feet: [0.5, 65.0, 0.5],
                view_distance: 8,
                rules: Rules {
                    pvp: false,
                    ..Rules::default()
                },
                registry: u64::MAX,
            }),
            ToClient::Rejected {
                reason: HelloRefusal::Registry.reason(),
            },
            ToClient::ChunkBatch { seq: 3 },
            ToClient::ChunkForget {
                pos: [-30_000_000, 5],
            },
            ToClient::BlockUpdates {
                changes: vec![([0, -64, 0], 0), ([15, 319, -1], 29_000)],
            },
            ToClient::BlockEntity {
                pos: [4, 70, 4],
                tag: Nbt(vec![10, 0, 0, 0]),
            },
            ToClient::BlockEvent {
                pos: [4, 70, 4],
                kind: 1,
                param: 1,
            },
            ToClient::Crack {
                breaker: 0,
                pos: [2, 3, 4],
                stage: -1,
            },
            ToClient::LevelEvent {
                event: 2001,
                pos: [2, 3, 4],
                data: 1,
            },
            ToClient::Sound {
                event: "minecraft:block.chest.open".to_string(),
                pos: [4.5, 70.5, 4.5],
                volume: 0.5,
                pitch: 0.9,
                seed: -3,
            },
            ToClient::Explosion {
                center: [1.0, 2.0, 3.0],
                radius: 4.0,
                blocks: 12,
                interacts: true,
            },
            ToClient::MobAdd {
                id: 9,
                tag: Nbt(vec![10, 0, 0, 0]),
            },
            ToClient::MobLook {
                id: 9,
                look: Blob(vec![1, 2, 3]),
            },
            ToClient::MobRemove { ids: vec![9, 10] },
            ToClient::Player(Box::new(player_view())),
            menu(
                7,
                MenuKind::Chest { rows: 6 },
                Text::translate("container.chestDouble", Vec::new()),
                MenuExtras::None,
            ),
            menu(
                8,
                MenuKind::Enchanting,
                Text::literal("Enchant"),
                MenuExtras::Enchanting {
                    seed: -5,
                    shelves: 15,
                },
            ),
            menu(
                9,
                MenuKind::Anvil,
                Text::literal("Repair & Name"),
                MenuExtras::Anvil {
                    cost: 3,
                    name: Some("Pick".to_string()),
                },
            ),
            ToClient::MenuSlots {
                menu: 7,
                state: 2,
                slots: vec![(0, None), (89, Some(Nbt(vec![2])))],
                carried: Some(None),
                data: vec![(1, 200)],
            },
            ToClient::MenuClose { menu: 7 },
            ToClient::Equipment {
                client: 2,
                gear: GearView {
                    main: Some(GearItem {
                        id: "minecraft:diamond_sword".to_string(),
                        glint: false,
                        dye: None,
                        potion: None,
                    }),
                    off: Some(GearItem {
                        id: "minecraft:potion".to_string(),
                        glint: false,
                        dye: None,
                        potion: Some(0xF82423),
                    }),
                    armor: [Some(helmet), None, None, None],
                    hides_gun: true,
                },
            },
            ToClient::Chat(Clientbound::Player {
                sender: PlayerId(1),
                name: "Friend".to_string(),
                message: "hello".to_string(),
            }),
            ToClient::Chat(Clientbound::System {
                text: Text::translate("multiplayer.player.joined", vec![Text::literal("Friend")])
                    .color(minecraft_commands::text::YELLOW),
                overlay: false,
            }),
            ToClient::PlayerList {
                added: vec![(
                    1,
                    "Friend".to_string(),
                    PlayerUuid("00000000-0000-3000-8000-000000000000".to_string()),
                )],
                removed: vec![3],
            },
            ToClient::Time {
                game_time: 1_234_567,
                clocks: vec![("minecraft:overworld".to_string(), 6000.5, false)],
            },
            ToClient::Weather {
                rain: 1.0,
                thunder: 0.25,
            },
            ToClient::Dimension {
                epoch: 2,
                dimension: Dimension::End,
                origin: [0.0, -1140.0, 0.0],
                feet: [100.5, 49.0, 0.5],
            },
            boss(BossBarOperation::Add {
                name: Text::entity("minecraft:ender_dragon"),
                progress: 1.0,
                color: BossBarColor::Pink,
                overlay: BossBarOverlay::Progress,
                flags: BossBarFlags {
                    darken_screen: false,
                    play_music: true,
                    create_world_fog: true,
                },
            }),
            boss(BossBarOperation::Progress(0.5)),
            boss(BossBarOperation::Name(Text::literal("Dragon"))),
            boss(BossBarOperation::Style {
                color: BossBarColor::White,
                overlay: BossBarOverlay::Notched20,
            }),
            boss(BossBarOperation::Flags(BossBarFlags::default())),
            boss(BossBarOperation::Remove),
            ToClient::KilledBy {
                killer: Some("Friend".to_string()),
                line: Text::translate(
                    "death.attack.player",
                    vec![Text::literal("Sos"), Text::literal("Friend")],
                ),
            },
            ToClient::Credits,
            ToClient::PlayerEvent {
                client: 2,
                event: 35,
            },
            ToClient::Bursts {
                entities: Blob(vec![4, 5, 6]),
            },
            ToClient::Lectern(Some(BookView {
                pos: [4, 70, -9],
                pages: vec!["Once upon a time".to_string(), String::new()],
                page: 1,
                take: true,
            })),
            ToClient::Lectern(None),
            ToClient::Sleeping {
                client: 2,
                bed: Some(Lying {
                    feet: [10.5, 64.6875, -3.5],
                    facing: [0, -1],
                }),
                timer: 0,
            },
            ToClient::Sleeping {
                client: 2,
                bed: None,
                timer: 100,
            },
            ToClient::OpenBook { slot: 40 },
        ]
    }

    fn round_trip<T>(lane: Lane, value: &T)
    where
        T: Serialize + DeserializeOwned + PartialEq + fmt::Debug,
    {
        let bytes = encode(lane, value).unwrap();
        assert_eq!(&decode::<T>(lane, &bytes).unwrap(), value);
    }

    #[test]
    fn every_message_to_the_server_round_trips() {
        let messages = to_server();
        let mut seen = [false; 14];
        for message in &messages {
            seen[server_index(message)] = true;
            round_trip(Lane::Reliable, message);
        }
        assert!(seen.iter().all(|&seen| seen));
        round_trip(
            Lane::Reliable,
            &ToServerFrame {
                frame: 5,
                ack: 4,
                messages,
            },
        );
    }

    #[test]
    fn every_message_to_the_client_round_trips() {
        let messages = to_client();
        let mut seen = [false; 32];
        for message in &messages {
            seen[client_index(message)] = true;
            round_trip(Lane::Reliable, message);
        }
        assert!(seen.iter().all(|&seen| seen));
        round_trip(
            Lane::Reliable,
            &ToClientFrame {
                frame: u32::MAX,
                ack: 0,
                messages,
            },
        );
    }

    #[test]
    fn book_messages_take_the_next_free_indices() {
        let edit = ToServer::EditBook {
            slot: 40,
            pages: full_book(),
            title: Some("Diary".to_string()),
        };
        assert_eq!(server_index(&edit), 12);
        assert_eq!(encode(Lane::Reliable, &edit).unwrap()[0], 12);
        let open = ToClient::OpenBook { slot: 8 };
        assert_eq!(client_index(&open), 31);
        assert_eq!(encode(Lane::Reliable, &open).unwrap(), vec![31, 8]);
        let bytes = encode(Lane::Reliable, &edit).unwrap();
        assert!(bytes.len() > 100 * 1024 && bytes.len() < RELIABLE_LIMIT);
        let ToServer::EditBook { pages, title, .. } = decode(Lane::Reliable, &bytes).unwrap()
        else {
            panic!("not an edit");
        };
        assert_eq!(pages.len(), 100);
        assert!(pages.iter().all(|page| page.chars().count() == 1024));
        assert_eq!(title.as_deref(), Some("Diary"));
    }

    #[test]
    fn a_held_potion_colour_round_trips_on_protocol_4() {
        assert_eq!(crate::PROTOCOL, 4);
        let potion = GearItem {
            id: "minecraft:splash_potion".to_string(),
            glint: false,
            dye: None,
            potion: Some(0x87A363),
        };
        round_trip(Lane::Reliable, &potion);
        let bytes = encode(Lane::Reliable, &potion).unwrap();
        let back: GearItem = decode(Lane::Reliable, &bytes).unwrap();
        assert_eq!(back.potion, Some(0x87A363));
    }

    #[test]
    fn state_lane_messages_round_trip() {
        round_trip(
            Lane::State,
            &HandState {
                tick: 99,
                attack: true,
                use_item: false,
                target: block(),
            },
        );
        round_trip(
            Lane::State,
            &StateFrame {
                tick: 100,
                epoch: 3,
                mobs: Blob(vec![7; 2000]),
                entities: Blob(Vec::new()),
            },
        );
    }

    #[test]
    fn oversize_input_is_refused() {
        let frame = StateFrame {
            mobs: Blob(vec![0; STATE_LIMIT]),
            ..StateFrame::default()
        };
        assert!(matches!(
            encode(Lane::State, &frame),
            Err(CodecError::TooLarge {
                limit: STATE_LIMIT,
                ..
            })
        ));
        let bytes = encode(Lane::Reliable, &frame).unwrap();
        assert!(matches!(
            decode::<StateFrame>(Lane::State, &bytes),
            Err(CodecError::TooLarge { .. })
        ));
        assert_eq!(decode::<StateFrame>(Lane::Reliable, &bytes).unwrap(), frame);
        let huge = vec![0u8; RELIABLE_LIMIT + 1];
        assert_eq!(
            decode::<ToServerFrame>(Lane::Reliable, &huge),
            Err(CodecError::TooLarge {
                len: RELIABLE_LIMIT + 1,
                limit: RELIABLE_LIMIT,
            })
        );
    }

    #[test]
    fn trailing_and_truncated_bytes_are_refused() {
        let mut bytes = encode(Lane::Reliable, &ToServer::Ack).unwrap();
        bytes.extend_from_slice(&[0, 1]);
        assert_eq!(
            decode::<ToServer>(Lane::Reliable, &bytes),
            Err(CodecError::Trailing { extra: 2 })
        );
        let bytes = encode(Lane::Reliable, &ToServer::RenameItem("name".into())).unwrap();
        assert!(matches!(
            decode::<ToServer>(Lane::Reliable, &bytes[..bytes.len() - 1]),
            Err(CodecError::Malformed(_))
        ));
        assert!(matches!(
            decode::<ToServer>(Lane::Reliable, &[200]),
            Err(CodecError::Malformed(_))
        ));
    }

    #[test]
    fn a_long_count_with_no_data_behind_it_is_refused() {
        let bytes = [3, 0xff, 0xff, 0xff, 0xff, 0x0f];
        assert!(matches!(
            decode::<ToServer>(Lane::Reliable, &bytes),
            Err(CodecError::Malformed(_))
        ));
    }

    fn batch() -> ChunkBatchBlob {
        ChunkBatchBlob {
            seq: 0x0102_0304,
            chunks: vec![
                ([-1, 2], ChunkBlob(vec![2, 0x78, 0x9c, 1, 2, 3])),
                ([i32::MIN, i32::MAX], ChunkBlob(Vec::new())),
            ],
        }
    }

    #[test]
    fn the_chunk_batch_header_round_trips() {
        let batch = batch();
        let bytes = batch.encode().unwrap();
        assert_eq!(&bytes[..9], &[b'M', b'C', 1, 1, 2, 3, 4, 0, 2]);
        assert_eq!(&bytes[9..21], &[255, 255, 255, 255, 0, 0, 0, 2, 0, 0, 0, 6]);
        assert_eq!(bytes.len(), batch.encoded_len());
        assert_eq!(ChunkBatchBlob::peek_seq(&bytes), Some(0x0102_0304));
        assert_eq!(ChunkBatchBlob::decode(&bytes).unwrap(), batch);
        let empty = ChunkBatchBlob::default();
        assert_eq!(
            ChunkBatchBlob::decode(&empty.encode().unwrap()).unwrap(),
            empty
        );
    }

    #[test]
    fn broken_chunk_batches_are_refused() {
        let bytes = batch().encode().unwrap();
        let mut magic = bytes.clone();
        magic[0] = b'B';
        assert_eq!(ChunkBatchBlob::decode(&magic), Err(CodecError::BadMagic));
        assert_eq!(ChunkBatchBlob::peek_seq(&magic), None);
        let mut version = bytes.clone();
        version[2] = 9;
        assert_eq!(
            ChunkBatchBlob::decode(&version),
            Err(CodecError::BadVersion(9))
        );
        assert!(matches!(
            ChunkBatchBlob::decode(&bytes[..bytes.len() - 1]),
            Err(CodecError::Malformed(_))
        ));
        assert!(matches!(
            ChunkBatchBlob::decode(&bytes[..4]),
            Err(CodecError::Malformed(_))
        ));
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            ChunkBatchBlob::decode(&trailing),
            Err(CodecError::Trailing { extra: 1 })
        );
        let mut lying = bytes;
        lying[7] = 0xff;
        lying[8] = 0xff;
        assert!(matches!(
            ChunkBatchBlob::decode(&lying),
            Err(CodecError::Malformed(_))
        ));
    }

    #[test]
    fn a_chunk_batch_over_the_bulk_limit_is_refused() {
        let batch = ChunkBatchBlob {
            seq: 1,
            chunks: vec![([0, 0], ChunkBlob(vec![0; BULK_LIMIT]))],
        };
        assert!(matches!(
            batch.encode(),
            Err(CodecError::TooLarge {
                limit: BULK_LIMIT,
                ..
            })
        ));
        assert!(matches!(
            ChunkBatchBlob::decode(&vec![0; BULK_LIMIT + 1]),
            Err(CodecError::TooLarge { .. })
        ));
        const { assert!(BATCH_CUT < BULK_LIMIT) };
    }
}
