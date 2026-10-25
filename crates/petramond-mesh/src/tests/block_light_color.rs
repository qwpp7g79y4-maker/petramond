use super::*;
use crate::vertex::decode_vertex_light;
use petramond_world::light::{BlockLight6, LightRgb};

fn mesh_lit_floor(block_light: impl Fn(i32, i32, i32) -> LightRgb) -> ChunkMesh {
    let section = floor_section(Block::Stone);
    build_section_mesh(
        &section,
        SectionPos::new(0, 0, 0),
        test_ctx(),
        &crate::WorldReads {
            block: &|wx, wy, wz| {
                if in_section(wx, wy, wz) {
                    section.block_raw(wx as usize, wy as usize, wz as usize)
                } else {
                    Block::Air.id()
                }
            },
            cell_state: &|_, _, _| petramond_world::block::ShapeState::NONE,
            fluid_meta: &|_, _, _| 0,
            held_fluid: &|_, _, _| 0,
            biome: &|_, _| 0,
            skylight: &|_, _, _| 0,
            blocklight: &block_light,
            loaded: &|_, _, _| true,
            dyed: &|_, _, _| false,
        },
    )
}

fn top_lights_at_x(mesh: &ChunkMesh, x: f32) -> Vec<BlockLight6> {
    mesh.opaque
        .iter()
        .filter(|v| (v.pos[1] - 1.0).abs() < 1e-3 && (v.pos[0] - x).abs() < 1e-3)
        .map(decode_vertex_light)
        .collect()
}

fn floor_top(mesh: &ChunkMesh) -> Vec<&Vertex> {
    mesh.opaque
        .iter()
        .filter(|v| shade_idx(v) == 0 && (v.pos[1] - 1.0).abs() < 1e-3)
        .collect()
}

#[test]
fn a_saturated_block_light_reaches_the_vertex_with_its_hue() {
    let purple = LightRgb::new(12, 8, 30);
    let mesh = mesh_lit_floor(move |_, _, _| purple);
    let want = BlockLight6::from_x2(purple);
    assert!(
        want.r() != want.g() && want.g() != want.b(),
        "the fixture must actually be coloured"
    );
    let top = floor_top(&mesh);
    assert_eq!(
        top.len(),
        4,
        "a uniformly-lit floor must still merge to one quad — colour must not \
         block a merge, and this test must be reading the MERGED emitter's words"
    );
    for v in top {
        assert_eq!(decode_vertex_light(v), want);
    }
}

#[test]
fn a_face_between_two_colours_averages_the_channels_not_the_hue() {
    let purple = LightRgb::new(24, 4, 30);
    let green = LightRgb::new(4, 30, 8);
    let mesh = mesh_lit_floor(move |wx, _, _| if wx < 8 { purple } else { green });

    for c in top_lights_at_x(&mesh, 0.0) {
        assert_eq!(c, BlockLight6::from_x2(purple), "deep in the purple half");
    }
    for c in top_lights_at_x(&mesh, 16.0) {
        assert_eq!(c, BlockLight6::from_x2(green), "deep in the green half");
    }

    let seam = top_lights_at_x(&mesh, 8.0);
    assert!(!seam.is_empty(), "the seam column must be meshed");
    let p6 = BlockLight6::from_x2(purple).channels();
    let g6 = BlockLight6::from_x2(green).channels();
    let mixed = seam
        .iter()
        .find(|c| {
            (0..3).all(|i| {
                let (lo, hi) = (p6[i].min(g6[i]), p6[i].max(g6[i]));
                c.channels()[i] > lo && c.channels()[i] < hi
            })
        })
        .unwrap_or_else(|| panic!("no blended corner on the seam: {seam:?}"));
    assert_ne!(*mixed, BlockLight6::grey(mixed.luminance()));
}
