fn model_bytes(kind: BlockModelKind) -> Vec<u8> {
    let file = def(kind).model_file;
    crate::assets::read_bytes(file)
        .unwrap_or_else(|| panic!("bbmodel '{file}' not found"))
        .0
}

use glam::Vec3;

use crate::asset_cache::CompiledAsset;
use crate::block::Aabb;
use petramond_math::face::Face;

use super::geometry::{face_slot, FLAT_FACE_BIAS};
use super::*;

const WB: BlockModelKind = BlockModelKind::FurnitureWorkbench;

#[test]
fn baking_nested_group_rotations_preserves_every_authored_corner() {
    use crate::bbmodel::{euler_quat, Model};
    use glam::Mat4;

    let source = serde_json::json!({
        "resolution": {"width": 16, "height": 16},
        "textures": [{"source": PIXEL_URI}],
        "groups": [
            {"uuid": "parent", "origin": [3, 5, -2], "rotation": [21, 37, -16]},
            {"uuid": "child", "origin": [-1, 4, 2], "rotation": [-13, 72, 9]}
        ],
        "elements": [{
            "uuid": "cube", "type": "cube", "from": [2, 3, 4], "to": [6, 8, 9],
            "origin": [1, -2, 3], "rotation": [-25, 17, 40],
            "faces": {"up": {"uv": [0, 0, 16, 16], "texture": 0}}
        }],
        "outliner": [{"uuid": "parent", "children": [
            {"uuid": "child", "children": ["cube"]}
        ]}]
    })
    .to_string();
    let authored = Model::load(&source).unwrap();
    let compiled = BlockModel::compile(source.as_bytes()).unwrap();
    let original = &authored.cubes[0];
    let baked = &compiled.cubes[0];
    let transform = |origin, rotation| {
        Mat4::from_translation(origin)
            * Mat4::from_quat(euler_quat(rotation))
            * Mat4::from_translation(-origin)
    };
    let before =
        authored.rest_pose()[original.bone] * transform(original.origin, original.rotation);
    let after = transform(baked.origin, baked.rotation);
    for (a, b) in super::geometry::box_corners(original.from, original.to)
        .into_iter()
        .zip(super::geometry::box_corners(baked.from, baked.to))
    {
        let expected = before.transform_point3(a);
        let actual = after.transform_point3(b);
        assert!(
            expected.abs_diff_eq(actual, 1e-4),
            "{expected:?} != {actual:?}"
        );
    }
}

#[test]
fn every_registered_model_compiles_with_geometry_and_texture() {
    for &kind in all() {
        let m = BlockModel::compile(&model_bytes(kind))
            .unwrap_or_else(|e| panic!("{kind:?} fails to compile: {e}"));
        assert!(!m.cubes.is_empty(), "{kind:?} has no geometry");
        assert_eq!(
            m.texture_rgba.len(),
            (m.tex_w * m.tex_h * 4) as usize,
            "{kind:?} texture size mismatch"
        );
        assert!(m.tex_w > 0 && m.tex_h > 0, "{kind:?} has no texture");
    }
}

#[test]
fn flat_model_cubes_emit_one_biased_surface_face() {
    let cube = ModelCube {
        name: String::new(),
        from: Vec3::new(0.0, 0.5, 0.0),
        to: Vec3::new(1.0, 0.5, 1.0),
        origin: Vec3::ZERO,
        rotation: Vec3::ZERO,
        faces: [Some(crate::bbmodel::FaceUv::new([0.0, 0.0, 1.0, 1.0])); 6],
        cull: [None; 6],
    };
    let support = ModelCube {
        name: String::new(),
        from: Vec3::ZERO,
        to: Vec3::new(1.0, 0.5, 1.0),
        origin: Vec3::ZERO,
        rotation: Vec3::ZERO,
        faces: [Some(crate::bbmodel::FaceUv::new([0.0, 0.0, 1.0, 1.0])); 6],
        cull: [None; 6],
    };
    let all = [cube, support];
    assert_eq!(
        render_face_bias(&all[0], &all, Face::PosY),
        Some(Vec3::Y * FLAT_FACE_BIAS)
    );
    assert_eq!(render_face_bias(&all[0], &all, Face::NegY), None);
    assert_eq!(render_face_bias(&all[0], &all, Face::PosX), None);
    assert_eq!(render_face_bias(&all[0], &all, Face::PosZ), None);
}

