use crate::model::{Element, ElementRotation, Face, ResolvedModel};
use crate::pack::ResourceId;
use crate::scene::Block;
use anyhow::Result;
use glam::{Mat3, Mat4, Quat, Vec3};
use serde_json::Value;
use std::f32::consts::{FRAC_PI_2, PI};

pub(crate) const ENTITY_SHEET_ALPHA: f32 = 0.5;

const UV_INSET: f32 = 1.0 / 32.0;
const SHEET_HALF_THICKNESS: f32 = 0.01;
const ALL_FACES: [&str; 6] = ["down", "up", "west", "north", "east", "south"];
const SHULKER_SCALE: f32 = 0.9995;
const BANNER_SCALE: f32 = 2.0 / 3.0;
const BANNER_FLAG_ANGLE: f32 = (-0.0125 + 0.01) * PI;
const DYES: [(&str, u32); 16] = [
    ("white", 16383998),
    ("orange", 16351261),
    ("magenta", 13061821),
    ("light_blue", 3847130),
    ("yellow", 16701501),
    ("lime", 8439583),
    ("pink", 15961002),
    ("gray", 4673362),
    ("light_gray", 10329495),
    ("cyan", 1481884),
    ("purple", 8991416),
    ("blue", 3949738),
    ("brown", 8606770),
    ("green", 6192150),
    ("red", 11546150),
    ("black", 1908001),
];
const COPPER_CHESTS: [(&str, &str); 4] = [
    ("copper_chest", "copper"),
    ("exposed_copper_chest", "copper_exposed"),
    ("weathered_copper_chest", "copper_weathered"),
    ("oxidized_copper_chest", "copper_oxidized"),
];
const SUPPORTED_SPECIALS: [&str; 8] = [
    "minecraft:chest",
    "minecraft:shulker_box",
    "minecraft:head",
    "minecraft:player_head",
    "minecraft:banner",
    "minecraft:conduit",
    "minecraft:decorated_pot",
    "minecraft:shield",
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct Cube {
    uv: [f32; 2],
    origin: [f32; 3],
    size: [f32; 3],
    inflate: f32,
    mirror: bool,
}

impl Cube {
    const fn new(uv: [f32; 2], origin: [f32; 3], size: [f32; 3]) -> Self {
        Self {
            uv,
            origin,
            size,
            inflate: 0.0,
            mirror: false,
        }
    }

    const fn inflated(self, inflate: f32) -> Self {
        Self { inflate, ..self }
    }

    const fn mirrored(self) -> Self {
        Self {
            mirror: true,
            ..self
        }
    }
}

struct Sheet {
    texture: ResourceId,
    size: [f32; 2],
    tinted: bool,
}

impl Sheet {
    fn new(texture: &str, size: [f32; 2]) -> Result<Self> {
        Ok(Self {
            texture: ResourceId::parse(texture)?,
            size,
            tinted: false,
        })
    }

    fn tinted(self) -> Self {
        Self {
            tinted: true,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Head {
    Skeleton,
    WitherSkeleton,
    Zombie,
    Creeper,
    Dragon,
    Piglin,
    Player,
}

impl Head {
    fn of_block(path: &str) -> Option<(Self, bool)> {
        Some(match path {
            "skeleton_skull" => (Self::Skeleton, false),
            "skeleton_wall_skull" => (Self::Skeleton, true),
            "wither_skeleton_skull" => (Self::WitherSkeleton, false),
            "wither_skeleton_wall_skull" => (Self::WitherSkeleton, true),
            "zombie_head" => (Self::Zombie, false),
            "zombie_wall_head" => (Self::Zombie, true),
            "creeper_head" => (Self::Creeper, false),
            "creeper_wall_head" => (Self::Creeper, true),
            "dragon_head" => (Self::Dragon, false),
            "dragon_wall_head" => (Self::Dragon, true),
            "piglin_head" => (Self::Piglin, false),
            "piglin_wall_head" => (Self::Piglin, true),
            "player_head" => (Self::Player, false),
            "player_wall_head" => (Self::Player, true),
            _ => return None,
        })
    }

    fn of_kind(kind: &str) -> Option<Self> {
        Some(match kind {
            "skeleton" => Self::Skeleton,
            "wither_skeleton" => Self::WitherSkeleton,
            "zombie" => Self::Zombie,
            "creeper" => Self::Creeper,
            "dragon" => Self::Dragon,
            "piglin" => Self::Piglin,
            "player" => Self::Player,
            _ => return None,
        })
    }

    fn texture(self) -> &'static str {
        match self {
            Self::Skeleton => "minecraft:entity/skeleton/skeleton",
            Self::WitherSkeleton => "minecraft:entity/skeleton/wither_skeleton",
            Self::Zombie => "minecraft:entity/zombie/zombie",
            Self::Creeper => "minecraft:entity/creeper/creeper",
            Self::Dragon => "minecraft:entity/enderdragon/dragon",
            Self::Piglin => "minecraft:entity/piglin/piglin",
            Self::Player => "minecraft:entity/player/slim/steve",
        }
    }
}

pub(crate) struct ItemLeaf<'a> {
    pub(crate) model: &'a str,
    pub(crate) special: Option<&'a Value>,
    pub(crate) transform: Mat4,
}

pub(crate) fn unmipped_sheet(texture: &ResourceId) -> bool {
    texture.path.starts_with("entity/")
        && !["entity/bell/", "entity/conduit/", "entity/enchantment/"]
            .iter()
            .any(|prefix| texture.path.starts_with(prefix))
}

pub(crate) fn dye_color(name: &str) -> Option<[f32; 3]> {
    DYES.iter()
        .find(|(dye, _)| *dye == name)
        .map(|&(_, rgb)| [rgb >> 16, rgb >> 8, rgb].map(|channel| (channel & 255) as f32 / 255.0))
}

pub(crate) fn banner_dye(block: &Block) -> Option<[f32; 3]> {
    let path = block.id.path.as_str();
    let color = path
        .strip_suffix("_wall_banner")
        .or_else(|| path.strip_suffix("_banner"))?;
    dye_color(color)
}

pub(crate) fn is_chest(block: &Block) -> bool {
    matches!(
        block.id.path.as_str(),
        "chest" | "trapped_chest" | "ender_chest"
    ) || copper_chest_texture(&block.id.path).is_some()
}

fn copper_chest_texture(path: &str) -> Option<&'static str> {
    let path = path.strip_prefix("waxed_").unwrap_or(path);
    COPPER_CHESTS
        .iter()
        .find(|(block, _)| *block == path)
        .map(|&(_, texture)| texture)
}

pub(crate) fn chest_texture(path: &str, chest_type: &str) -> Result<ResourceId> {
    if path == "ender_chest" {
        return ResourceId::parse("minecraft:entity/chest/ender");
    }
    let base = match path {
        "trapped_chest" => "trapped",
        _ => copper_chest_texture(path).unwrap_or("normal"),
    };
    let suffix = match chest_type {
        "left" => "_left",
        "right" => "_right",
        _ => "",
    };
    ResourceId::parse(&format!("minecraft:entity/chest/{base}{suffix}"))
}

pub(crate) fn textures() -> Result<Vec<ResourceId>> {
    let mut names = vec![
        "entity/chest/ender".to_owned(),
        "entity/banner/banner_base".to_owned(),
        "entity/banner/base".to_owned(),
        "entity/bell/bell_body".to_owned(),
        "entity/conduit/base".to_owned(),
        "entity/decorated_pot/decorated_pot_base".to_owned(),
        "entity/decorated_pot/decorated_pot_side".to_owned(),
        "entity/enchantment/enchanting_table_book".to_owned(),
        "entity/shield/shield_base_nopattern".to_owned(),
        "entity/shulker/shulker".to_owned(),
    ];
    for base in ["normal", "trapped"]
        .into_iter()
        .chain(COPPER_CHESTS.iter().map(|&(_, texture)| texture))
    {
        for suffix in ["", "_left", "_right"] {
            names.push(format!("entity/chest/{base}{suffix}"));
        }
    }
    for (dye, _) in DYES {
        names.push(format!("entity/shulker/shulker_{dye}"));
    }
    let mut textures = names
        .iter()
        .map(|name| ResourceId::parse(&format!("minecraft:{name}")))
        .collect::<Result<Vec<_>>>()?;
    for head in [
        Head::Skeleton,
        Head::WitherSkeleton,
        Head::Zombie,
        Head::Creeper,
        Head::Dragon,
        Head::Piglin,
        Head::Player,
    ] {
        textures.push(ResourceId::parse(head.texture())?);
    }
    Ok(textures)
}

pub(crate) fn block_model(block: &Block) -> Result<Option<ResolvedModel>> {
    let path = block.id.path.as_str();
    let property = |key: &str| block.properties.get(key).map(String::as_str);
    let mut elements = Vec::new();
    if let Some((head, wall)) = Head::of_block(path) {
        let pose = if wall {
            let facing = property("facing").unwrap_or("north");
            let (step_x, step_z) = horizontal_step(facing);
            Mat4::from_translation(Vec3::new(0.5 - 0.25 * step_x, 0.25, 0.5 - 0.25 * step_z))
                * Mat4::from_rotation_y((-y_rot(opposite(facing))).to_radians())
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        } else {
            Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
                * Mat4::from_rotation_y((-segment_degrees(property("rotation"))).to_radians())
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        };
        push_head(&mut elements, head, None, pose)?;
    } else if path == "shulker_box" || path.ends_with("_shulker_box") {
        let texture = match path.strip_suffix("_shulker_box") {
            Some(color) => format!("minecraft:entity/shulker/shulker_{color}"),
            None => "minecraft:entity/shulker/shulker".to_owned(),
        };
        let facing = property("facing").unwrap_or("up");
        let pose = Mat4::from_translation(Vec3::splat(0.5))
            * Mat4::from_scale(Vec3::splat(SHULKER_SCALE))
            * direction_rotation(facing)
            * Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0))
            * Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0));
        push_shulker(&mut elements, Sheet::new(&texture, [64.0, 64.0])?, pose);
    } else if banner_dye(block).is_some() {
        let wall = path.ends_with("_wall_banner");
        let degrees = if wall {
            y_rot(property("facing").unwrap_or("north"))
        } else {
            segment_degrees(property("rotation"))
        };
        let pose = Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
            * Mat4::from_rotation_y((-degrees).to_radians())
            * Mat4::from_scale(Vec3::new(BANNER_SCALE, -BANNER_SCALE, -BANNER_SCALE));
        push_banner(&mut elements, !wall, pose)?;
    } else if path == "conduit" {
        push_conduit(&mut elements, Mat4::from_translation(Vec3::splat(0.5)))?;
    } else if path == "decorated_pot" {
        let facing = property("facing").unwrap_or("north");
        let pose = around_center(Mat4::from_rotation_y((180.0 - y_rot(facing)).to_radians()));
        push_decorated_pot(&mut elements, pose)?;
    } else {
        return Ok(None);
    }
    Ok(Some(ResolvedModel { elements }))
}

