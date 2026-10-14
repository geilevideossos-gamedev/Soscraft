//! The held item on the Minecraft map when no gun is selected, placed as
//! MinecraftOSS's viewer places it (26.3 `applyItemArmTransform` with the
//! item's `firstperson_righthand` display), swinging as vanilla swings it.
//! Vertices are in view space: x right, y up, z back.
use glam::{Mat4, Vec3};

const TOTEM: &str = "minecraft:totem_of_undying";
const FULL_BRIGHT: f32 = 15.0;

pub(crate) fn append_totem(
    mesh: &mut minecraft_terrain::mesh::ChunkMesh,
    view: &mut minecraft_survival::entity_view::EntityView,
    pose: Mat4,
    light_at: Vec3,
    (packs, atlas, light): (&minecraft_terrain::pack::PackStack, &minecraft_terrain::mesh::Atlas, &minecraft_terrain::lighting::SkyLight),
) {
    let display = minecraft_terrain::pack::ResourceId::parse(TOTEM)
        .ok()
        .and_then(|id| minecraft_terrain::model::item_display_transform(packs, &id, "fixed").ok())
        .unwrap_or(Mat4::IDENTITY);
    let start = mesh.vertices.len() as u32;
    let totem = view.held_item_mesh(&[(pose * display, TOTEM.to_owned(), false)], light_at, packs, atlas, light);
    mesh.vertices.extend(totem.vertices.into_iter().map(|vertex| minecraft_terrain::mesh::Vertex { sky_light: FULL_BRIGHT, block_light: FULL_BRIGHT, ..vertex }));
    mesh.indices.extend(totem.indices.into_iter().map(|index| index + start));
    mesh.faces += totem.faces;
}

pub(crate) fn hand_clip(aspect: f32) -> [f32; 16] {
    let f = 1.0 / (35.0f32.to_radians()).tan();
    let near = 0.05;
    Mat4::from_cols(
        glam::Vec4::new(f / aspect, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, f, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, 0.0, -1.0),
        glam::Vec4::new(0.0, 0.0, near, 0.0),
    )
    .to_cols_array()
}

/// Ticks of a swing (`LivingEntity.getCurrentSwingDuration`).
pub(crate) const SWING_TICKS: f32 = minecraft_survival::actions::SWING_TICKS;

/// The item's view-space pose (`applyItemArmTransform` then its display).
pub(crate) fn item_pose(display: Mat4, swing: f32, inverse_height: f32, arm: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(arm * 0.56, -0.52 - 0.6 * inverse_height, -0.72)) * item_swing_transform(swing, arm) * display
}

fn item_swing_transform(swing: f32, arm: f32) -> Mat4 {
    let root = swing.sqrt();
    let x = -0.4 * (root * std::f32::consts::PI).sin();
    let y = 0.2 * (root * std::f32::consts::TAU).sin();
    let z = -0.2 * (swing * std::f32::consts::PI).sin();
    let y_rotation = (swing * swing * std::f32::consts::PI).sin();
    let xz_rotation = (root * std::f32::consts::PI).sin();
    Mat4::from_translation(Vec3::new(arm * x, y, z))
        * Mat4::from_rotation_y((arm * (45.0 - 20.0 * y_rotation)).to_radians())
        * Mat4::from_rotation_z((arm * -20.0 * xz_rotation).to_radians())
        * Mat4::from_rotation_x((-80.0 * xz_rotation).to_radians())
        * Mat4::from_rotation_y((arm * -45.0f32).to_radians())
}

pub(crate) fn eat_transform(remaining: f32, partial: f32, duration: u32, arm: f32) -> Mat4 {
    let duration = duration.max(1) as f32;
    let left = (remaining - partial + 1.0).min(duration);
    let progress = left / duration;
    let bob = if progress < 0.8 { ((left / 4.0 * std::f32::consts::PI).cos() * 0.1).abs() } else { 0.0 };
    let lift = 1.0 - f64::from(progress).powf(27.0) as f32;
    Mat4::from_translation(Vec3::new(lift * 0.6 * arm, bob + lift * -0.5, 0.0))
        * Mat4::from_rotation_y((arm * lift * 90.0).to_radians())
        * Mat4::from_rotation_x((lift * 10.0).to_radians())
        * Mat4::from_rotation_z((arm * lift * 30.0).to_radians())
}