#[test]
fn flat_model_cubes_bias_away_from_backing_surface() {
    let poster = ModelCube {
        name: String::new(),
        from: Vec3::new(0.0, 0.0, 0.5),
        to: Vec3::new(1.0, 1.0, 0.5),
        origin: Vec3::ZERO,
        rotation: Vec3::ZERO,
        faces: [Some(crate::bbmodel::FaceUv::new([0.0, 0.0, 1.0, 1.0])); 6],
        cull: [None; 6],
    };
    let backing = ModelCube {
        name: String::new(),
        from: Vec3::new(0.0, 0.0, 0.5),
        to: Vec3::new(1.0, 1.0, 0.75),
        origin: Vec3::ZERO,
        rotation: Vec3::ZERO,
        faces: [Some(crate::bbmodel::FaceUv::new([0.0, 0.0, 1.0, 1.0])); 6],
        cull: [None; 6],
    };
    let all = [poster, backing];
    assert_eq!(
        render_face_bias(&all[0], &all, Face::NegZ),
        Some(Vec3::NEG_Z * FLAT_FACE_BIAS)
    );
    assert_eq!(render_face_bias(&all[0], &all, Face::PosZ), None);
}

#[test]
fn unsupported_flat_model_cubes_fall_back_to_authored_positive_face() {
    let mut cube = ModelCube {
        name: String::new(),
        from: Vec3::new(0.0, 0.5, 0.0),
        to: Vec3::new(1.0, 0.5, 1.0),
        origin: Vec3::ZERO,
        rotation: Vec3::ZERO,
        faces: [Some(crate::bbmodel::FaceUv::new([0.0, 0.0, 1.0, 1.0])); 6],
        cull: [None; 6],
    };
    let all = [cube.clone()];
    assert_eq!(
        render_face_bias(&cube, &all, Face::PosY),
        Some(Vec3::Y * FLAT_FACE_BIAS)
    );
    cube.faces[face_slot(Face::PosY)] = None;
    let all = [cube.clone()];
    assert_eq!(
        render_face_bias(&cube, &all, Face::NegY),
        Some(Vec3::NEG_Y * FLAT_FACE_BIAS)
    );
}

#[test]
fn thick_model_cubes_emit_all_faces_without_bias() {
    let cube = ModelCube {
        name: String::new(),
        from: Vec3::ZERO,
        to: Vec3::ONE,
        origin: Vec3::ZERO,
        rotation: Vec3::ZERO,
        faces: [Some(crate::bbmodel::FaceUv::new([0.0, 0.0, 1.0, 1.0])); 6],
        cull: [None; 6],
    };

    for face in Face::ALL {
        assert_eq!(
            render_face_bias(&cube, std::slice::from_ref(&cube), face),
            Some(Vec3::ZERO)
        );
    }
}

#[test]
fn every_footprint_cell_is_covered_and_splits_the_cubes() {
    let inst = instance(WB);
    let total: usize = inst.cells.iter().map(|c| c.cubes.len()).sum();
    assert_eq!(
        total,
        inst.cubes.len(),
        "every cube assigned to exactly one cell"
    );
    for off in [[0, 0, 0], [1, 0, 0]] {
        let c = inst.cell(off).expect("floor cell present");
        assert!(!c.collision.is_empty(), "floor cell {off:?} collides");
    }
}

#[test]
fn cells_are_local_and_within_unit_bounds() {
    let inst = instance(WB);
    for c in &inst.cells {
        for b in &c.collision {
            for i in 0..3 {
                assert!(
                    b.min[i] >= -1e-3 && b.max[i] <= 1.0 + 1e-3,
                    "cell-local box"
                );
                assert!(b.max[i] > b.min[i]);
            }
        }
    }
}

#[test]
fn a_pass_through_part_keeps_its_visuals_but_loses_its_collision() {
    let solid = ModelCube {
        name: "solid".into(),
        from: Vec3::new(0.0, 0.0, 0.0),
        to: Vec3::new(16.0, 16.0, 16.0),
        origin: Vec3::ZERO,
        rotation: Vec3::ZERO,
        faces: [Some(crate::bbmodel::FaceUv::new([0.0, 0.0, 1.0, 1.0])); 6],
        cull: [None; 6],
    };
    let water = ModelCube {
        name: "water".into(),
        from: Vec3::new(2.0, 2.0, 2.0),
        to: Vec3::new(14.0, 14.0, 14.0),
        origin: Vec3::ZERO,
        rotation: Vec3::ZERO,
        faces: [Some(crate::bbmodel::FaceUv::new([0.0, 0.0, 1.0, 1.0])); 6],
        cull: [None; 6],
    };
    let mut model = BlockModel {
        cubes: vec![solid, water],
        texture_rgba: vec![0, 0, 0, 0],
        tex_w: 1,
        tex_h: 1,
        collision: Vec::new(),
        bounds: Aabb {
            min: [0.0; 3],
            max: [1.0; 3],
        },
        display: BlockDisplay::default(),
        display_pivot: [0.0, 8.0, 0.0],
    };
    model.rebake();
    assert_eq!(model.collision.len(), 2, "both cubes collide before hiding");
    let bounds_before = model.bounds;

    model.apply_part_roles(
        &[("water", PartRole::PassThrough)],
        PartRole::Visible,
        "test:trough_filled",
    );
    assert_eq!(
        model.collision.len(),
        1,
        "only the solid cube collides after hiding water"
    );
    assert_eq!(
        model.bounds, bounds_before,
        "bounds still hug the visible water"
    );
    assert_eq!(model.cubes.len(), 2, "water cube is still rendered");
}

