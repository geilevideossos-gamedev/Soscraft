use crate::anim::remote_body::{CpuBodyGeom, CpuSurfMeta};
use bevy::prelude::*;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
pub struct ReferenceBone {
    name: String,
    bind: [f32; 16],
    inverse_bind: [f32; 16],
}
pub fn reference() -> Option<&'static Vec<ReferenceBone>> {
    static RIG: OnceLock<Option<Vec<ReferenceBone>>> = OnceLock::new();
    RIG.get_or_init(|| {
        let root = std::env::var_os("IW4L_SKATE_ASSETS")?;
        let data = std::fs::read(std::path::Path::new(&root).join("rig.json")).ok()?;
        serde_json::from_slice(&data).ok()
    })
    .as_ref()
}

fn mapping(name: &str) -> Option<(&'static str, Option<(&'static str, &'static str)>)> {
    Some(match name {
        "j_mainroot" | "pelvis" => ("HIPS", Some(("j_spinelower", "SPINE"))),
        "j_spinelower" => ("SPINE", Some(("j_spineupper", "SPINE1"))),
        "j_spineupper" => ("SPINE1", Some(("j_spine4", "SPINE3"))),
        "j_spine4" => ("SPINE3", Some(("j_neck", "NECK"))),
        "j_neck" => ("NECK", Some(("j_head", "HEAD"))),
        "j_head" => ("HEAD", None),
        "j_clavicle_le" => ("LEFTSHOULDER", Some(("j_shoulder_le", "LEFTARM"))),
        "j_shoulder_le" => ("LEFTARM", Some(("j_elbow_le", "LEFTFOREARM"))),
        "j_elbow_le" => ("LEFTFOREARM", Some(("j_wrist_le", "LEFTHAND"))),
        "j_wrist_le" => ("LEFTHAND", None),
        "j_clavicle_ri" => ("RIGHTSHOULDER", Some(("j_shoulder_ri", "RIGHTARM"))),
        "j_shoulder_ri" => ("RIGHTARM", Some(("j_elbow_ri", "RIGHTFOREARM"))),
        "j_elbow_ri" => ("RIGHTFOREARM", Some(("j_wrist_ri", "RIGHTHAND"))),
        "j_wrist_ri" => ("RIGHTHAND", None),
        "j_hip_le" => ("LEFTUPLEG", Some(("j_knee_le", "LEFTLEG"))),
        "j_knee_le" => ("LEFTLEG", Some(("j_ankle_le", "LEFTFOOT"))),
        "j_ankle_le" => ("LEFTFOOT", Some(("j_ball_le", "LEFTTOEBASE"))),
        "j_ball_le" => ("LEFTTOEBASE", None),
        "j_hip_ri" => ("RIGHTUPLEG", Some(("j_knee_ri", "RIGHTLEG"))),
        "j_knee_ri" => ("RIGHTLEG", Some(("j_ankle_ri", "RIGHTFOOT"))),
        "j_ankle_ri" => ("RIGHTFOOT", Some(("j_ball_ri", "RIGHTTOEBASE"))),
        "j_ball_ri" => ("RIGHTTOEBASE", None),
        _ => return None,
    })
}

fn convert(m: Mat4) -> Mat4 {
    let b = super::collision::basis();
    let mut out = b * m * b.inverse();
    out.w_axis = super::collision::from_skate(m.w_axis.truncate()).extend(1.);
    out
}

/// Align each soldier bind segment to the reference skater, then apply the
/// game's solved pose. Unmapped fingers, equipment and helper bones inherit
/// the nearest mapped ancestor, preserving their original local offsets.
pub fn pose<N: AsRef<str>>(dobj: &xmodel_runtime::DObj, names: &[N], bones: &[Mat4]) -> Vec<Mat4> {
    let Some(reference) = reference() else {
        return dobj.bones.iter().map(|b| b.bind_world).collect();
    };
    pose_with(reference, dobj, names, bones)
}

