use super::*;
use glam::Vec3;
use petramond::world::TrailPoint;
use petramond_math::world_pos::WorldPos;

fn inst(alpha: f32) -> ParticleInstance {
    ParticleInstance {
        quad_axes: None,
        pos: WorldPos::new(1.0, 2.0, 3.0),
        uv_min: [0.1, 0.2],
        uv_size: [0.05; 2],
        tint: [1.0, 1.0, 1.0],
        alpha,
        size: 0.1,
        skylight: lighting::FULL_SKYLIGHT,
        blocklight: petramond_world::light::BlockLight6::DARK,
    }
}

fn emitter_inst() -> ParticleEmitterInstance {
    ParticleEmitterInstance {
        origin: WorldPos::new(1.0, 2.0, 3.0),
        emitter: petramond_world::block::ParticleEmitter {
            anchor: petramond_world::block::ParticleEmitterAnchor::BlockTop,
            origin: [0.5, 1.0, 0.5],
            offset: [0.0, 0.0, 0.0],
            rate: [1.0, 1.0],
            lifetime: [1.0, 1.0],
            size: [0.2, 0.2],
            spawn_box: [0.0, 0.0, 0.0],
            velocity: [0.0, 1.0, 0.0],
            velocity_jitter: [0.0, 0.0, 0.0],
            color: Some([[1.0, 0.5, 0.0], [1.0, 1.0, 0.0]]),
            color_ramp: None,
            alpha: [0.8, 0.8],
            fade_power: 2.0,
            shrink_power: 1.0,
            self_lit: 1.0,
            spiral: [0.0, 0.0],
            requires_open: None,
            gravity: 0.0,
            lands: false,
            kill: petramond_world::block::EmitterKill::None,
            pulse: [0.0; 3],
            look: ParticleLook::Cube,
        },
        seed: 0x1234_5678_9ABC_DEF0,
        skylight: 0,
        blocklight: petramond_world::light::BlockLight6::DARK,
        floor_y: f32::NEG_INFINITY,
        ceiling_y: f32::INFINITY,
        trail: petramond::world::TrailSpan::default(),
    }
}