pub(crate) fn extra_elements(block: &Block) -> Result<Vec<Element>> {
    let mut elements = Vec::new();
    match block.id.path.as_str() {
        "bell" => push_bell(&mut elements)?,
        "enchanting_table" => {
            let pose = Mat4::from_translation(Vec3::new(0.5, 0.75 + 0.1, 0.5))
                * Mat4::from_rotation_z(80.0f32.to_radians());
            push_book(&mut elements, pose, [0.0, 0.1, 0.9])?;
        }
        "lectern"
            if block
                .properties
                .get("has_book")
                .is_some_and(|v| v == "true") =>
        {
            let facing = block
                .properties
                .get("facing")
                .map_or("north", String::as_str);
            let pose = Mat4::from_translation(Vec3::new(0.5, 1.0625, 0.5))
                * Mat4::from_rotation_y((-y_rot(clockwise(facing))).to_radians())
                * Mat4::from_rotation_z(67.5f32.to_radians())
                * Mat4::from_translation(Vec3::new(0.0, -0.125, 0.0));
            push_book(
                &mut elements,
                pose,
                [(0.0f32.sin() * 0.1 + 1.25) * 1.2, 0.1, 0.9],
            )?;
        }
        _ => {}
    }
    Ok(elements)
}

pub(crate) fn item_model(special: &Value) -> Result<Option<ResolvedModel>> {
    let field = |key: &str| special.get(key).and_then(Value::as_str);
    let mut elements = Vec::new();
    match field("type").unwrap_or("") {
        "minecraft:chest" => {
            let texture = sheet_id(field("texture").unwrap_or("normal"), "entity/chest")?;
            let chest_type = field("chest_type").unwrap_or("single");
            elements = crate::model::chest_model(texture, chest_type, "south").elements;
        }
        "minecraft:shulker_box" => {
            let texture = sheet_id(field("texture").unwrap_or("shulker"), "entity/shulker")?;
            let sheet = Sheet {
                texture,
                size: [64.0, 64.0],
                tinted: false,
            };
            push_shulker(&mut elements, sheet, Mat4::IDENTITY);
        }
        "minecraft:head" => {
            let Some(head) = field("kind").and_then(Head::of_kind) else {
                return Ok(None);
            };
            let texture = field("texture")
                .map(|texture| sheet_id(texture, "entity"))
                .transpose()?;
            push_head(&mut elements, head, texture, Mat4::IDENTITY)?;
        }
        "minecraft:player_head" => push_head(&mut elements, Head::Player, None, Mat4::IDENTITY)?,
        "minecraft:banner" => {
            let standing = field("attachment") != Some("wall");
            push_banner(&mut elements, standing, Mat4::IDENTITY)?;
        }
        "minecraft:conduit" => push_conduit(&mut elements, Mat4::IDENTITY)?,
        "minecraft:decorated_pot" => push_decorated_pot(&mut elements, Mat4::IDENTITY)?,
        "minecraft:shield" => push_shield(&mut elements)?,
        _ => return Ok(None),
    }
    Ok(Some(ResolvedModel { elements }))
}