#[test]
fn a_row_without_optional_parts_bakes_only_part_free_segments() {
    for &kind in all() {
        if !def(kind).parts.is_empty() {
            continue;
        }
        let inst = instance(kind);
        for cell in inst.cells.iter() {
            for facing in (0..4).map(crate::facing::Facing::from_u8) {
                let Some(tmpl) = inst.cell_template(cell.offset, facing) else {
                    continue;
                };
                let (mut covered_v, mut covered_i) = (0u32, 0u32);
                for seg in &tmpl.segments {
                    assert!(
                        seg.part.is_none(),
                        "{kind:?} declares no parts but baked a part-gated segment"
                    );
                    assert_eq!(
                        seg.run.vert_start, covered_v,
                        "{kind:?} cell {:?}: segments must be contiguous",
                        cell.offset
                    );
                    assert_eq!(seg.run.index_start, covered_i);
                    covered_v += seg.run.vert_len;
                    covered_i += seg.run.index_len;
                }
                assert_eq!(
                    covered_v as usize,
                    tmpl.verts.len(),
                    "{kind:?} cell {:?}: the segments must cover every vertex",
                    cell.offset
                );
                assert_eq!(covered_i as usize, tmpl.indices.len());
                assert!(
                    tmpl.indices
                        .iter()
                        .all(|&i| (i as usize) < tmpl.verts.len()),
                    "{kind:?} cell {:?}: index out of range",
                    cell.offset
                );
            }
        }
    }
}

#[test]
fn display_base_matrix_matches_blockbench_euler_convention() {
    let with_rot = |r: [f32; 3]| DisplayTransform {
        rotation: r,
        ..Default::default()
    };
    let close = |a: Vec3, b: Vec3| (a - b).length() < 1e-5;
    let m = with_rot([0.0, 90.0, 0.0]).base_matrix();
    assert!(close(m.transform_vector3(Vec3::X), -Vec3::Z));
    let m = with_rot([90.0, 0.0, 0.0]).base_matrix();
    assert!(close(m.transform_vector3(Vec3::Y), Vec3::Z));
    let m = with_rot([90.0, 90.0, 0.0]).base_matrix();
    assert!(close(m.transform_vector3(Vec3::X), Vec3::Y));
}

#[test]
fn display_base_matrix_rotates_about_the_authored_pivot() {
    let piv = Vec3::new(0.25, -0.5, 0.125);
    let t = DisplayTransform {
        rotation: [16.0, 14.0, 4.0],
        translation: [1.0, 2.0, 3.0],
        rotation_pivot: piv.to_array(),
        ..Default::default()
    };
    let moved = t.base_matrix().transform_point3(piv);
    let expected = piv + Vec3::new(1.0, 2.0, 3.0) / 16.0;
    assert!(
        (moved - expected).length() < 1e-5,
        "pivot must stay fixed under rotation (moved to {moved:?}, expected {expected:?})"
    );
}

#[test]
fn display_from_unit_is_an_unmirrored_uniform_rescale() {
    let m = instance(WB).display_from_unit;
    let (x, y, z) = (
        m.transform_vector3(Vec3::X),
        m.transform_vector3(Vec3::Y),
        m.transform_vector3(Vec3::Z),
    );
    for (v, axis) in [(x, Vec3::X), (y, Vec3::Y), (z, Vec3::Z)] {
        let k = v.dot(axis);
        assert!(k > 0.0, "axis {axis:?} must keep its direction, got {v:?}");
        assert!(
            (v - axis * k).length() < 1e-6,
            "axis {axis:?} must not rotate, got {v:?}"
        );
    }
    assert!(
        (x.length() - y.length()).abs() < 1e-6 && (y.length() - z.length()).abs() < 1e-6,
        "rescale must be uniform"
    );
}

const PIXEL_URI: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";

