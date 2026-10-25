use super::*;
use crate::skinned::skin_positions;
use petramond::mob::Mob;
use petramond_math::world_pos::WorldPos;
use petramond_world::bbmodel::clips::{self, Gait};

fn owl_model() -> Model {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/models/owl.bbmodel"
    ));
    Model::load(src).expect("owl model")
}

fn instance(anim_time: f32, moving: bool) -> MobRenderInstance {
    MobRenderInstance {
        kind: Mob::Owl,
        pos: WorldPos::new(10.0, 64.0, -5.0),
        yaw: 0.0,
        tilt: petramond_math::math::Tilt::LEVEL,
        anim_time,
        gait: moving.then_some(Gait::Walk),
        lean: 0.0,
        gait_weight: 1.0,
        gait_fades: crate::ArenaRange::default(),
        head_yaw: 0.0,
        head_pitch: 0.0,
        skylight: 63,
        blocklight: petramond_world::light::BlockLight6::DARK,
        hurt: 0.0,
        shorn: false,
        emitter_tint: [1.0, 1.0, 1.0],
        emitter_self_lit: 0.0,
        anims: crate::ArenaRange::default(),
        ragdoll: None,
        ragdoll_span: None,
        held: [None; 2],
    }
}

fn pose_with(
    model: &Model,
    scale: f32,
    insts: &[MobRenderInstance],
    layers: MobLayers<'_>,
    rig: &MobRig,
) -> SkinBatch {
    let mut batch = SkinBatch::default();
    let range = pose_mob_instances(
        MobPoseSpecies {
            model,
            scale,
            rig,
            cache: &mut MobPoseCache::default(),
        },
        insts,
        layers,
        petramond_math::math::IVec3::ZERO,
        &mut batch,
        &mut Vec::new(),
    );
    assert_eq!(range, 0..insts.len() as u32);
    batch
}

fn pose(model: &Model, scale: f32, insts: &[MobRenderInstance], rig: &MobRig) -> SkinBatch {
    let layers = MobLayers {
        arena: &crate::MobArena::default(),
        names: &crate::AnimNames::default(),
    };
    pose_with(model, scale, insts, layers, rig)
}

fn skin(model: &Model, scale: f32, inst: &MobRenderInstance, rig: &MobRig) -> Vec<Vec3> {
    let batch = pose(model, scale, std::slice::from_ref(inst), rig);
    skin_positions(&rig.mesh(model, scale), &batch, 0)
}

fn skin_with(
    model: &Model,
    insts: &[MobRenderInstance],
    layers: MobLayers<'_>,
    rig: &MobRig,
    i: u32,
) -> Vec<Vec3> {
    let batch = pose_with(model, 0.25, insts, layers, rig);
    skin_positions(&rig.mesh(model, 0.25), &batch, i)
}

#[test]
fn each_instance_gets_its_own_palette_run() {
    let m = owl_model();
    let rig = MobRig::resolve(&m, None, None);
    let batch = pose(&m, 0.25, &[instance(0.0, true), instance(0.3, true)], &rig);
    let slots = bone_slots(&m);
    assert_eq!(batch.palette.len() as u32, 2 * slots);
    assert_eq!(batch.instances[0].bone_base, 0);
    assert_eq!(batch.instances[1].bone_base, slots);
    let mut batch = batch;
    let range = pose_mob_instances(
        MobPoseSpecies {
            model: &m,
            scale: 0.25,
            rig: &rig,
            cache: &mut MobPoseCache::default(),
        },
        std::slice::from_ref(&instance(0.0, false)),
        MobLayers {
            arena: &crate::MobArena::default(),
            names: &crate::AnimNames::default(),
        },
        petramond_math::math::IVec3::ZERO,
        &mut batch,
        &mut Vec::new(),
    );
    assert_eq!(range, 2..3);
}

#[test]
fn moving_plays_walk_idle_uses_rest_pose() {
    let m = owl_model();
    let rig = MobRig::resolve(&m, None, None);
    let at = |t: f32, moving: bool| skin(&m, 0.25, &instance(t, moving), &rig);
    let walk_a = at(0.0, true);
    let walk_b = at(0.25, true);
    let moved = walk_a
        .iter()
        .zip(&walk_b)
        .any(|(a, b)| (a.z - b.z).abs() > 1e-3);
    assert!(moved, "walking mob's legs move between phases");

    let rest_a = at(0.0, false);
    let rest_b = at(0.25, false);
    assert!(
        rest_a == rest_b,
        "idle mob ignores anim_time (always the rest pose)"
    );
}

#[test]
fn head_look_rotates_the_head_when_idle() {
    let m = owl_model();
    let rig = MobRig::resolve(&m, None, None);
    let looking = |head_yaw: f32| {
        let mut inst = instance(0.0, false);
        inst.head_yaw = head_yaw;
        skin(&m, 0.25, &inst, &rig)
    };
    let straight = looking(0.0);
    let turned = looking(1.0);
    assert!(
        straight.iter().zip(&turned).any(|(a, b)| a != b),
        "head-look should rotate the head when idle"
    );
}