pub(crate) fn special_tints(model: &Value) -> Vec<[u8; 3]> {
    item_leaves(model, false)
        .first()
        .and_then(|leaf| leaf.special)
        .filter(|special| special.get("type").and_then(Value::as_str) == Some("minecraft:banner"))
        .and_then(|special| special.get("color").and_then(Value::as_str))
        .and_then(dye_color)
        .map_or_else(Vec::new, |rgb| {
            vec![rgb.map(|channel| (channel * 255.0).round() as u8)]
        })
}

pub(crate) fn uses_item_geometry(model: &Value) -> bool {
    let leaves = item_leaves(model, false);
    leaves.len() > 1
        || leaves
            .iter()
            .any(|leaf| leaf.special.is_some() || leaf.transform != Mat4::IDENTITY)
}

pub(crate) fn item_leaves(model: &Value, using_item: bool) -> Vec<ItemLeaf<'_>> {
    let mut leaves = Vec::new();
    collect_leaves(model, using_item, Mat4::IDENTITY, &mut leaves);
    leaves
}

fn collect_leaves<'a>(
    value: &'a Value,
    using_item: bool,
    parent: Mat4,
    out: &mut Vec<ItemLeaf<'a>>,
) {
    let transform = parent * transformation(value.get("transformation"));
    let first_yielding = |out: &mut Vec<ItemLeaf<'a>>, options: Vec<&'a Value>| {
        for option in options {
            let mut found = Vec::new();
            collect_leaves(option, using_item, transform, &mut found);
            if !found.is_empty() {
                out.extend(found);
                return;
            }
        }
    };
    match value.get("type").and_then(Value::as_str).unwrap_or("") {
        "minecraft:model" => {
            if let Some(model) = value.get("model").and_then(Value::as_str) {
                out.push(ItemLeaf {
                    model,
                    special: None,
                    transform,
                });
            }
        }
        "minecraft:special" => {
            let base = value.get("base").and_then(Value::as_str);
            let special = value.get("model");
            let kind = special
                .and_then(|special| special.get("type"))
                .and_then(Value::as_str);
            if let (Some(model), Some(special), Some(kind)) = (base, special, kind) {
                if SUPPORTED_SPECIALS.contains(&kind) {
                    out.push(ItemLeaf {
                        model,
                        special: Some(special),
                        transform,
                    });
                }
            }
        }
        "minecraft:composite" => {
            for model in value
                .get("models")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                collect_leaves(model, using_item, transform, out);
            }
        }
        "minecraft:condition" => {
            let on_true = value.get("on_true");
            let on_false = value.get("on_false");
            let using = using_item
                && value.get("property").and_then(Value::as_str) == Some("minecraft:using_item");
            let order = if using {
                [on_true, on_false]
            } else {
                [on_false, on_true]
            };
            first_yielding(out, order.into_iter().flatten().collect());
        }
        "minecraft:select" | "minecraft:range_dispatch" => {
            let mut options: Vec<&Value> = value.get("fallback").into_iter().collect();
            for entry in value
                .get("cases")
                .or_else(|| value.get("entries"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                options.extend(entry.get("model"));
            }
            first_yielding(out, options);
        }
        _ => {}
    }
}

fn transformation(raw: Option<&Value>) -> Mat4 {
    let Some(raw) = raw else {
        return Mat4::IDENTITY;
    };
    let number = |value: &Value| value.as_f64().map(|v| v as f32);
    if let Some(values) = raw.as_array() {
        let values = values.iter().filter_map(number).collect::<Vec<_>>();
        return <[f32; 16]>::try_from(values.as_slice())
            .map_or(Mat4::IDENTITY, |m| Mat4::from_cols_array(&m).transpose());
    }
    let vector = |key: &str, default: Vec3| {
        raw.get(key)
            .and_then(Value::as_array)
            .map(|v| v.iter().filter_map(number).collect::<Vec<_>>())
            .filter(|v| v.len() == 3)
            .map_or(default, |v| Vec3::new(v[0], v[1], v[2]))
    };
    let rotation = |key: &str| {
        let Some(value) = raw.get(key) else {
            return Quat::IDENTITY;
        };
        if let Some(q) = value
            .as_array()
            .map(|v| v.iter().filter_map(number).collect::<Vec<_>>())
            .filter(|v| v.len() == 4)
        {
            let quat = Quat::from_xyzw(q[0], q[1], q[2], q[3]);
            return if quat.length_squared() > 0.0 {
                quat.normalize()
            } else {
                Quat::IDENTITY
            };
        }
        let axis = value
            .get("axis")
            .and_then(Value::as_array)
            .map(|v| v.iter().filter_map(number).collect::<Vec<_>>())
            .filter(|v| v.len() == 3)
            .map(|v| Vec3::new(v[0], v[1], v[2]));
        match (axis, value.get("angle").and_then(number)) {
            (Some(axis), Some(angle)) if axis.length_squared() > 0.0 => {
                Quat::from_axis_angle(axis.normalize(), angle)
            }
            _ => Quat::IDENTITY,
        }
    };
    Mat4::from_translation(vector("translation", Vec3::ZERO))
        * Mat4::from_quat(rotation("left_rotation"))
        * Mat4::from_scale(vector("scale", Vec3::ONE))
        * Mat4::from_quat(rotation("right_rotation"))
}

pub(crate) fn transformed(mut element: Element, transform: Mat4) -> Element {
    if transform == Mat4::IDENTITY {
        return element;
    }
    let outer = Mat3::from_mat4(transform);
    if outer == Mat3::IDENTITY && element.rotation.is_none() && element.rotation_y == 0 {
        let offset = transform.w_axis.truncate();
        element.from = (Vec3::from(element.from) + offset).to_array();
        element.to = (Vec3::from(element.to) + offset).to_array();
        return element;
    }
    let (linear, offset) = element_affine(&element);
    let linear = outer * linear;
    let offset = transform.transform_point3(offset);
    let shift = linear.inverse() * offset;
    element.from = (Vec3::from(element.from) + shift).to_array();
    element.to = (Vec3::from(element.to) + shift).to_array();
    element.rotation = Some(ElementRotation {
        origin: [0.0; 3],
        matrix: linear.to_cols_array_2d(),
    });
    element.rotation_y = 0;
    element
}