#[test]
fn compiled_block_model_layout_change_requires_a_format_version_bump() {
    const PINNED_VERSION: u32 = 15;
    const COMPILED_HEX: &str = "01000000000000000400000000000000626f647900000000000000000000000000008041000080400000804100000000000000000000000000000000000000000000000000000100000000000000000000803f0000803f000000000000000000000400000000000000ff0000ff010000000100000001000000000000000000000000000000000000000000804100008040000080410000000000000000000000000000804100008040000080410000000000007041000000000000000000000000000000000000803f0000803f0000803f000000000000000000000000000000000000000000000000000000f0410000344200000000000000000000803f000000009a99193f9a99193f9a99193f0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000803f0000803f0000803f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000803f0000803f0000803f0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000803f0000803f0000803f000000000000000000000000000000000000000000000000000000000000004100000000";

    const SRC: &str = r##"{
        "resolution": { "width": 16, "height": 16 },
        "textures": [{ "uv_width": 16, "uv_height": 16, "source": "URI" }],
        "elements": [
            { "uuid": "c", "type": "cube", "name": "body", "from": [0,0,0], "to": [16,4,16],
              "faces": { "up": { "uv": [0,0,16,16], "texture": 0 } } }
        ],
        "outliner": ["c"],
        "display": {
            "gui": { "rotation": [30, 45, 0], "translation": [0, 1, 0], "scale": [0.6, 0.6, 0.6] },
            "firstperson_righthand": { "rotation": [0, 15, 0] }
        }
    }"##;
    let m = BlockModel::compile(
        SRC.replace("\"URI\"", &format!("\"{PIXEL_URI}\""))
            .as_bytes(),
    )
    .expect("canonical block model compiles");
    let bytes = bincode::serialize(&m).expect("serializes");
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert!(
        BlockModel::FORMAT_VERSION == PINNED_VERSION && hex == COMPILED_HEX,
        "compiled .llblock layout or version changed.\n\
         FORMAT_VERSION: {} (pinned {PINNED_VERSION})\n\
         serialized canonical model:\n{hex}\n\
         If any serialized struct or the compile output changed, bump FORMAT_VERSION \
         in `impl CompiledAsset for BlockModel` and update PINNED_VERSION + COMPILED_HEX \
         together. A layout change WITHOUT the bump lets stale caches mis-decode into \
         garbage models (the invisible-hushjaw class of bug).",
        BlockModel::FORMAT_VERSION,
    );
}

#[test]
fn a_hitbox_part_picks_by_bounds_where_bare_geometry_defers_to_its_texels() {
    let cube = |name: &str| ModelCube {
        name: name.into(),
        from: Vec3::new(0.0, 0.0, 0.0),
        to: Vec3::new(1.0, 0.125, 1.0),
        origin: Vec3::ZERO,
        rotation: Vec3::ZERO,
        faces: [None; 6],
        cull: [None; 6],
    };
    let roles: &[(&str, PartRole)] = &[("hit", PartRole::Hitbox)];
    let pick = |c: ModelCube| {
        super::query::ray_vs_model_cubes(
            Vec3::new(0.5, 2.0, 0.5),
            Vec3::new(0.0, -1.0, 0.0),
            std::slice::from_ref(&c),
            |cube, face, mn, mx, hit| {
                super::query::pick_face_solid(
                    |name| part_role(roles, PartRole::Visible, name).picks_by_bounds(),
                    cube,
                    face,
                    mn,
                    mx,
                    hit,
                    |_, _, _, _, _| false,
                )
            },
        )
    };
    let t = pick(cube("hit")).expect("a hitbox is aimed at by its bounds");
    assert!((t - 1.875).abs() < 1e-4, "first crossing at its top: {t}");
    assert!(
        pick(cube("art")).is_none(),
        "a cube whose texels are all transparent is not a pick target"
    );
}

/// Two cubes whose faces share a plane flicker between their textures. Every registered row is
/// checked as it draws its model: its own part roles, offsets and optional parts.
#[test]
fn every_model_row_keeps_its_faces_apart() {
    use crate::bbmodel::{Model, Shown};
    let mut fights = Vec::new();
    for &kind in all() {
        let d = def(kind);
        let src = String::from_utf8(model_bytes(kind)).expect("utf-8 model");
        let mut model = Model::load(&src).unwrap_or_else(|e| panic!("{}: {e}", d.key));
        for (name, off) in d.part_offsets {
            for c in model.cubes.iter_mut().filter(|c| c.name == *name) {
                let off = Vec3::from_array(*off);
                (c.from, c.to, c.origin) = (c.from + off, c.to + off, c.origin + off);
            }
        }
        let shown = |name: &str| match d.parts.iter().position(|p| *p == name) {
            Some(i) => Shown::Part(i),
            None if d.part_role(name).draws() => Shown::Always,
            None => Shown::Never,
        };
        for f in model.coplanar_faces_where(shown) {
            fights.push(format!(
                "{} ({}): {} {:?} / {} {:?} over {:.2}",
                d.key, d.model_file, f.a.0, f.a.1, f.b.0, f.b.1, f.area
            ));
        }
    }
    assert!(fights.is_empty(), "coplanar faces:\n{}", fights.join("\n"));
}
