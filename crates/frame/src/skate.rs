use bevy::prelude::*;

/// Local skating presentation. Physics remains owned by the optional Skate host.
#[derive(Resource, Default)]
pub struct SkateMode {
    pub active: bool,
    pub airborne: bool,
    pub entering: bool,
    pub preloaded: bool,
    pub preload_pending: bool,
    pub controller: Option<usize>,
    pub keyboard: bool,
    pub toggle_requested: bool,
    pub refused: bool,
    pub input_blocked: bool,
    pub client: u32,
    pub root: Mat4,
    pub bones: Vec<Mat4>,
    pub names: Vec<String>,
    pub camera: Option<(Transform, f32)>,
    pub tick: u64,
    pub status: String,
}

pub const SKATE_BONES: [&str; 23] = [
    "HIPS",
    "SPINE",
    "SPINE1",
    "SPINE3",
    "NECK",
    "HEAD",
    "LEFTSHOULDER",
    "LEFTARM",
    "LEFTFOREARM",
    "LEFTHAND",
    "RIGHTSHOULDER",
    "RIGHTARM",
    "RIGHTFOREARM",
    "RIGHTHAND",
    "LEFTUPLEG",
    "LEFTLEG",
    "LEFTFOOT",
    "LEFTTOEBASE",
    "RIGHTUPLEG",
    "RIGHTLEG",
    "RIGHTFOOT",
    "RIGHTTOEBASE",
    "SKATEBOARD_ROOT",
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkatePose {
    pub root: Mat4,
    pub bones: Vec<Mat4>,
}

impl SkatePose {
    pub fn from_named(root: Mat4, names: &[String], bones: &[Mat4]) -> Option<Self> {
        let bones = SKATE_BONES
            .iter()
            .map(|bone| {
                let index = names.iter().position(|name| name == bone)?;
                bones.get(index).copied()
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self { root, bones })
    }

    pub fn of(mode: &SkateMode) -> Option<Self> {
        if !mode.active || mode.bones.is_empty() {
            return None;
        }
        Self::from_named(mode.root, &mode.names, &mode.bones)
    }
}

#[derive(Resource, Default)]
pub struct SkatePoses(pub std::collections::BTreeMap<u32, SkatePose>);