pub(crate) fn element_affine(element: &Element) -> (Mat3, Vec3) {
    let (rotation, rotation_offset) = element.rotation.map_or((Mat3::IDENTITY, Vec3::ZERO), |r| {
        let matrix = Mat3::from_cols_array_2d(&r.matrix);
        let origin = Vec3::from(r.origin);
        (matrix, origin - matrix * origin)
    });
    let (turn, turn_offset) = match element.rotation_y {
        90 => (
            Mat3::from_cols(Vec3::Z, Vec3::Y, Vec3::NEG_X),
            Vec3::new(1.0, 0.0, 0.0),
        ),
        180 => (
            Mat3::from_diagonal(Vec3::new(-1.0, 1.0, -1.0)),
            Vec3::new(1.0, 0.0, 1.0),
        ),
        270 => (
            Mat3::from_cols(Vec3::NEG_Z, Vec3::Y, Vec3::X),
            Vec3::new(0.0, 0.0, 1.0),
        ),
        _ => (Mat3::IDENTITY, Vec3::ZERO),
    };
    (turn * rotation, turn * rotation_offset + turn_offset)
}

fn sheet_id(raw: &str, directory: &str) -> Result<ResourceId> {
    let id = ResourceId::parse(raw)?;
    ResourceId::parse(&format!("{}:{directory}/{}", id.namespace, id.path))
}

fn segment_degrees(rotation: Option<&str>) -> f32 {
    let segment = rotation.and_then(|r| r.parse::<u8>().ok()).unwrap_or(0) % 16;
    let degrees = segment as f32 * 22.5;
    if degrees >= 180.0 {
        degrees - 360.0
    } else {
        degrees
    }
}

fn y_rot(facing: &str) -> f32 {
    match facing {
        "west" => 90.0,
        "north" => 180.0,
        "east" => 270.0,
        _ => 0.0,
    }
}

fn opposite(facing: &str) -> &'static str {
    match facing {
        "north" => "south",
        "south" => "north",
        "west" => "east",
        "east" => "west",
        "up" => "down",
        _ => "up",
    }
}

fn clockwise(facing: &str) -> &'static str {
    match facing {
        "north" => "east",
        "east" => "south",
        "south" => "west",
        _ => "north",
    }
}

fn horizontal_step(facing: &str) -> (f32, f32) {
    match facing {
        "north" => (0.0, -1.0),
        "south" => (0.0, 1.0),
        "west" => (-1.0, 0.0),
        "east" => (1.0, 0.0),
        _ => (0.0, 0.0),
    }
}

fn direction_rotation(facing: &str) -> Mat4 {
    let xyz = |x: f32, z: f32| Mat4::from_rotation_x(x) * Mat4::from_rotation_z(z);
    match facing {
        "down" => Mat4::from_rotation_x(PI),
        "north" => xyz(FRAC_PI_2, PI),
        "south" => Mat4::from_rotation_x(FRAC_PI_2),
        "west" => xyz(FRAC_PI_2, FRAC_PI_2),
        "east" => xyz(FRAC_PI_2, -FRAC_PI_2),
        _ => Mat4::IDENTITY,
    }
}

fn around_center(rotation: Mat4) -> Mat4 {
    Mat4::from_translation(Vec3::splat(0.5)) * rotation * Mat4::from_translation(Vec3::splat(-0.5))
}

fn part(offset: [f32; 3], rotation: [f32; 3]) -> Mat4 {
    Mat4::from_translation(Vec3::from(offset) / 16.0)
        * Mat4::from_rotation_z(rotation[2])
        * Mat4::from_rotation_y(rotation[1])
        * Mat4::from_rotation_x(rotation[0])
}

fn push_cube(elements: &mut Vec<Element>, sheet: &Sheet, cube: Cube, pose: Mat4, visible: &[&str]) {
    let [w, h, d] = cube.size;
    let [u, v] = cube.uv;
    let (u0, u1, u2) = (u, u + d, u + d + w);
    let (u22, u3, u4) = (u2 + w, u2 + d, u2 + d + w);
    let (v0, v1, v2) = (v, v + d, v + d + h);
    let low = Vec3::from(cube.origin) - Vec3::splat(cube.inflate);
    let extent = Vec3::from(cube.size) + Vec3::splat(2.0 * cube.inflate);
    let thickness = extent.cmpgt(Vec3::ZERO);
    let pad = Vec3::select(thickness, Vec3::ZERO, Vec3::splat(SHEET_HALF_THICKNESS));
    let linear = Mat3::from_mat4(pose);
    let shift = linear.inverse() * pose.w_axis.truncate();
    let from = (low - pad) / 16.0 + shift;
    let to = (low + extent + pad) / 16.0 + shift;
    let mut faces = Vec::new();
    for (direction, rect) in [
        ("down", [u1, v0, u2, v1]),
        ("up", [u2, v1, u22, v0]),
        ("west", [u0, v1, u1, v2]),
        ("north", [u1, v1, u2, v2]),
        ("east", [u2, v1, u3, v2]),
        ("south", [u3, v1, u4, v2]),
    ] {
        if !visible.contains(&direction) {
            continue;
        }
        let placed = match (cube.mirror, direction) {
            (true, "west") => "east",
            (true, "east") => "west",
            _ => direction,
        };
        let has_area = match placed {
            "down" | "up" => thickness.x && thickness.z,
            "north" | "south" => thickness.x && thickness.y,
            _ => thickness.y && thickness.z,
        };
        if !has_area {
            continue;
        }
        let [t0, t1, t2, t3] = rect;
        let [a, b, c, e] = match placed {
            "down" | "up" => [t0, t1, t2, t3],
            _ => [t2, t3, t0, t1],
        };
        let (a, c) = if cube.mirror { (c, a) } else { (a, c) };
        let (a, c) = inset(a, c);
        let (b, e) = inset(b, e);
        faces.push(Face {
            direction: placed.to_owned(),
            texture: sheet.texture.clone(),
            uv: [
                a / sheet.size[0],
                b / sheet.size[1],
                c / sheet.size[0],
                e / sheet.size[1],
            ],
            cull: false,
            cullface: None,
            tint: sheet.tinted,
            tint_index: sheet.tinted.then_some(0),
            force_translucent: false,
        });
    }
    if faces.is_empty() {
        return;
    }
    elements.push(Element {
        from: from.to_array(),
        to: to.to_array(),
        faces,
        rotation_y: 0,
        rotation: Some(ElementRotation {
            origin: [0.0; 3],
            matrix: linear.to_cols_array_2d(),
        }),
        shade_direction_override: None,
    });
}

fn inset(start: f32, end: f32) -> (f32, f32) {
    if start < end {
        (start + UV_INSET, end - UV_INSET)
    } else {
        (start - UV_INSET, end + UV_INSET)
    }
}

fn push_shulker(elements: &mut Vec<Element>, sheet: Sheet, pose: Mat4) {
    let body = pose * part([0.0, 24.0, 0.0], [0.0; 3]);
    let lid = Cube::new([0.0, 0.0], [-8.0, -16.0, -8.0], [16.0, 12.0, 16.0]);
    let base = Cube::new([0.0, 28.0], [-8.0, -8.0, -8.0], [16.0, 8.0, 16.0]);
    push_cube(elements, &sheet, lid, body, &ALL_FACES);
    push_cube(elements, &sheet, base, body, &ALL_FACES);
}

