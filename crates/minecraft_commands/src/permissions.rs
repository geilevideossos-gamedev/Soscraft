use std::path::Path;

use serde::{Deserialize, Serialize};

pub const ALL: u8 = 0;
pub const MODERATORS: u8 = 1;
pub const GAMEMASTERS: u8 = 2;
pub const ADMINS: u8 = 3;
pub const OWNERS: u8 = 4;
pub const OPS_FILE: &str = "ops.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpEntry {
    pub uuid: String,
    pub name: String,
    pub level: u8,
    #[serde(rename = "bypassesPlayerLimit")]
    pub bypasses_player_limit: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpList(pub Vec<OpEntry>);

impl OpList {
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) if text.trim().is_empty() => Ok(Self::default()),
            Ok(text) => serde_json::from_str(&text)
                .map(Self)
                .map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.0).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn level_of(&self, uuid: &str) -> Option<u8> {
        self.0
            .iter()
            .find(|entry| entry.uuid.eq_ignore_ascii_case(uuid))
            .map(|entry| entry.level)
    }

    pub fn add(&mut self, uuid: &str, name: &str, level: u8) {
        self.remove(uuid);
        self.0.push(OpEntry {
            uuid: uuid.to_owned(),
            name: name.to_owned(),
            level,
            bypasses_player_limit: false,
        });
    }

    pub fn remove(&mut self, uuid: &str) -> bool {
        let before = self.0.len();
        self.0
            .retain(|entry| !entry.uuid.eq_ignore_ascii_case(uuid));
        self.0.len() < before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_follow_vanilla_permission_names() {
        assert_eq!(
            [ALL, MODERATORS, GAMEMASTERS, ADMINS, OWNERS],
            [0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn ops_json_round_trips_with_vanilla_field_names() {
        let dir = std::env::temp_dir().join(format!(
            "minecraft_commands-ops-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(OPS_FILE);
        assert_eq!(OpList::load(&path), Ok(OpList::default()));
        let mut ops = OpList::default();
        ops.add("b50ad385-829d-3141-a216-7e7d7539ba7f", "Notch", OWNERS);
        ops.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            json,
            serde_json::json!([{
                "uuid": "b50ad385-829d-3141-a216-7e7d7539ba7f",
                "name": "Notch",
                "level": 4,
                "bypassesPlayerLimit": false
            }])
        );
        let loaded = OpList::load(&path).unwrap();
        assert_eq!(loaded, ops);
        assert_eq!(
            loaded.level_of("B50AD385-829D-3141-A216-7E7D7539BA7F"),
            Some(4)
        );
        let mut changed = loaded;
        changed.add("b50ad385-829d-3141-a216-7e7d7539ba7f", "Notch", 2);
        assert_eq!(changed.0.len(), 1);
        assert_eq!(
            changed.level_of("b50ad385-829d-3141-a216-7e7d7539ba7f"),
            Some(2)
        );
        assert!(changed.remove("b50ad385-829d-3141-a216-7e7d7539ba7f"));
        assert!(!changed.remove("b50ad385-829d-3141-a216-7e7d7539ba7f"));
        std::fs::write(&path, "[").unwrap();
        assert!(OpList::load(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