fn pose_with<N: AsRef<str>>(
    reference: &[ReferenceBone],
    dobj: &xmodel_runtime::DObj,
    names: &[N],
    bones: &[Mat4],
) -> Vec<Mat4> {
    let source = |name: &str| {
        reference
            .iter()
            .find(|b| b.name == name)
            .map(|b| convert(Mat4::from_cols_array(&b.bind)))
    };
    let mut skin = Vec::with_capacity(dobj.bones.len());
    for bone in &dobj.bones {
        if bone.model != 0 {
            // Attached models use model-local bind positions. Their root must
            // start at the posed attachment bone, not at its skinning delta.
            let attachment = bone.parent.and_then(|parent| {
                let transform = *skin.get(parent)?;
                Some(if dobj.bones[parent].model == bone.model {
                    transform
                } else {
                    transform * dobj.bones[parent].bind_world
                })
            });
            skin.push(attachment.unwrap_or(Mat4::IDENTITY));
            continue;
        }
        if matches!(bone.name.as_str(), "j_spine4" | "j_neck" | "j_head") {
            skin.push(
                bone.parent
                    .and_then(|parent| skin.get(parent).copied())
                    .unwrap_or(Mat4::IDENTITY),
            );
            continue;
        }
        let mapped = mapping(&bone.name).and_then(|(name, child)| {
            let bind = source(name)?;
            let index = names.iter().position(|n| n.as_ref() == name)?;
            let posed = convert(*bones.get(index)?);
            let from = bone.bind_world.w_axis.truncate();
            let to = bind.w_axis.truncate();
            let facing = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
            let rotation = child
                .and_then(|(iw, skate)| {
                    let iw = dobj
                        .bones
                        .iter()
                        .find(|b| b.name == iw)?
                        .bind_world
                        .w_axis
                        .truncate();
                    let sk = source(skate)?.w_axis.truncate();
                    Some(
                        Quat::from_rotation_arc(
                            (facing * (iw - from)).try_normalize()?,
                            (sk - to).try_normalize()?,
                        ) * facing,
                    )
                })
                .unwrap_or_else(|| {
                    // Terminal joints use their parent's alignment rather than a
                    // new arbitrary roll, keeping wrists and head seams attached.
                    bone.parent
                        .and_then(|p| dobj.bones.get(p))
                        .and_then(|p| mapping(&p.name))
                        .and_then(|(pn, _)| {
                            let s = source(pn)?.w_axis.truncate();
                            let p = p_from(dobj, bone.parent)?;
                            Some(
                                Quat::from_rotation_arc(
                                    (facing * (from - p)).try_normalize()?,
                                    (to - s).try_normalize()?,
                                ) * facing,
                            )
                        })
                        .unwrap_or(facing)
                });
            let fit = Mat4::from_rotation_translation(rotation, to - rotation * from);
            Some(posed * bind.inverse() * fit)
        });
        skin.push(
            mapped
                .or_else(|| bone.parent.and_then(|p| skin.get(p).copied()))
                .unwrap_or(Mat4::IDENTITY),
        );
    }
    dobj.bones
        .iter()
        .zip(skin)
        .map(|(b, s)| s * b.bind_world)
        .collect()
}
fn p_from(dobj: &xmodel_runtime::DObj, index: Option<usize>) -> Option<Vec3> {
    Some(dobj.bones.get(index?)?.bind_world.w_axis.truncate())
}

fn board_skin<'a, N: AsRef<str>>(
    reference: &[ReferenceBone],
    targets: impl Iterator<Item = &'a str>,
    names: &[N],
    bones: &[Mat4],
) -> Result<Vec<Mat4>, String> {
    let rb = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let skin = |target: &str| {
        let i = names.iter().position(|n| n.as_ref() == target)?;
        let reference = reference.iter().find(|b| b.name == target)?;
        Some(*bones.get(i)? * rb * Mat4::from_cols_array(&reference.inverse_bind))
    };
    let deck = skin("SKATEBOARD_ROOT");
    targets
        .map(|target| skin(target).or(deck).ok_or_else(|| format!("missing Skate joint {target}")))
        .collect()
}