fn push_head(
    elements: &mut Vec<Element>,
    head: Head,
    texture: Option<ResourceId>,
    pose: Mat4,
) -> Result<()> {
    let texture = match texture {
        Some(texture) => texture,
        None => ResourceId::parse(head.texture())?,
    };
    let sheet = |size: [f32; 2]| Sheet {
        texture: texture.clone(),
        size,
        tinted: false,
    };
    match head {
        Head::Dragon => {
            let sheet = sheet([256.0, 256.0]);
            let head_pose = pose
                * Mat4::from_translation(Vec3::new(0.0, -7.986666 * 0.75, 0.0) / 16.0)
                * Mat4::from_scale(Vec3::splat(0.75));
            for cube in [
                Cube::new([176.0, 44.0], [-6.0, -1.0, -24.0], [12.0, 5.0, 16.0]),
                Cube::new([112.0, 30.0], [-8.0, -8.0, -10.0], [16.0, 16.0, 16.0]),
                Cube::new([0.0, 0.0], [-5.0, -12.0, -4.0], [2.0, 4.0, 6.0]).mirrored(),
                Cube::new([112.0, 0.0], [-5.0, -3.0, -22.0], [2.0, 2.0, 4.0]).mirrored(),
                Cube::new([0.0, 0.0], [3.0, -12.0, -4.0], [2.0, 4.0, 6.0]),
                Cube::new([112.0, 0.0], [3.0, -3.0, -22.0], [2.0, 2.0, 4.0]),
            ] {
                push_cube(elements, &sheet, cube, head_pose, &ALL_FACES);
            }
            let jaw = head_pose * part([0.0, 4.0, -8.0], [(0.0f32.sin() + 1.0) * 0.2, 0.0, 0.0]);
            let jaw_cube = Cube::new([176.0, 65.0], [-6.0, 0.0, -16.0], [12.0, 4.0, 16.0]);
            push_cube(elements, &sheet, jaw_cube, jaw, &ALL_FACES);
        }
        Head::Piglin => {
            let sheet = sheet([64.0, 64.0]);
            for cube in [
                Cube::new([0.0, 0.0], [-5.0, -8.0, -4.0], [10.0, 8.0, 8.0]),
                Cube::new([31.0, 1.0], [-2.0, -4.0, -5.0], [4.0, 4.0, 1.0]),
                Cube::new([2.0, 4.0], [2.0, -2.0, -5.0], [1.0, 2.0, 1.0]),
                Cube::new([2.0, 0.0], [-3.0, -2.0, -5.0], [1.0, 2.0, 1.0]),
            ] {
                push_cube(elements, &sheet, cube, pose, &ALL_FACES);
            }
            let ear = (0.0f32.cos() + 2.5) * 0.2;
            let left = Cube::new([51.0, 6.0], [0.0, 0.0, -2.0], [1.0, 5.0, 4.0]);
            let right = Cube::new([39.0, 6.0], [-1.0, 0.0, -2.0], [1.0, 5.0, 4.0]);
            let left_pose = pose * part([4.5, -6.0, 0.0], [0.0, 0.0, -ear]);
            let right_pose = pose * part([-4.5, -6.0, 0.0], [0.0, 0.0, ear]);
            push_cube(elements, &sheet, left, left_pose, &ALL_FACES);
            push_cube(elements, &sheet, right, right_pose, &ALL_FACES);
        }
        _ => {
            let humanoid = matches!(head, Head::Zombie | Head::Player);
            let sheet = sheet([64.0, if humanoid { 64.0 } else { 32.0 }]);
            let skull = Cube::new([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]);
            push_cube(elements, &sheet, skull, pose, &ALL_FACES);
            if humanoid {
                let hat = Cube::new([32.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]);
                push_cube(elements, &sheet, hat.inflated(0.25), pose, &ALL_FACES);
            }
        }
    }
    Ok(())
}

fn push_banner(elements: &mut Vec<Element>, standing: bool, pose: Mat4) -> Result<()> {
    let base = Sheet::new("minecraft:entity/banner/banner_base", [64.0, 64.0])?;
    let flag = Sheet::new("minecraft:entity/banner/base", [64.0, 64.0])?.tinted();
    let (bar_y, bar_z, flag_z) = if standing {
        (-44.0, -1.0, 0.0)
    } else {
        (-20.5, 9.5, 10.5)
    };
    if standing {
        let pole = Cube::new([44.0, 0.0], [-1.0, -42.0, -1.0], [2.0, 42.0, 2.0]);
        push_cube(elements, &base, pole, pose, &ALL_FACES);
    }
    let bar = Cube::new([0.0, 42.0], [-10.0, bar_y, bar_z], [20.0, 2.0, 2.0]);
    push_cube(elements, &base, bar, pose, &ALL_FACES);
    let flag_pose = pose * part([0.0, bar_y, flag_z], [BANNER_FLAG_ANGLE, 0.0, 0.0]);
    let cloth = Cube::new([0.0, 0.0], [-10.0, 0.0, -2.0], [20.0, 40.0, 1.0]);
    push_cube(elements, &flag, cloth, flag_pose, &ALL_FACES);
    Ok(())
}

fn push_conduit(elements: &mut Vec<Element>, pose: Mat4) -> Result<()> {
    let sheet = Sheet::new("minecraft:entity/conduit/base", [32.0, 16.0])?;
    let shell = Cube::new([0.0, 0.0], [-3.0, -3.0, -3.0], [6.0, 6.0, 6.0]);
    push_cube(elements, &sheet, shell, pose, &ALL_FACES);
    Ok(())
}

fn push_decorated_pot(elements: &mut Vec<Element>, pose: Mat4) -> Result<()> {
    let base = Sheet::new(
        "minecraft:entity/decorated_pot/decorated_pot_base",
        [32.0, 32.0],
    )?;
    let side = Sheet::new(
        "minecraft:entity/decorated_pot/decorated_pot_side",
        [16.0, 16.0],
    )?;
    let neck = pose * part([0.0, 37.0, 16.0], [PI, 0.0, 0.0]);
    let rim = Cube::new([0.0, 0.0], [4.0, 17.0, 4.0], [8.0, 3.0, 8.0]).inflated(-0.1);
    let lip = Cube::new([0.0, 5.0], [5.0, 20.0, 5.0], [6.0, 1.0, 6.0]).inflated(0.2);
    push_cube(elements, &base, rim, neck, &ALL_FACES);
    push_cube(elements, &base, lip, neck, &ALL_FACES);
    let cap = Cube::new([-14.0, 13.0], [0.0; 3], [14.0, 0.0, 14.0]);
    push_cube(
        elements,
        &base,
        cap,
        pose * part([1.0, 16.0, 1.0], [0.0; 3]),
        &ALL_FACES,
    );
    push_cube(
        elements,
        &base,
        cap,
        pose * part([1.0, 0.0, 1.0], [0.0; 3]),
        &ALL_FACES,
    );
    let wall = Cube::new([1.0, 0.0], [0.0; 3], [14.0, 16.0, 0.0]);
    for (offset, rotation) in [
        ([15.0, 16.0, 1.0], [0.0, 0.0, PI]),
        ([1.0, 16.0, 1.0], [0.0, -FRAC_PI_2, PI]),
        ([15.0, 16.0, 15.0], [0.0, FRAC_PI_2, PI]),
        ([1.0, 16.0, 15.0], [PI, 0.0, 0.0]),
    ] {
        push_cube(
            elements,
            &side,
            wall,
            pose * part(offset, rotation),
            &["north"],
        );
    }
    Ok(())
}

