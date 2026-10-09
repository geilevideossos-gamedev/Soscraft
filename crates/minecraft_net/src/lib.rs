use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum Dimension {
    #[default]
    #[serde(rename = "minecraft:overworld")]
    Overworld,
    #[serde(rename = "minecraft:the_nether")]
    Nether,
    #[serde(rename = "minecraft:the_end")]
    End,
}

impl Dimension {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name.trim_start_matches("minecraft:") {
            "overworld" => Self::Overworld,
            "nether" | "the_nether" => Self::Nether,
            "end" | "the_end" => Self::End,
            _ => return None,
        })
    }

    pub fn dimension_type(self) -> &'static str {
        match self {
            Self::Overworld => "minecraft:overworld",
            Self::Nether => "minecraft:the_nether",
            Self::End => "minecraft:the_end",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerUuid(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Dimension; 3] = [Dimension::Overworld, Dimension::Nether, Dimension::End];

    #[test]
    fn dimension_types_parse_back() {
        for dimension in ALL {
            assert_eq!(
                Dimension::parse(dimension.dimension_type()),
                Some(dimension)
            );
        }
        assert_eq!(Dimension::parse("overworld"), Some(Dimension::Overworld));
        assert_eq!(Dimension::parse("nether"), Some(Dimension::Nether));
        assert_eq!(Dimension::parse("the_end"), Some(Dimension::End));
        assert_eq!(Dimension::parse("minecraft:the_moon"), None);
        assert_eq!(Dimension::default(), Dimension::Overworld);
    }

    #[test]
    fn dimensions_and_uuids_survive_serde() {
        for dimension in ALL {
            let json = serde_json::to_string(&dimension).unwrap();
            assert_eq!(json, format!("\"{}\"", dimension.dimension_type()));
            assert_eq!(serde_json::from_str::<Dimension>(&json).unwrap(), dimension);
        }
        let uuid = PlayerUuid("b50ad385-829d-3141-a216-7e7d7539ba7f".to_string());
        let json = serde_json::to_string(&uuid).unwrap();
        assert_eq!(json, "\"b50ad385-829d-3141-a216-7e7d7539ba7f\"");
        assert_eq!(serde_json::from_str::<PlayerUuid>(&json).unwrap(), uuid);
    }
}