pub(crate) fn trident_charge_transform(used: f32, arm: f32) -> Mat4 {
    let held = used - 1.0;
    let charge = (held / 10.0).min(1.0);
    let shake = if charge > 0.1 { ((held - 0.1) * 1.3).sin() * (charge - 0.1) } else { 0.0 };
    Mat4::from_translation(Vec3::new(arm * -0.5, 0.7, 0.1))
        * Mat4::from_rotation_x((-55.0f32).to_radians())
        * Mat4::from_rotation_y((arm * 35.3).to_radians())
        * Mat4::from_rotation_z((arm * -9.785).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, charge * 0.2))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + charge * 0.2))
        * Mat4::from_rotation_y((arm * -45.0).to_radians())
}

pub(crate) const ACTIVATION_TICKS: u32 = 40;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ItemActivation {
    pub(crate) ticks: u32,
    pub(crate) off: [f32; 2],
}

impl ItemActivation {
    pub(crate) fn activate(&mut self, mut random: impl FnMut() -> f32) {
        self.ticks = ACTIVATION_TICKS;
        let x = random() * 2.0 - 1.0;
        let y = random() * 2.0 - 1.0;
        self.off = [x, y];
    }

    pub(crate) fn tick(&mut self, ticks: u32) {
        self.ticks = self.ticks.saturating_sub(ticks);
    }

    pub(crate) fn pose(&self, partial: f32, aspect: f32) -> Option<Mat4> {
        activation_pose(self.ticks, partial, self.off, aspect)
    }
}