fn one_live_emitter_time(inst: &ParticleEmitterInstance, age: f32) -> f32 {
    let schedule = emitter_schedule(inst.seed, inst.emitter.rate);
    emitter_birth_time(inst.seed, schedule, 10) + age
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct ParticleVertex {
    pos: [f32; 3],
    uv: [f32; 2],
    tint: [f32; 3],
    shade: f32,
    alpha: f32,
}

/// Same expansion as `vs_particle`, just in Rust. Cubes: 24 verts, face = index / 4, corner =
/// index % 4 (bl, br, tr, tl). Oriented quads: first face only, the collapsed rest dropped.
fn expand(rows: &[ParticleRow]) -> Vec<ParticleVertex> {
    let mut out = Vec::new();
    for row in rows {
        let c = Vec3::from(row.center);
        for vertex in 0..VERTS_PER_CUBE {
            let (face, corner) = (vertex / 4, vertex % 4);
            let sx = if corner == 1 || corner == 2 {
                1.0
            } else {
                -1.0
            };
            let sy = if corner >= 2 { 1.0 } else { -1.0 };
            let (pos, shade) = if row.quad != 0 {
                if face != 0 {
                    continue;
                }
                let pos = c + Vec3::from(row.right) * sx + Vec3::from(row.up) * sy;
                (pos, 1.0)
            } else {
                let f = &FACES[face];
                let fc = c + f.right.cross(f.up) * row.half;
                let mut pos = fc + f.right * row.half * sx + f.up * row.half * sy;
                pos.y = c.y + (pos.y - c.y) * row.stretch;
                (pos, f.shade)
            };
            out.push(ParticleVertex {
                pos: pos.to_array(),
                uv: [
                    if corner == 1 || corner == 2 {
                        row.uv_max[0]
                    } else {
                        row.uv_min[0]
                    },
                    if corner >= 2 {
                        row.uv_min[1]
                    } else {
                        row.uv_max[1]
                    },
                ],
                tint: row.tint,
                shade,
                alpha: row.alpha,
            });
        }
    }
    out
}

fn vertex_center(v: &[ParticleVertex]) -> Vec3 {
    let sum = v.iter().fold(Vec3::ZERO, |acc, p| acc + Vec3::from(p.pos));
    sum / v.len() as f32
}

fn x_extent(v: &[ParticleVertex]) -> f32 {
    let min_x = v.iter().map(|p| p.pos[0]).fold(f32::INFINITY, f32::min);
    let max_x = v.iter().map(|p| p.pos[0]).fold(f32::NEG_INFINITY, f32::max);
    max_x - min_x
}

fn max_alpha(v: &[ParticleVertex]) -> f32 {
    v.iter().map(|p| p.alpha).fold(0.0, f32::max)
}

#[test]
fn oriented_particles_keep_their_plane_and_atlas_split_among_cubes() {
    let right = Vec3::new(0.2, 0.3, 0.0);
    let up = Vec3::Z * 0.4;
    let quad = ParticleInstance {
        quad_axes: Some([right, up]),
        ..inst(1.0)
    };
    let mut rows = Vec::new();
    let (total, block) = build_particles_split(
        &[quad, inst(1.0)],
        &[inst(1.0)],
        LightEnv::IDENTITY,
        petramond_math::math::IVec3::ZERO,
        &mut rows,
    );
    assert_eq!(
        (total, block),
        (3, 2),
        "one row per particle, block rows first"
    );
    let vertices = expand(&rows);
    assert_eq!(expand(&rows[..2]).len(), 4 + VERTS_PER_CUBE);
    assert_eq!(expand(&rows[2..]).len(), VERTS_PER_CUBE);
    let normal = right.cross(up).normalize();
    for vertex in &vertices[..4] {
        assert!(
            (Vec3::from(vertex.pos) - quad.pos.relative_to(glam::IVec3::ZERO))
                .dot(normal)
                .abs()
                < 1e-6
        );
    }
    assert_eq!(
        vertices[0].pos,
        (quad.pos - right - up)
            .relative_to(glam::IVec3::ZERO)
            .to_array()
    );
    assert_eq!(
        vertices[2].pos,
        (quad.pos + right + up)
            .relative_to(glam::IVec3::ZERO)
            .to_array()
    );
}

#[test]
fn block_emitter_particles_rise_shrink_and_fade() {
    let inst = emitter_inst();
    let mut young = Vec::new();
    let mut old = Vec::new();
    let mut scratch = Vec::new();

    let young_n = build_transparent_emitter_particles(
        std::slice::from_ref(&inst),
        &[],
        &[],
        one_live_emitter_time(&inst, 0.25),
        petramond_math::math::IVec3::ZERO,
        Vec3::ZERO,
        f32::MAX,
        LightEnv::IDENTITY,
        1.0,
        &mut young,
        &mut scratch,
    );
    let old_n = build_transparent_emitter_particles(
        std::slice::from_ref(&inst),
        &[],
        &[],
        one_live_emitter_time(&inst, 0.75),
        petramond_math::math::IVec3::ZERO,
        Vec3::ZERO,
        f32::MAX,
        LightEnv::IDENTITY,
        1.0,
        &mut old,
        &mut scratch,
    );

    assert_eq!((young_n, old_n), (1, 1), "one live particle, one row");
    let (young, old) = (expand(&young), expand(&old));
    assert_eq!(young.len(), VERTS_PER_CUBE);
    assert!(
        vertex_center(&old).y > vertex_center(&young).y,
        "emitter particles move upward over their lifetime"
    );
    assert!(
        x_extent(&old) < x_extent(&young),
        "emitter particles shrink as they age"
    );
    assert!(
        max_alpha(&old) < max_alpha(&young),
        "emitter particles fade as they age"
    );
}

#[test]
fn spiral_emitter_particles_orbit_the_vertical_axis_as_they_age() {
    let mut inst = emitter_inst();
    inst.emitter.spiral = [0.5, 1.0];
    let mut early = Vec::new();
    let mut late = Vec::new();
    let mut scratch = Vec::new();

    build_transparent_emitter_particles(
        std::slice::from_ref(&inst),
        &[],
        &[],
        one_live_emitter_time(&inst, 0.25),
        petramond_math::math::IVec3::ZERO,
        Vec3::ZERO,
        f32::MAX,
        LightEnv::IDENTITY,
        1.0,
        &mut early,
        &mut scratch,
    );
    build_transparent_emitter_particles(
        std::slice::from_ref(&inst),
        &[],
        &[],
        one_live_emitter_time(&inst, 0.5),
        petramond_math::math::IVec3::ZERO,
        Vec3::ZERO,
        f32::MAX,
        LightEnv::IDENTITY,
        1.0,
        &mut late,
        &mut scratch,
    );

    let axis = inst.origin;
    let horiz = |rows: &[ParticleRow]| {
        let c = vertex_center(&expand(rows));
        Vec3::new(c.x - axis.x as f32, 0.0, c.z - axis.z as f32)
    };
    let (a, b) = (horiz(&early), horiz(&late));
    assert!(
        (a.length() - b.length()).abs() < 1e-4,
        "one particle keeps its orbit radius: {} vs {}",
        a.length(),
        b.length()
    );
    assert!(
        (0.3 - 1e-4..=0.5 + 1e-4).contains(&a.length()),
        "orbit radius stays within the row's spiral radius: {}",
        a.length()
    );
    assert!(
        a.angle_between(b) > 0.3,
        "a quarter nominal revolution rotates the offset (angle {})",
        a.angle_between(b)
    );
}

#[test]
fn ramp_emitter_particles_cool_through_the_ramp_as_they_age() {
    let mut inst = emitter_inst();
    let ramp: petramond_world::block::ColorRamp =
        serde_json::from_str(r#"[[1.0, 1.0, 0.9], [1.0, 0.5, 0.1], [0.1, 0.1, 0.1]]"#)
            .expect("3-stop ramp parses");
    inst.emitter.color = None;
    inst.emitter.color_ramp = Some(ramp);
    let mut young = Vec::new();
    let mut old = Vec::new();
    let mut scratch = Vec::new();

    build_transparent_emitter_particles(
        std::slice::from_ref(&inst),
        &[],
        &[],
        one_live_emitter_time(&inst, 0.1),
        petramond_math::math::IVec3::ZERO,
        Vec3::ZERO,
        f32::MAX,
        LightEnv::IDENTITY,
        1.0,
        &mut young,
        &mut scratch,
    );
    build_transparent_emitter_particles(
        std::slice::from_ref(&inst),
        &[],
        &[],
        one_live_emitter_time(&inst, 0.9),
        petramond_math::math::IVec3::ZERO,
        Vec3::ZERO,
        f32::MAX,
        LightEnv::IDENTITY,
        1.0,
        &mut old,
        &mut scratch,
    );

    let luma = |rows: &[ParticleRow]| {
        let t = rows[0].tint;
        t[0] + t[1] + t[2]
    };
    assert!(
        luma(&young) > 2.0,
        "a young particle sits near the hot end of the ramp: {}",
        luma(&young)
    );
    assert!(
        luma(&old) < 1.0,
        "an old particle has cooled toward the dark end: {}",
        luma(&old)
    );
}

#[test]
fn block_emitter_rate_range_jitters_spawn_intervals() {
    let seed = 0xCAFE_BABE_F00D_1234;
    let schedule = emitter_schedule(seed, [1.0, 2.0]);
    let mut saw_variation = false;
    let mut prev = emitter_birth_time(seed, schedule, 0);

    for seq in 1..64 {
        let birth = emitter_birth_time(seed, schedule, seq);
        let gap = birth - prev;
        assert!(
            (0.5..=1.0).contains(&gap),
            "rate range 1.0-2.0 should produce gaps in 0.5-1.0 seconds, got {gap}"
        );
        saw_variation |= (gap - schedule.base_gap).abs() > 0.01;
        prev = birth;
    }

    assert!(
        saw_variation,
        "range emitters should not spawn on a fixed cadence"
    );
}

#[test]
fn the_row_stride_matches_its_declared_layout() {
    assert_eq!(std::mem::size_of::<ParticleRow>(), 80);
    assert_eq!(std::mem::offset_of!(ParticleRow, uv_min), 48);
    assert_eq!(std::mem::offset_of!(ParticleRow, tint), 64);
    assert_eq!(std::mem::offset_of!(ParticleRow, quad), 76);
}

#[test]
fn landing_particles_fall_and_vanish_at_their_floor() {
    let mut inst = emitter_inst();
    inst.emitter.velocity = [0.0, 0.0, 0.0];
    inst.emitter.gravity = 10.0;
    inst.emitter.lifetime = [2.0, 2.0];
    inst.emitter.rate = [1.0, 1.0];
    inst.emitter.lands = true;
    inst.floor_y = (inst.origin.y - 2.0) as f32;
    let mut scratch = Vec::new();
    let mut rows = Vec::new();
    let mut fell = false;
    for step in 0..40 {
        let t = one_live_emitter_time(&inst, 0.05 * step as f32);
        build_transparent_emitter_particles(
            std::slice::from_ref(&inst),
            &[],
            &[],
            t,
            petramond_math::math::IVec3::ZERO,
            Vec3::ZERO,
            f32::MAX,
            LightEnv::IDENTITY,
            1.0,
            &mut rows,
            &mut scratch,
        );
        for cube in &scratch {
            assert!(
                cube.pos.y - cube.size * 0.5 > inst.floor_y,
                "a particle drawn under its floor at t={t}: {:?}",
                cube.pos
            );
            if f64::from(cube.pos.y) < inst.origin.y - 0.5 {
                fell = true;
            }
        }
    }
    assert!(fell, "gravity carried a particle down before it landed");
}

fn solid(pos: WorldPos, size: f32, stretch: f32, speck: bool) -> crate::SolidParticleInstance {
    let look = if speck {
        ParticleLook::Speck
    } else {
        ParticleLook::Cube
    };
    crate::SolidParticleInstance {
        pos,
        color: [1.0; 3],
        alpha: 0.5,
        size,
        stretch,
        skylight: lighting::FULL_SKYLIGHT,
        blocklight: petramond_world::light::BlockLight6::DARK,
        submerged: false,
        look,
    }
}

fn build_solids(
    solids: &[crate::SolidParticleInstance],
    cam: Vec3,
    pixel_scale: f32,
) -> Vec<ParticleRow> {
    let (mut rows, mut scratch) = (Vec::new(), Vec::new());
    build_transparent_emitter_particles(
        &[],
        &[],
        solids,
        0.0,
        petramond_math::math::IVec3::ZERO,
        cam,
        pixel_scale,
        LightEnv::IDENTITY,
        1.0,
        false,
        &mut rows,
        &mut scratch,
    );
    rows
}

#[test]
fn specks_face_the_eye_from_every_side() {
    let at = WorldPos::new(0.0, 0.0, 0.0);
    for cam in [
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(-3.0, 1.0, 0.5),
        Vec3::new(0.0, 6.0, 0.0),
        Vec3::new(0.0, -6.0, 0.0),
    ] {
        let rows = build_solids(&[solid(at, 0.2, 1.0, true)], cam, f32::MAX);
        let row = rows[0];
        let (right, up) = (Vec3::from(row.right), Vec3::from(row.up));
        let normal = right.cross(up);
        assert!(
            normal.normalize().dot(cam.normalize()) > 0.99,
            "a speck seen from {cam} turns its back face (culled) to the eye"
        );
        assert!((right.length() - 0.1).abs() < 1e-4 && (up.length() - 0.1).abs() < 1e-4);
    }
}

#[test]
fn particles_under_the_pixel_floor_keep_their_coverage() {
    let pixel_scale = 800.0;
    let at = WorldPos::new(0.0, 0.0, 0.0);
    let cam = Vec3::new(0.0, 0.0, 40.0);
    let coverage = |r: &ParticleRow| r.alpha * r.half * r.half * r.stretch;
    for (size, stretch, speck) in [(0.01, 1.0, false), (0.02, 1.0, true), (0.01, 8.0, false)] {
        let near = build_solids(&[solid(at, size, stretch, speck)], cam, f32::MAX)[0];
        let held = build_solids(&[solid(at, size, stretch, speck)], cam, pixel_scale)[0];
        let px = held.half * 2.0 * pixel_scale / 40.0;
        assert!(px >= MIN_CUBE_PX - 1e-3, "held at {px} px");
        assert!(held.half > near.half && held.alpha < near.alpha);
        assert!((coverage(&held) - coverage(&near)).abs() < coverage(&near) * 1e-3);
        assert!(
            (held.half * held.stretch - near.half * near.stretch).abs() < 1e-5
                || held.stretch == 1.0,
            "a stretched particle keeps its length while it can"
        );
    }
}

fn build_emitters(
    emitters: &[ParticleEmitterInstance],
    trails: &[TrailPoint],
    time: f32,
) -> Vec<ParticleRow> {
    let (mut rows, mut scratch) = (Vec::new(), Vec::new());
    build_transparent_emitter_particles(
        emitters,
        trails,
        &[],
        time,
        petramond_math::math::IVec3::ZERO,
        Vec3::ZERO,
        f32::MAX,
        LightEnv::IDENTITY,
        1.0,
        false,
        &mut rows,
        &mut scratch,
    );
    rows
}

fn still_stream() -> ParticleEmitterInstance {
    let mut inst = emitter_inst();
    inst.emitter.rate = [20.0, 20.0];
    inst.emitter.lifetime = [1.0, 1.0];
    inst.emitter.velocity = [0.0, 0.0, 0.0];
    inst.emitter.fade_power = 0.25;
    inst.emitter.shrink_power = 0.25;
    inst
}

#[test]
fn a_moving_emitter_leaves_its_particles_where_they_were_born() {
    let mut inst = still_stream();
    let at = |x: f64| WorldPos::new(x, 2.0, 3.0);
    let trails = [
        TrailPoint {
            time: 9.0,
            origin: at(0.0),
        },
        TrailPoint {
            time: 10.2,
            origin: at(0.0),
        },
        TrailPoint {
            time: 10.4,
            origin: at(5.0),
        },
        TrailPoint {
            time: 10.9,
            origin: at(5.0),
        },
    ];
    inst.origin = at(5.0);
    inst.trail = petramond::world::TrailSpan { start: 0, len: 4 };
    let rows = build_emitters(&[inst], &trails, 10.9);
    let xs: Vec<f32> = rows.iter().map(|r| r.center[0]).collect();
    assert!(
        xs.iter().any(|&x| x < 0.1),
        "born before the move: left behind {xs:?}"
    );
    assert!(
        xs.iter().any(|&x| x > 4.9),
        "born after it: at the new spot {xs:?}"
    );
    assert!(xs.iter().all(|&x| (-0.01..=5.01).contains(&x)));

    inst.trail = petramond::world::TrailSpan::default();
    let still = build_emitters(&[inst], &[], 10.9);
    assert!(
        still.iter().all(|r| r.center[0] > 4.9),
        "no trail: all at the origin"
    );
}

#[test]
fn a_pulsing_emitter_only_births_in_its_open_window() {
    let mut inst = still_stream();
    inst.emitter.lifetime = [0.05, 0.05];
    let lit_share = |inst: &ParticleEmitterInstance| {
        let steps = 400;
        let lit = (0..steps)
            .filter(|i| !build_emitters(&[*inst], &[], 3.0 + *i as f32 * 0.025).is_empty())
            .count();
        lit as f32 / steps as f32
    };
    let streaming = lit_share(&inst);
    inst.emitter.pulse = [1.0, 0.25, 0.25];
    let pulsing = lit_share(&inst);
    assert!(
        streaming > 0.6,
        "a stream is nearly always alive ({streaming})"
    );
    assert!(
        (0.1..0.45).contains(&pulsing),
        "a quarter-open pulse is dark most of each period ({pulsing})"
    );
}

#[test]
fn particles_past_the_ceiling_are_not_drawn() {
    let mut inst = still_stream();
    inst.emitter.velocity = [0.0, 2.0, 0.0];
    inst.emitter.lifetime = [3.0, 3.0];
    inst.ceiling_y = inst.origin.y as f32 + 1.5;
    let rows = build_emitters(&[inst], &[], 20.0);
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r.center[1] <= inst.ceiling_y));
    inst.ceiling_y = f32::INFINITY;
    let open = build_emitters(&[inst], &[], 20.0);
    assert!(open
        .iter()
        .any(|r| r.center[1] > inst.origin.y as f32 + 1.5));
}