#[test]
fn arena_ranges_pose_each_instance_from_its_own_layers() {
    let m = owl_model();
    let rig = MobRig::resolve(&m, None, None);
    let mut interner = crate::AnimInterner::default();
    let walk = interner.intern(clips::WALK);
    let missing = interner.intern("no_such_clip");
    assert_eq!(interner.intern(clips::WALK), walk, "a name interns once");
    let arena = crate::MobArena {
        gait_fades: vec![crate::GaitFade {
            clip: Gait::Walk,
            phase: 0.25,
            weight: 1.0,
        }],
        anims: vec![
            crate::AnimLayer {
                anim: missing,
                phase: 0.25,
                weight: 1.0,
            },
            crate::AnimLayer {
                anim: walk,
                phase: 0.25,
                weight: 1.0,
            },
        ],
        ragdoll: Vec::new(),
    };
    let layers = MobLayers {
        arena: &arena,
        names: interner.names(),
    };
    let mut named = instance(0.0, false);
    named.anims = crate::ArenaRange { start: 0, len: 2 };
    let mut fading = instance(0.0, false);
    fading.gait_fades = crate::ArenaRange { start: 0, len: 1 };
    let mut unknown = instance(0.0, false);
    unknown.anims = crate::ArenaRange { start: 0, len: 1 };
    let insts = [named, fading, unknown, instance(0.0, false)];

    let walking = skin(&m, 0.25, &instance(0.25, true), &rig);
    let rest = skin(&m, 0.25, &instance(0.0, false), &rig);
    assert_eq!(skin_with(&m, &insts, layers, &rig, 0), walking);
    assert_eq!(skin_with(&m, &insts, layers, &rig, 1), walking);
    assert_eq!(skin_with(&m, &insts, layers, &rig, 2), rest);
    assert_eq!(skin_with(&m, &insts, layers, &rig, 3), rest);
}

#[test]
fn anim_clips_follow_the_table_that_names_the_id() {
    let m = owl_model();
    let mut ours = crate::AnimInterner::default();
    let mut theirs = crate::AnimInterner::default();
    let id = ours.intern(clips::WALK);
    assert_eq!(theirs.intern("no_such_clip"), id, "both tables start at 0");
    fn ptr(clip: Option<&Animation>) -> Option<*const Animation> {
        clip.map(|a| a as *const Animation)
    }
    let walk = ptr(m.animation(clips::WALK));
    assert!(walk.is_some(), "the fixture has a walk clip");
    let mut cache = AnimClips::default();
    assert_eq!(ptr(cache.resolve(&m, ours.names(), id)), walk);
    assert_eq!(ptr(cache.resolve(&m, ours.names(), id)), walk);
    assert_eq!(ptr(cache.resolve(&m, theirs.names(), id)), None);
    assert_eq!(ptr(cache.resolve(&m, ours.names(), id)), walk);
    assert!(
        cache.resolve(&m, ours.names(), crate::AnimId(9)).is_none(),
        "an id outside the table resolves to nothing"
    );
}

/// A ragdoll's bones move inside the frame frozen where it died: a body that slid away and sank
/// lies where its bones are, which is what culls it, lights it and casts its shadow.
#[test]
fn a_ragdolls_span_follows_its_bones_away_from_where_it_died() {
    let died = WorldPos::new(10.0, 64.0, -5.0);
    let scale = 0.25;
    let yaw = std::f32::consts::FRAC_PI_2;
    // Model space: the bones slid 24 units (6 blocks) along the body and sank 40 (10 blocks).
    let bones = [
        (Vec3::new(-2.0, -40.0, 22.0), glam::Quat::IDENTITY),
        (Vec3::new(2.0, -38.0, 26.0), glam::Quat::IDENTITY),
    ];
    let span = super::ragdoll_span(died, yaw, petramond_math::math::Tilt::LEVEL, scale, &bones)
        .expect("bones give a span");
    let frame = petramond_math::math::Tilt::LEVEL.body_frame(yaw);
    for (p, _) in bones {
        let at = died + frame.transform_vector3(p * scale);
        for (lo, v, hi) in [
            (span.min.x, at.x, span.max.x),
            (span.min.y, at.y, span.max.y),
            (span.min.z, at.z, span.max.z),
        ] {
            assert!(lo - 1e-4 <= v && v <= hi + 1e-4, "{at:?} outside {span:?}");
        }
    }
    let ground = span.ground();
    assert!(
        (ground.y - (died.y - 10.0)).abs() < 1e-3,
        "it rests 10 blocks down: {ground:?}"
    );
    let moved = ((ground.x - died.x).powi(2) + (ground.z - died.z).powi(2)).sqrt();
    assert!((moved - 6.0).abs() < 0.1, "it slid 6 blocks away: {moved}");
    assert!(
        super::ragdoll_span(died, yaw, petramond_math::math::Tilt::LEVEL, scale, &[]).is_none()
    );
}