pub(crate) fn activation_pose(ticks: u32, partial: f32, off: [f32; 2], aspect: f32) -> Option<Mat4> {
    if ticks == 0 {
        return None;
    }
    let t = ((ACTIVATION_TICKS - ticks.min(ACTIVATION_TICKS)) as f32 + partial) / ACTIVATION_TICKS as f32;
    let t2 = t * t;
    let t3 = t * t2;
    let curve = 10.25 * t3 * t2 - 24.95 * t2 * t2 + 25.5 * t3 - 13.8 * t2 + 4.0 * t;
    let angle = curve * std::f32::consts::PI;
    let sway = (angle * 2.0).sin().abs();
    let tilt = (6.0 * (t * 8.0).cos()).to_radians();
    Some(
        Mat4::from_translation(Vec3::new(off[0] * 0.3 * aspect * sway, off[1] * 0.3 * sway, -10.0 + 9.0 * angle.sin()))
            * Mat4::from_scale(Vec3::splat(0.8))
            * Mat4::from_rotation_y((900.0 * angle.sin().abs()).to_radians())
            * Mat4::from_rotation_x(tilt)
            * Mat4::from_rotation_z(tilt),
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum UseShown {
    Eat { remaining: u32, duration: u32 },
    Block,
    Charge { ticks: u32 },
    Scope,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HeldUse {
    pub(crate) slot: usize,
    pub(crate) kind: UseShown,
}

pub(crate) fn host_use(survival: &minecraft_survival::survival::Survival, hand: &minecraft_survival::player::Hand) -> Option<HeldUse> {
    survival
        .eating()
        .map(|eating| HeldUse { slot: eating.slot, kind: UseShown::Eat { remaining: eating.remaining_ticks(), duration: eating.info.consume_ticks } })
        .or_else(|| hand.blocking.map(|blocking| HeldUse { slot: blocking.slot, kind: UseShown::Block }))
        .or_else(|| hand.charging.map(|charging| HeldUse { slot: charging.slot, kind: UseShown::Charge { ticks: charging.ticks } }))
        .or_else(|| hand.scoping.map(|scoping| HeldUse { slot: scoping.slot, kind: UseShown::Scope }))
}

pub(crate) struct Held<'a> {
    pub(crate) main: Option<(usize, &'a minecraftoss_player::inventory::ItemStack)>,
    pub(crate) offhand: Option<&'a minecraftoss_player::inventory::ItemStack>,
    pub(crate) swing: f32,
    pub(crate) swing_offhand: bool,
    pub(crate) using: Option<HeldUse>,
    pub(crate) partial: f32,
    pub(crate) activation: Option<Mat4>,
}

pub(crate) type HandLook<'a> = (&'a minecraft_terrain::pack::PackStack, &'a minecraft_terrain::mesh::Atlas, &'a minecraft_terrain::lighting::SkyLight, &'a minecraft_terrain::dials::Dials);

pub(crate) fn held_hand_mesh(held: &Held<'_>, view: &mut minecraft_survival::entity_view::EntityView, (packs, atlas, light, dials): HandLook<'_>, light_at: Vec3, aspect: f32) -> Option<(Vec<u8>, Vec<u32>, [f32; 16])> {
    let offhand = minecraft_survival::offhand::OFFHAND;
    let stacks = posed_stacks(held);
    if held.activation.is_none() && stacks.iter().all(|(stack, _)| stack.is_none()) {
        return None;
    }
    let mut posed = Vec::new();
    for (stack, arm) in stacks {
        let Some((slot, stack)) = stack else {
            continue;
        };
        let using = held.using.filter(|using| using.slot == slot).map(|using| using.kind);
        let raised = matches!(using, Some(UseShown::Block | UseShown::Charge { .. }));
        let display = minecraft_terrain::pack::ResourceId::parse(&stack.id)
            .ok()
            .and_then(|id| minecraft_terrain::model::item_hand_transform(packs, &id, arm < 0.0, raised).ok())
            .unwrap_or(Mat4::IDENTITY);
        let display = match using {
            Some(UseShown::Charge { ticks }) => trident_charge_transform(ticks as f32 + held.partial, arm) * display,
            _ => display,
        };
        let eat = match using {
            Some(UseShown::Eat { remaining, duration }) => Some(eat_transform(remaining as f32, held.partial, duration, arm)),
            _ => None,
        };
        let swung = if raised || eat.is_some() || held.swing_offhand != (slot == offhand) { 0.0 } else { held.swing };
        let glint = minecraft_terrain::glint::has_glint(stack);
        let key = minecraft_terrain::item_tint::key(dials.key(&stack.id), minecraft_terrain::potions::stack_color(stack.components.as_ref()));
        posed.push((eat.unwrap_or(Mat4::IDENTITY) * item_pose(display, swung, 0.0, arm), key, glint));
    }
    let mut mesh = view.held_item_mesh(&posed, light_at, packs, atlas, light);
    if let Some(pose) = held.activation {
        append_totem(&mut mesh, view, pose, light_at, (packs, atlas, light));
    }
    let vertices = minecraft_terrain::mesh::section_vertices(&mesh);
    Some((bytemuck::cast_slice(&vertices).to_vec(), mesh.indices, hand_clip(aspect)))
}

type PosedStack<'a> = (Option<(usize, &'a minecraftoss_player::inventory::ItemStack)>, f32);

fn posed_stacks<'a>(held: &Held<'a>) -> [PosedStack<'a>; 2] {
    let scoping = held.using.is_some_and(|using| using.kind == UseShown::Scope);
    let offhand = held.offhand.map(|stack| (minecraft_survival::offhand::OFFHAND, stack));
    [(held.main, 1.0), (offhand, -1.0)].map(|(stack, arm)| (stack.filter(|_| !scoping), arm))
}

