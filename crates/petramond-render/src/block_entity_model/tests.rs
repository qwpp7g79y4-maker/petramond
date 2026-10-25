use super::*;
use petramond_math::facing::Facing;
use petramond_world::door::THICKNESS;

const FACINGS: [Facing; 4] = [Facing::North, Facing::South, Facing::West, Facing::East];

fn inst(block: Block, facing: Facing, variant: u8, open01: f32) -> BlockEntityInstance {
    BlockEntityInstance {
        pos: glam::IVec3::new(10, 64, -5),
        block,
        facing,
        variant,
        open01,
        skylight: super::super::lighting::FULL_SKYLIGHT,
        blocklight: petramond_world::light::BlockLight6::DARK,
    }
}

fn bake(instances: &[BlockEntityInstance]) -> (Vec<Vertex>, Vec<u32>) {
    let (mut v, mut i) = (Vec::new(), Vec::new());
    push_block_entities(instances, glam::IVec3::ZERO, &mut v, &mut i);
    (v, i)
}

fn span(v: &[Vertex], axis: usize) -> (f32, f32) {
    v.iter().fold((f32::MAX, f32::MIN), |(lo, hi), vert| {
        (lo.min(vert.pos[axis]), hi.max(vert.pos[axis]))
    })
}

fn extent(v: &[Vertex], axis: usize) -> f32 {
    let (lo, hi) = span(v, axis);
    hi - lo
}

#[test]
fn thin_edge_faces_carry_a_uv_slice_mode() {
    let (v, _) = bake(&[inst(Block::OakDoor, Facing::South, 0, 0.0)]);
    let slice = |face: usize| (v[face * 4].packed >> petramond_mesh::UV_MODE_SHIFT) & 0x3;
    assert_eq!(slice(0), 1, "PosX side edge crops U");
    assert_eq!(slice(1), 1, "NegX side edge crops U");
    assert_eq!(slice(2), 2, "PosY top edge crops V");
    assert_eq!(slice(3), 2, "NegY bottom edge crops V");
    assert_eq!(slice(4), 0, "PosZ front art is full-tile");
    assert_eq!(slice(5), 0, "NegZ back art is full-tile");
    let (chest, _) = bake(&[inst(Block::Chest, Facing::South, 0, 0.0)]);
    assert!(chest
        .iter()
        .all(|v| (v.packed >> petramond_mesh::UV_MODE_SHIFT) & 0x3 == 0));
}

#[test]
fn opening_lifts_a_trapdoor_off_the_floor_onto_an_edge() {
    let closed = bake(&[inst(Block::OakTrapdoor, Facing::South, 0, 0.0)]).0;
    let open = bake(&[inst(Block::OakTrapdoor, Facing::South, 0, 1.0)]).0;
    assert!(extent(&closed, 1) < 0.25, "closed: thin on Y");
    assert!(extent(&open, 1) > 0.9, "open: full height");
    assert!(extent(&open, 2) < 0.25, "open: thin on Z (on its edge)");
}

#[test]
fn a_swung_panel_keeps_to_its_own_cell() {
    let arc = (std::f32::consts::SQRT_2 - 1.0) * THICKNESS / 2.0 + 1e-4;
    for facing in FACINGS {
        for variant in [0, 1] {
            for step in 0..=4 {
                let open01 = step as f32 / 4.0;
                let slack = if step == 0 || step == 4 { 1e-4 } else { arc };
                let (v, _) = bake(&[inst(Block::OakTrapdoor, facing, variant, open01)]);
                for (axis, origin) in [(0, 10.0), (1, 64.0), (2, -5.0)] {
                    let (lo, hi) = span(&v, axis);
                    assert!(
                        lo >= origin - slack && hi <= origin + 1.0 + slack,
                        "{facing:?} variant={variant} open={step}/4: axis {axis} {lo}..{hi} \
                         escaped the cell at {origin}"
                    );
                }
            }
        }
    }
}