fn push_bell(elements: &mut Vec<Element>) -> Result<()> {
    let sheet = Sheet::new("minecraft:entity/bell/bell_body", [32.0, 32.0])?;
    let body = part([8.0, 12.0, 8.0], [0.0; 3]);
    let bell = Cube::new([0.0, 0.0], [-3.0, -6.0, -3.0], [6.0, 7.0, 6.0]);
    push_cube(elements, &sheet, bell, body, &ALL_FACES);
    let lip = Cube::new([0.0, 13.0], [4.0, 4.0, 4.0], [8.0, 2.0, 8.0]);
    push_cube(
        elements,
        &sheet,
        lip,
        body * part([-8.0, -12.0, -8.0], [0.0; 3]),
        &ALL_FACES,
    );
    Ok(())
}

fn push_book(
    elements: &mut Vec<Element>,
    pose: Mat4,
    [open, flip1, flip2]: [f32; 3],
) -> Result<()> {
    let sheet = Sheet::new(
        "minecraft:entity/enchantment/enchanting_table_book",
        [64.0, 32.0],
    )?;
    let pages = open.sin();
    let parts = [
        (
            Cube::new([0.0, 0.0], [-6.0, -5.0, -0.005], [6.0, 10.0, 0.005]),
            [0.0, 0.0, -1.0],
            PI + open,
        ),
        (
            Cube::new([16.0, 0.0], [0.0, -5.0, -0.005], [6.0, 10.0, 0.005]),
            [0.0, 0.0, 1.0],
            -open,
        ),
        (
            Cube::new([12.0, 0.0], [-1.0, -5.0, 0.0], [2.0, 10.0, 0.005]),
            [0.0; 3],
            FRAC_PI_2,
        ),
        (
            Cube::new([0.0, 10.0], [0.0, -4.0, -0.99], [5.0, 8.0, 1.0]),
            [pages, 0.0, 0.0],
            open,
        ),
        (
            Cube::new([12.0, 10.0], [0.0, -4.0, -0.01], [5.0, 8.0, 1.0]),
            [pages, 0.0, 0.0],
            -open,
        ),
        (
            Cube::new([24.0, 10.0], [0.0, -4.0, 0.0], [5.0, 8.0, 0.005]),
            [pages, 0.0, 0.0],
            open - open * 2.0 * flip1,
        ),
        (
            Cube::new([24.0, 10.0], [0.0, -4.0, 0.0], [5.0, 8.0, 0.005]),
            [pages, 0.0, 0.0],
            open - open * 2.0 * flip2,
        ),
    ];
    for (cube, offset, y_rotation) in parts {
        push_cube(
            elements,
            &sheet,
            cube,
            pose * part(offset, [0.0, y_rotation, 0.0]),
            &ALL_FACES,
        );
    }
    Ok(())
}