pub(crate) fn swing_shown(swing: Option<f32>) -> f32 {
    swing.map_or(0.0, |t| (t / SWING_TICKS).clamp(0.0, 1.0))
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct UseClock {
    seen: Option<minecraft_net::messages::UsingView>,
    at: u64,
}

impl UseClock {
    pub(crate) fn follow(&mut self, using: Option<minecraft_net::messages::UsingView>, tick: u64, selected: usize) -> Option<HeldUse> {
        if self.seen != using || tick < self.at {
            self.seen = using;
            self.at = tick;
        }
        let using = using?;
        let passed = u32::try_from(tick - self.at).unwrap_or(u32::MAX);
        let ticks = u32::try_from(using.ticks).unwrap_or(0);
        let slot = match using.hand {
            minecraft_net::messages::Hand::Main => selected,
            minecraft_net::messages::Hand::Off => minecraft_survival::offhand::OFFHAND,
        };
        let kind = match using.kind {
            minecraft_net::messages::UseKind::Eat { duration } => UseShown::Eat { remaining: ticks.saturating_sub(passed), duration: u32::try_from(duration).unwrap_or(0).max(1) },
            minecraft_net::messages::UseKind::Block => UseShown::Block,
            minecraft_net::messages::UseKind::Charge => UseShown::Charge { ticks: ticks.saturating_add(passed) },
            minecraft_net::messages::UseKind::Scope => UseShown::Scope,
        };
        Some(HeldUse { slot, kind })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_activation_rises_spins_and_returns_like_vanilla() {
        let off = [0.7, -0.4];
        assert!(activation_pose(0, 0.5, off, 1.5).is_none());
        let at = |ticks: u32, partial: f32| activation_pose(ticks, partial, off, 1.5).unwrap();
        let centre = |pose: Mat4| pose.transform_point3(Vec3::ZERO);
        let start = at(40, 0.0);
        assert!((centre(start) - Vec3::new(0.0, 0.0, -10.0)).length() < 1e-4, "{}", centre(start));
        let tilt = 6.0f32.to_radians();
        let expected = Mat4::from_translation(Vec3::new(0.0, 0.0, -10.0)) * Mat4::from_scale(Vec3::splat(0.8)) * Mat4::from_rotation_x(tilt) * Mat4::from_rotation_z(tilt);
        assert!(start.abs_diff_eq(expected, 1e-4));
        let end = at(1, 1.0);
        assert!((centre(end) - Vec3::new(0.0, 0.0, -10.0)).length() < 1e-3, "{}", centre(end));
        for (ticks, partial, t) in [(30, 0.0, 0.25f32), (20, 0.0, 0.5), (10, 0.0, 0.75), (21, 0.5, 0.4875)] {
            let curve = 10.25 * t.powi(5) - 24.95 * t.powi(4) + 25.5 * t.powi(3) - 13.8 * t * t + 4.0 * t;
            let a = curve * std::f32::consts::PI;
            let sway = (2.0 * a).sin().abs();
            let point = centre(at(ticks, partial));
            let wanted = Vec3::new(0.7 * 0.3 * 1.5 * sway, -0.4 * 0.3 * sway, -10.0 + 9.0 * a.sin());
            assert!((point - wanted).length() < 1e-4, "t {t}: {point} vs {wanted}");
            let spun = at(ticks, partial) * Mat4::from_rotation_z(-(6.0 * (8.0 * t).cos()).to_radians()) * Mat4::from_rotation_x(-(6.0 * (8.0 * t).cos()).to_radians());
            let facing = spun.transform_vector3(Vec3::X) / 0.8;
            let spin = (900.0 * a.sin().abs()).to_radians();
            assert!((facing - Vec3::new(spin.cos(), 0.0, -spin.sin())).length() < 1e-4, "t {t}");
        }
        let half = 0.5f32;
        let curve = 10.25 * half.powi(5) - 24.95 * half.powi(4) + 25.5 * half.powi(3) - 13.8 * half * half + 4.0 * half;
        assert!((curve - 0.4984375).abs() < 1e-5);
        assert!(centre(at(20, 0.0)).z > -1.9);
        let mut activation = ItemActivation::default();
        let mut draws = [0.25f32, 0.75].into_iter();
        activation.activate(|| draws.next().unwrap());
        assert_eq!(activation, ItemActivation { ticks: 40, off: [-0.5, 0.5] });
        activation.tick(39);
        assert!(activation.pose(0.0, 1.0).is_some());
        activation.tick(2);
        assert!(activation.pose(0.0, 1.0).is_none());
    }

    #[test]
    fn the_left_arm_mirrors_the_right_through_a_swing() {
        let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        for swing in [0.0, 0.25, 0.5, 1.0] {
            let right = item_pose(Mat4::IDENTITY, swing, 0.0, 1.0);
            let left = item_pose(Mat4::IDENTITY, swing, 0.0, -1.0);
            assert!(left.abs_diff_eq(flip * right * flip, 1e-5));
        }
        let rest = item_pose(Mat4::IDENTITY, 0.0, 0.0, -1.0).transform_point3(Vec3::ZERO);
        assert!((rest - Vec3::new(-0.56, -0.52, -0.72)).length() < 1e-6);
    }

    #[test]
    fn eating_starts_from_the_idle_pose_before_the_first_tick() {
        assert!(eat_transform(32.0, 0.0, 32, 1.0).abs_diff_eq(Mat4::IDENTITY, 1e-6));
        assert!(eat_transform(32.0, 0.7, 32, -1.0).abs_diff_eq(Mat4::IDENTITY, 1e-6));
    }

    #[test]
    fn eating_holds_the_item_at_the_mouth_and_bobs_every_four_ticks() {
        let middle = eat_transform(16.0, 0.0, 32, 1.0).transform_point3(Vec3::ZERO);
        let bob = (17.0f32 / 4.0 * std::f32::consts::PI).cos().abs() * 0.1;
        assert!((middle - Vec3::new(0.6, -0.5 + bob, 0.0)).length() < 1e-4);
        assert!((bob - 0.0707).abs() < 1e-3);
        let crest = eat_transform(15.0, 0.0, 32, 1.0).transform_point3(Vec3::ZERO);
        assert!((crest.y - (-0.4)).abs() < 1e-4);
        let early = eat_transform(30.0, 0.0, 32, 1.0);
        let lift = 1.0 - (31.0f64 / 32.0).powf(27.0) as f32;
        assert!((early.transform_point3(Vec3::ZERO) - Vec3::new(0.6 * lift, -0.5 * lift, 0.0)).length() < 1e-5);
    }

    #[test]
    fn the_left_arm_mirrors_the_right_while_eating() {
        let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        for remaining in [31.0, 24.0, 9.0, 1.0] {
            let right = eat_transform(remaining, 0.4, 32, 1.0) * item_pose(Mat4::IDENTITY, 0.0, 0.0, 1.0);
            let left = eat_transform(remaining, 0.4, 32, -1.0) * item_pose(Mat4::IDENTITY, 0.0, 0.0, -1.0);
            assert!(left.abs_diff_eq(flip * right * flip, 1e-5));
        }
    }

    #[test]
    fn a_charging_trident_is_raised_and_pulled_back_over_half_a_second() {
        let start = trident_charge_transform(1.0, 1.0).transform_point3(Vec3::ZERO);
        assert!((start - Vec3::new(-0.5, 0.7, 0.1)).length() < 1e-5, "{start}");
        let unturn = Mat4::from_rotation_y(45.0f32.to_radians());
        let back = |used: f32| (trident_charge_transform(used, 1.0) * unturn).transform_vector3(Vec3::Z).length();
        assert!((back(1.0) - 1.0).abs() < 1e-5);
        assert!((back(11.0) - 1.2).abs() < 1e-5);
        assert!((back(40.0) - 1.2).abs() < 1e-5);
        let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        for used in [1.0, 4.5, 12.0] {
            let right = trident_charge_transform(used, 1.0);
            let left = trident_charge_transform(used, -1.0);
            assert!(left.abs_diff_eq(flip * right * flip, 1e-5));
        }
    }

    #[test]
    fn dried_kelp_reaches_the_same_pose_in_half_the_ticks() {
        for (kelp, bread) in [(15.0, 30.0), (8.0, 16.0)] {
            let kelp_lift = eat_transform(kelp, 1.0, 16, 1.0).transform_point3(Vec3::ZERO).x;
            let bread_lift = eat_transform(bread, 1.0, 32, 1.0).transform_point3(Vec3::ZERO).x;
            assert!((kelp_lift - bread_lift).abs() < 1e-5);
        }
    }

    fn using(hand: minecraft_net::messages::Hand, ticks: i32, kind: minecraft_net::messages::UseKind) -> Option<minecraft_net::messages::UsingView> {
        Some(minecraft_net::messages::UsingView { hand, ticks, kind })
    }

    #[test]
    fn a_friends_use_counter_counts_eating_down_between_views() {
        use minecraft_net::messages::{Hand, UseKind};
        let mut clock = UseClock::default();
        let steak = using(Hand::Main, 32, UseKind::Eat { duration: 32 });
        let eat = |remaining| Some(HeldUse { slot: 3, kind: UseShown::Eat { remaining, duration: 32 } });
        assert_eq!(clock.follow(steak, 100, 3), eat(32));
        assert_eq!(clock.follow(steak, 110, 3), eat(22));
        assert_eq!(clock.follow(steak, 140, 3), eat(0), "it stays at the mouth until the host says it is done");
        let resent = using(Hand::Main, 20, UseKind::Eat { duration: 32 });
        assert_eq!(clock.follow(resent, 141, 3), eat(20), "a new view re-bases the counter");
        assert_eq!(clock.follow(resent, 145, 3), eat(16));
        assert_eq!(clock.follow(None, 146, 3), None);
        let kelp = using(Hand::Off, 16, UseKind::Eat { duration: 16 });
        assert_eq!(clock.follow(kelp, 150, 3), Some(HeldUse { slot: minecraft_survival::offhand::OFFHAND, kind: UseShown::Eat { remaining: 16, duration: 16 } }));
        assert_eq!(clock.follow(kelp, 158, 3).map(|held| held.kind), Some(UseShown::Eat { remaining: 8, duration: 16 }));
        assert_eq!(clock.follow(kelp, 2, 3).map(|held| held.kind), Some(UseShown::Eat { remaining: 16, duration: 16 }), "a new level's clock starts again");
    }

    #[test]
    fn a_friends_trident_charge_counts_up_and_a_shield_and_spyglass_hold() {
        use minecraft_net::messages::{Hand, UseKind};
        let mut clock = UseClock::default();
        let charge = using(Hand::Main, 2, UseKind::Charge);
        assert_eq!(clock.follow(charge, 10, 0), Some(HeldUse { slot: 0, kind: UseShown::Charge { ticks: 2 } }));
        assert_eq!(clock.follow(charge, 18, 0).map(|held| held.kind), Some(UseShown::Charge { ticks: 10 }));
        let shield = using(Hand::Off, 0, UseKind::Block);
        assert_eq!(clock.follow(shield, 20, 0), Some(HeldUse { slot: minecraft_survival::offhand::OFFHAND, kind: UseShown::Block }));
        assert_eq!(clock.follow(using(Hand::Main, 5, UseKind::Scope), 21, 7), Some(HeldUse { slot: 7, kind: UseShown::Scope }));
    }

    #[test]
    fn scoping_hides_both_held_items() {
        let sword = minecraftoss_player::inventory::ItemStack::new("minecraft:diamond_sword", 1);
        let shield = minecraftoss_player::inventory::ItemStack::new("minecraft:shield", 1);
        let mut held = Held { main: Some((2, &sword)), offhand: Some(&shield), swing: 0.0, swing_offhand: false, using: None, partial: 0.0, activation: None };
        let shown = |held: &Held<'_>| posed_stacks(held).map(|(stack, arm)| (stack.map(|(slot, stack)| (slot, stack.id.clone())), arm));
        assert_eq!(shown(&held), [(Some((2, "minecraft:diamond_sword".to_owned())), 1.0), (Some((minecraft_survival::offhand::OFFHAND, "minecraft:shield".to_owned())), -1.0)]);
        held.using = Some(HeldUse { slot: 2, kind: UseShown::Scope });
        assert_eq!(shown(&held), [(None, 1.0), (None, -1.0)]);
        held.using = Some(HeldUse { slot: minecraft_survival::offhand::OFFHAND, kind: UseShown::Block });
        assert!(shown(&held).iter().all(|(stack, _)| stack.is_some()));
    }

    #[test]
    fn the_host_hand_uses_eating_before_a_shield_a_trident_or_a_spyglass() {
        let mut survival = minecraft_survival::survival::Survival::default();
        let mut hand = minecraft_survival::player::Hand::default();
        assert_eq!(host_use(&survival, &hand), None);
        hand.scoping = Some(minecraft_survival::spyglass::Scoping { slot: 1, ticks: 4 });
        assert_eq!(host_use(&survival, &hand), Some(HeldUse { slot: 1, kind: UseShown::Scope }));
        hand.charging = Some(minecraft_survival::trident::Charging { slot: 1, ticks: 6 });
        assert_eq!(host_use(&survival, &hand), Some(HeldUse { slot: 1, kind: UseShown::Charge { ticks: 6 } }));
        hand.blocking = Some(minecraft_survival::offhand::Blocking { slot: 40, ticks: 2 });
        assert_eq!(host_use(&survival, &hand), Some(HeldUse { slot: 40, kind: UseShown::Block }));
        survival.status.food.level = 4;
        assert!(survival.start_eating(0, &minecraftoss_player::inventory::ItemStack::new("minecraft:dried_kelp", 1)));
        assert_eq!(host_use(&survival, &hand), Some(HeldUse { slot: 0, kind: UseShown::Eat { remaining: 16, duration: 16 } }));
    }
}