pub fn board<N: AsRef<str>>(names: &[N], bones: &[Mat4], geom: &mut CpuBodyGeom) -> Result<(), String> {
    let model = assets::bot_model::local_skate_board().ok_or("missing board model")?;
    let reference = reference().ok_or("missing board bind pose")?;
    let targets = model.joints.iter().map(|j| j.target.as_str());
    let matrices = board_skin(reference, targets, names, bones)?;
    for surface in &model.surfaces {
        let vertex_base = geom.packed.len() as u32;
        let index_start = geom.indices.len() as u32;
        for v in &surface.vertices {
            let mut pos = Vec3::ZERO;
            let mut normal = Vec3::ZERO;
            for i in 0..4 {
                let m = matrices[v.joints[i]];
                pos += m.transform_point3(Vec3::from_array(v.position)) * v.weights[i];
                normal += m.transform_vector3(Vec3::from_array(v.normal)) * v.weights[i];
            }
            pos = super::collision::from_skate(pos);
            normal = super::collision::basis()
                .transform_vector3(normal)
                .normalize_or(Vec3::Z);
            let mut packed = [0; asset_iw4::size::GFX_PACKED_VERTEX];
            for (i, f) in pos.to_array().iter().enumerate() {
                packed[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
            }
            packed[12..16].copy_from_slice(&1f32.to_le_bytes());
            packed[16..20].fill(255);
            packed[20..24].copy_from_slice(&v.uv.to_le_bytes());
            let pack = |n: Vec3| {
                [
                    (n.x * 127. + 127.5) as u8,
                    (n.y * 127. + 127.5) as u8,
                    (n.z * 127. + 127.5) as u8,
                    63,
                ]
            };
            packed[24..28].copy_from_slice(&pack(normal));
            packed[28..32].copy_from_slice(&pack(normal.any_orthonormal_vector()));
            geom.packed.push(packed);
        }
        geom.indices
            .extend(surface.indices.iter().map(|i| vertex_base + i));
        geom.surfaces.push(CpuSurfMeta {
            index_start,
            index_count: surface.indices.len() as u32,
            name: Some(surface.material.clone()),
        });
    }
    geom.decoded_n = geom.packed.len();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame::skate::SKATE_BONES;

    fn affine(seed: f32) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            Vec3::ONE,
            Quat::from_euler(EulerRot::XYZ, seed * 0.3, seed * -0.2, seed * 0.7),
            Vec3::new(seed, seed * 2.0 - 1.0, 0.5 - seed),
        )
    }

    fn reference_bone(name: &str, seed: f32) -> ReferenceBone {
        let bind = affine(seed);
        ReferenceBone {
            name: name.to_owned(),
            bind: bind.to_cols_array(),
            inverse_bind: bind.inverse().to_cols_array(),
        }
    }

    fn soldier() -> xmodel_runtime::DObj {
        let joints = [
            ("j_mainroot", None),
            ("j_spinelower", Some(0)),
            ("j_spineupper", Some(1)),
            ("j_spine4", Some(2)),
            ("j_neck", Some(3)),
            ("j_head", Some(4)),
            ("j_clavicle_le", Some(3)),
            ("j_shoulder_le", Some(6)),
            ("j_elbow_le", Some(7)),
            ("j_wrist_le", Some(8)),
            ("j_thumb_le_1", Some(9)),
            ("j_clavicle_ri", Some(3)),
            ("j_shoulder_ri", Some(11)),
            ("j_elbow_ri", Some(12)),
            ("j_wrist_ri", Some(13)),
            ("j_hip_le", Some(0)),
            ("j_knee_le", Some(15)),
            ("j_ankle_le", Some(16)),
            ("j_ball_le", Some(17)),
            ("j_hip_ri", Some(0)),
            ("j_knee_ri", Some(19)),
            ("j_ankle_ri", Some(20)),
            ("j_ball_ri", Some(21)),
        ];
        let at = |i: usize| Vec3::new(i as f32 * 0.1, (i % 3) as f32, 10.0 + i as f32);
        let model = xmodel_runtime::ModelPoseSrc {
            name: "soldier".to_owned(),
            num_bones: joints.len(),
            num_root_bones: 1,
            scale: 1.0,
            no_scale_part_bits: [0; 6],
            bone_names: joints.iter().map(|(name, _)| (*name).to_owned()).collect(),
            parent_list: joints
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, (_, parent))| (i - parent.unwrap()) as u8)
                .collect(),
            quats: vec![[0, 0, 0, i16::MAX]; joints.len() - 1],
            trans: vec![[1.0, 0.0, 0.0]; joints.len() - 1],
            base_mat: (0..joints.len()).map(|i| (Quat::IDENTITY, at(i))).collect(),
        };
        xmodel_runtime::DObj::build(&[(&model, None)]).unwrap()
    }

    fn rig() -> Vec<ReferenceBone> {
        SKATE_BONES
            .iter()
            .enumerate()
            .map(|(i, name)| reference_bone(name, i as f32 * 0.25 + 0.1))
            .chain([reference_bone("FRONTTRUCK", 9.0)])
            .collect()
    }

    fn engine() -> (Vec<String>, Vec<Mat4>) {
        let mut names: Vec<String> = SKATE_BONES.iter().rev().map(|n| (*n).to_owned()).collect();
        names.insert(3, "FRONTTRUCK".to_owned());
        names.insert(0, "LEFTFINGER0".to_owned());
        let bones = (0..names.len()).map(|i| affine(1.0 + i as f32 * 0.37)).collect();
        (names, bones)
    }

    #[test]
    fn the_pose_from_the_mapped_bones_equals_the_pose_from_every_skate_bone() {
        let (rig, dobj) = (rig(), soldier());
        let (names, bones) = engine();
        let full = pose_with(&rig, &dobj, &names, &bones);
        let shared = frame::SkatePose::from_named(Mat4::IDENTITY, &names, &bones).unwrap();
        assert_eq!(shared.bones.len(), SKATE_BONES.len());
        let reduced = pose_with(&rig, &dobj, &SKATE_BONES, &shared.bones);
        assert_eq!(full, reduced);
        let bind: Vec<_> = dobj.bones.iter().map(|b| b.bind_world).collect();
        assert!(
            full.iter().zip(&bind).filter(|(a, b)| a != b).count() > 15,
            "the skate pose moves the soldier"
        );
        let thumb = full[9] * dobj.bones[9].bind_world.inverse() * dobj.bones[10].bind_world;
        assert!(full[10].abs_diff_eq(thumb, 1e-4), "an unmapped finger follows its wrist");
    }

    #[test]
    fn board_joints_beyond_the_shared_bones_ride_the_deck() {
        let rig = rig();
        let (names, bones) = engine();
        let targets = ["SKATEBOARD_ROOT", "FRONTTRUCK"];
        let own = board_skin(&rig, targets.into_iter(), &names, &bones).unwrap();
        assert_ne!(own[0], own[1]);
        let shared = frame::SkatePose::from_named(Mat4::IDENTITY, &names, &bones).unwrap();
        let seen = board_skin(&rig, targets.into_iter(), &SKATE_BONES, &shared.bones).unwrap();
        assert_eq!(seen, vec![own[0], own[0]]);
        let no_deck = ["HIPS"];
        assert!(board_skin(&rig, targets.into_iter(), &no_deck, &bones[..1]).is_err());
    }
}