fn push_shield(elements: &mut Vec<Element>) -> Result<()> {
    let sheet = Sheet::new(
        "minecraft:entity/shield/shield_base_nopattern",
        [64.0, 64.0],
    )?;
    let plate = Cube::new([0.0, 0.0], [-6.0, -11.0, -2.0], [12.0, 22.0, 1.0]);
    let handle = Cube::new([26.0, 0.0], [-1.0, -3.0, -1.0], [2.0, 6.0, 6.0]);
    push_cube(elements, &sheet, plate, Mat4::IDENTITY, &ALL_FACES);
    push_cube(elements, &sheet, handle, Mat4::IDENTITY, &ALL_FACES);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{TintKind, element_corners, quad_normal};
    use crate::pack::PackStack;
    use image::{Rgba, RgbaImage};
    use std::fs;

    fn corners(model: &ResolvedModel) -> Vec<Vec3> {
        model.elements.iter().flat_map(element_points).collect()
    }

    fn element_points(element: &Element) -> Vec<Vec3> {
        element
            .faces
            .iter()
            .flat_map(|face| element_corners(element, &face.direction).unwrap())
            .map(Vec3::from_array)
            .collect()
    }

    fn bounds(points: &[Vec3]) -> (Vec3, Vec3) {
        points.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(low, high), point| (low.min(*point), high.max(*point)),
        )
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).abs().max_element() < 1e-4
    }

    fn uv_at(face: &Face, corner: usize) -> [f32; 2] {
        let [a, b, c, d] = face.uv;
        [[a, b], [a, d], [c, d], [c, b]][corner]
    }

    #[test]
    fn upward_shulker_box_base_and_lid_fill_the_block() {
        let block = Block::new("minecraft:red_shulker_box").with("facing", "up");
        let model = block_model(&block).unwrap().unwrap();
        assert_eq!(model.elements.len(), 2);
        let (lid_low, lid_high) = bounds(&element_points(&model.elements[0]));
        let (base_low, base_high) = bounds(&element_points(&model.elements[1]));
        assert!((base_low.y - 0.00025).abs() < 1e-4 && (base_high.y - 0.5).abs() < 1e-3);
        assert!((lid_low.y - 0.25).abs() < 1e-3 && (lid_high.y - 0.99975).abs() < 1e-4);
        assert!(
            model.elements[0].faces[0]
                .texture
                .path
                .ends_with("shulker/shulker_red")
        );
        let down = Block::new("minecraft:shulker_box").with("facing", "down");
        let down = block_model(&down).unwrap().unwrap();
        let (base_low, _) = bounds(&element_points(&down.elements[1]));
        assert!(
            base_low.y > 0.49,
            "a downward box keeps its base at the top"
        );
    }

    #[test]
    fn wall_skull_sits_against_its_wall_and_rotation_eight_is_the_item() {
        let wall = Block::new("minecraft:skeleton_wall_skull").with("facing", "north");
        let wall = block_model(&wall).unwrap().unwrap();
        let (low, high) = bounds(&corners(&wall));
        assert!(close(low, Vec3::new(0.25, 0.25, 0.5)) && close(high, Vec3::new(0.75, 0.75, 1.0)));
        let standing = Block::new("minecraft:zombie_head").with("rotation", "8");
        let standing = block_model(&standing).unwrap().unwrap();
        assert_eq!(standing.elements.len(), 2, "head and hat");
        let special = serde_json::json!({"type": "minecraft:head", "kind": "zombie"});
        let item = item_model(&special).unwrap().unwrap();
        let transform = transformation(Some(&serde_json::json!({
            "translation": [0.5, 0.0, 0.5],
            "left_rotation": [1.0, 0.0, 0.0, 0.0]
        })));
        let item = ResolvedModel {
            elements: item
                .elements
                .into_iter()
                .map(|element| transformed(element, transform))
                .collect(),
        };
        let (item_corners, block_corners) = (corners(&item), corners(&standing));
        assert_eq!(item_corners.len(), block_corners.len());
        assert!(
            item_corners
                .iter()
                .zip(&block_corners)
                .all(|(a, b)| close(*a, *b))
        );
    }

    #[test]
    fn chest_front_faces_the_way_the_chest_does() {
        let texture = ResourceId::parse("minecraft:entity/chest/normal").unwrap();
        for (facing, normal) in [
            ("east", Vec3::X),
            ("north", Vec3::NEG_Z),
            ("south", Vec3::Z),
            ("west", Vec3::NEG_X),
        ] {
            let model = crate::model::chest_model(texture.clone(), "single", facing);
            let front = element_corners(&model.elements[2], "south").unwrap();
            assert!(close(quad_normal(&front).unwrap(), normal), "{facing}");
        }
        let copper = chest_texture("waxed_weathered_copper_chest", "left").unwrap();
        assert_eq!(copper.path, "entity/chest/copper_weathered_left");
        let ender = chest_texture("ender_chest", "single").unwrap();
        assert_eq!(ender.path, "entity/chest/ender");
        let trapped = Block::new("minecraft:trapped_chest").with("type", "right");
        assert!(is_chest(&trapped) && !is_chest(&Block::new("minecraft:barrel")));
    }

    #[test]
    fn model_part_faces_map_the_skin_like_the_chest_model() {
        let texture = ResourceId::parse("minecraft:entity/chest/normal").unwrap();
        let chest = crate::model::chest_model(texture.clone(), "single", "south");
        let sheet = Sheet {
            texture,
            size: [64.0, 64.0],
            tinted: false,
        };
        let mut parts = Vec::new();
        let lid = Cube::new([0.0, 0.0], [1.0, 0.0, 0.0], [14.0, 5.0, 14.0]);
        let pose = part([0.0, 9.0, 1.0], [0.0; 3]);
        push_cube(&mut parts, &sheet, lid, pose, &ALL_FACES);
        let reference = &chest.elements[1];
        assert_eq!(parts[0].faces.len(), 6);
        for face in &parts[0].faces {
            let ours = element_corners(&parts[0], &face.direction).unwrap();
            let theirs = element_corners(reference, &face.direction).unwrap();
            let theirs_face = reference
                .faces
                .iter()
                .find(|other| other.direction == face.direction)
                .unwrap();
            for (corner, position) in ours.iter().enumerate() {
                let position = Vec3::from_array(*position);
                let matching = theirs
                    .iter()
                    .position(|other| close(Vec3::from_array(*other), position))
                    .unwrap();
                let [u, v] = uv_at(face, corner);
                let [their_u, their_v] = uv_at(theirs_face, matching);
                assert!(
                    (u - their_u).abs() <= 0.25 / 64.0 + 1e-5
                        && (v - their_v).abs() <= UV_INSET / 64.0 + 1e-5,
                    "{} corner {corner}",
                    face.direction
                );
            }
        }
    }

    #[test]
    fn mirrored_cube_flips_its_skin_and_swaps_its_sides() {
        let sheet = Sheet::new("minecraft:entity/enderdragon/dragon", [256.0, 256.0]).unwrap();
        let cube = Cube::new([0.0, 0.0], [0.0; 3], [2.0, 4.0, 6.0]);
        let mut plain = Vec::new();
        let mut mirrored = Vec::new();
        push_cube(&mut plain, &sheet, cube, Mat4::IDENTITY, &ALL_FACES);
        push_cube(
            &mut mirrored,
            &sheet,
            cube.mirrored(),
            Mat4::IDENTITY,
            &ALL_FACES,
        );
        let uv = |elements: &[Element], direction: &str| {
            elements[0]
                .faces
                .iter()
                .find(|face| face.direction == direction)
                .unwrap()
                .uv
        };
        let west = uv(&plain, "west");
        assert_eq!([west[2], west[1], west[0], west[3]], uv(&mirrored, "east"));
        let up = uv(&plain, "up");
        assert_eq!([up[2], up[1], up[0], up[3]], uv(&mirrored, "up"));
    }

    #[test]
    fn banners_stand_two_blocks_tall_with_a_dyed_flag() {
        let block = Block::new("minecraft:red_banner").with("rotation", "0");
        let model = block_model(&block).unwrap().unwrap();
        let (pole_low, pole_high) = bounds(&element_points(&model.elements[0]));
        assert!(pole_low.y.abs() < 1e-4 && (pole_high.y - 1.75).abs() < 1e-4);
        let flag = model.elements.last().unwrap();
        assert!(
            flag.faces
                .iter()
                .all(|face| face.tint && face.texture.path == "entity/banner/base")
        );
        let (flag_low, flag_high) = bounds(&element_points(flag));
        assert!(
            flag_low.y > 0.16 && flag_high.y < 1.84,
            "{flag_low} {flag_high}"
        );
        let red = dye_color("red").unwrap();
        assert!(matches!(TintKind::of(&block), TintKind::Dye(color) if color == red));
        let wall = Block::new("minecraft:red_wall_banner").with("facing", "south");
        let wall_model = block_model(&wall).unwrap().unwrap();
        assert_eq!(wall_model.elements.len(), 2, "no pole on a wall");
        let (low, high) = bounds(&corners(&wall_model));
        assert!(low.z > -0.05 && high.z < 0.2, "{low} {high}");
        let item = serde_json::json!({
            "type": "minecraft:special",
            "base": "minecraft:item/template_banner",
            "model": {"type": "minecraft:banner", "color": "red"}
        });
        assert_eq!(special_tints(&item), vec![[176, 46, 38]]);
    }

    #[test]
    fn decorated_pot_sides_face_out_and_the_neck_tops_it() {
        let block = Block::new("minecraft:decorated_pot").with("facing", "east");
        let model = block_model(&block).unwrap().unwrap();
        let center = Vec3::splat(0.5);
        let sides = model
            .elements
            .iter()
            .filter(|element| {
                element.faces[0]
                    .texture
                    .path
                    .ends_with("decorated_pot_side")
            })
            .collect::<Vec<_>>();
        assert_eq!(sides.len(), 4);
        for side in sides {
            let quad = element_corners(side, &side.faces[0].direction).unwrap();
            let middle = quad.iter().map(|c| Vec3::from_array(*c)).sum::<Vec3>() / 4.0;
            assert!(quad_normal(&quad).unwrap().dot(middle - center) > 0.0);
        }
        let (_, high) = bounds(&corners(&model));
        assert!((high.y - 19.9 / 16.0).abs() < 0.01, "{high}");
    }

    #[test]
    fn bell_enchanting_table_and_lectern_add_their_entity_parts() {
        let points = |elements: &[Element]| -> Vec<Vec3> {
            elements.iter().flat_map(element_points).collect()
        };
        let bell = extra_elements(&Block::new("minecraft:bell")).unwrap();
        let (low, high) = bounds(&points(&bell));
        let top = Vec3::new(0.75, 13.0 / 16.0, 0.75);
        assert!(
            close(low, Vec3::splat(0.25)) && close(high, top),
            "{low} {high}"
        );
        let table = extra_elements(&Block::new("minecraft:enchanting_table")).unwrap();
        assert_eq!(table.len(), 7);
        let (low, _) = bounds(&points(&table));
        assert!(low.y > 0.75, "the book floats over the table");
        let empty = Block::new("minecraft:lectern").with("has_book", "false");
        assert!(extra_elements(&empty).unwrap().is_empty());
        let full = Block::new("minecraft:lectern").with("has_book", "true");
        assert_eq!(extra_elements(&full).unwrap().len(), 7);
    }

    #[test]
    fn item_leaves_compose_transformations_and_pick_the_using_model() {
        let bed = serde_json::json!({"type": "minecraft:composite", "models": [
            {"type": "minecraft:model", "model": "minecraft:block/red_bed_head"},
            {
                "type": "minecraft:model",
                "model": "minecraft:block/red_bed_foot",
                "transformation": {"translation": [0.0, 0.0, 1.0]}
            }
        ]});
        let leaves = item_leaves(&bed, false);
        assert_eq!(leaves.len(), 2);
        assert!(uses_item_geometry(&bed));
        let element = Element {
            from: [0.0; 3],
            to: [1.0, 0.5, 1.0],
            faces: Vec::new(),
            rotation_y: 0,
            rotation: None,
            shade_direction_override: None,
        };
        let moved = transformed(element.clone(), leaves[1].transform);
        assert_eq!((moved.from, moved.to), ([0.0, 0.0, 1.0], [1.0, 0.5, 2.0]));
        let turned = Element {
            rotation_y: 90,
            ..element
        };
        let turned = transformed(turned, Mat4::from_rotation_y(PI));
        assert!(turned.rotation.is_some() && turned.rotation_y == 0);
        let shield = serde_json::json!({
            "type": "minecraft:condition",
            "property": "minecraft:using_item",
            "on_false": {"type": "minecraft:special", "base": "minecraft:item/shield", "model": {"type": "minecraft:shield"}},
            "on_true": {"type": "minecraft:special", "base": "minecraft:item/shield_blocking", "model": {"type": "minecraft:shield"}},
            "transformation": {"scale": [1.0, -1.0, -1.0]}
        });
        assert_eq!(
            item_leaves(&shield, false)[0].model,
            "minecraft:item/shield"
        );
        assert_eq!(
            item_leaves(&shield, true)[0].model,
            "minecraft:item/shield_blocking"
        );
        let flip = Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0));
        assert_eq!(item_leaves(&shield, false)[0].transform, flip);
        let trident = serde_json::json!({
            "type": "minecraft:select",
            "property": "minecraft:display_context",
            "cases": [{"when": "gui", "model": {"type": "minecraft:model", "model": "minecraft:item/trident"}}],
            "fallback": {"type": "minecraft:special", "base": "minecraft:item/trident_in_hand", "model": {"type": "minecraft:trident"}}
        });
        assert_eq!(
            item_leaves(&trident, false)[0].model,
            "minecraft:item/trident"
        );
        assert!(!uses_item_geometry(&trident));
        let rotation = serde_json::json!({"left_rotation": [1.0, 0.0, 0.0, 0.0]});
        let flipped = transformation(Some(&rotation));
        assert!(close(flipped.transform_vector3(Vec3::Y), Vec3::NEG_Y));
    }

    #[test]
    fn special_items_get_3d_icons_and_item_geometry() {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        for (name, contents) in [
            (
                "assets/test/items/box.json",
                r#"{"model":{"type":"minecraft:special","base":"test:item/box","model":{"type":"minecraft:chest","texture":"test:box"}}}"#,
            ),
            (
                "assets/test/models/item/box.json",
                r#"{"display":{"gui":{"rotation":[30,45,0],"translation":[0,0,0],"scale":[0.625,0.625,0.625]},"ground":{"translation":[0,3,0],"scale":[0.25,0.25,0.25]}}}"#,
            ),
        ] {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        let sheet = root.join("assets/test/textures/entity/chest/box.png");
        fs::create_dir_all(sheet.parent().unwrap()).unwrap();
        RgbaImage::from_pixel(64, 64, Rgba([200, 160, 100, 255]))
            .save(sheet)
            .unwrap();
        let packs = PackStack::open(vec![root.into()]).unwrap();
        let id = ResourceId::parse("test:box").unwrap();
        let model = crate::model::resolve_item_model(&packs, &id)
            .unwrap()
            .unwrap();
        assert_eq!(model.elements.len(), 3);
        assert_eq!(
            model.elements[0].faces[0].texture.key(),
            "test:entity/chest/box"
        );
        let icon = crate::item_icons::item_icon(&packs, "test:box", 32)
            .unwrap()
            .unwrap();
        let shades = icon
            .pixels()
            .filter(|pixel| pixel[3] > 0)
            .map(|pixel| pixel[0])
            .collect::<std::collections::BTreeSet<_>>();
        assert!(shades.len() >= 3, "three lit sides: {shades:?}");
        let visible = icon.pixels().filter(|pixel| pixel[3] > 0).count();
        assert!(visible > 32 * 32 / 4 && visible < 32 * 32);
        assert!(icon.get_pixel(16, 16)[3] > 0);
        let texture = ResourceId::parse("test:entity/chest/box").unwrap();
        let atlas = crate::mesh::make_atlas(&packs, vec![texture]).unwrap();
        let mut items = minecraftoss_player::items::WorldItems::default();
        let stack = minecraftoss_player::inventory::ItemStack::new("test:box", 1);
        items.spawn(stack, glam::DVec3::new(0.5, 1.0, 0.5));
        let scene = crate::scene::HandcraftedScene::new();
        let light = crate::lighting::SkyLight::build(&scene);
        let mesh = crate::mesh::ItemVisuals::default()
            .mesh(&items, &packs, &atlas, &light, 1.0)
            .unwrap();
        assert_eq!(mesh.faces, 18, "a dropped chest draws all three boxes");
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| vertex.color[3] == ENTITY_SHEET_ALPHA)
        );
        let lowest = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        assert!(
            lowest > 1.0 && lowest < 1.3,
            "it bobs above the ground: {lowest}"
        );
    }
}
