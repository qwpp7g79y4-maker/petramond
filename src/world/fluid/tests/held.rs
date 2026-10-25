use super::*;
use crate::entity::fluid_fixture::{self, BRINE, FLOOR_Y, REED};

#[test]
fn blocks_that_hold_a_fluid_keep_it() {
    fluid_fixture::with_content("held-fluid", held_inner);
}

fn held_inner() {
    a_held_fluid_is_a_source();
    a_block_written_into_a_source_holds_it();
    a_held_fluid_is_left_behind_and_only_taken_from_a_source();
    a_stacking_plant_stands_on_its_own_kind();
}

fn reed() -> Block {
    fluid_fixture::block(REED)
}

fn brine() -> Block {
    fluid_fixture::block(BRINE)
}

fn delay() -> u32 {
    brine().fluid_def().unwrap().delay as u32 + 2
}

/// The fluid a row contains and the fluid a block holds both feed the open cells beside them,
/// and flow never washes either away. Every cell the sim checks keeps its read reach inside the
/// fixture's one chunk.
fn a_held_fluid_is_a_source() {
    let mut w = fluid_fixture::pool(Block::Air, FLOOR_Y - 1);
    let reed_at = IVec3::new(6, FLOOR_Y, 7);
    w.set_block_world(reed_at.x, reed_at.y, reed_at.z, reed());
    let fence_at = IVec3::new(9, FLOOR_Y, 8);
    w.set_block_world(fence_at.x, fence_at.y, fence_at.z, Block::OakFence);
    assert!(w.set_held_fluid(fence_at, Some(brine())));
    run_ticks(&mut w, delay() * 4);
    for (at, host) in [(reed_at, reed()), (fence_at, Block::OakFence)] {
        assert_eq!(
            block(&w, at.x, at.y, at.z),
            host,
            "flow never washes it away"
        );
        for d in CARDINALS {
            let p = at + d;
            assert_eq!(
                w.data.cell_fluid(p.x, p.y, p.z),
                Some(brine()),
                "{host:?} feeds the cell toward {d}"
            );
        }
    }
}

fn a_block_written_into_a_source_holds_it() {
    let mut w = fluid_fixture::pool(brine(), FLOOR_Y + 1);
    let held = |w: &ServerWorld, p: IVec3| w.data.held_fluid(p.x, p.y, p.z);
    let at = IVec3::new(4, FLOOR_Y, 4);
    w.set_block_world(at.x, at.y, at.z, Block::OakSlab);
    assert_eq!(
        held(&w, at),
        Some(brine()),
        "a slab written into a source keeps it"
    );
    w.set_block_world(at.x, at.y, at.z, Block::OakFence);
    assert_eq!(
        held(&w, at),
        Some(brine()),
        "and hands it on to what replaces it"
    );
    w.set_block_world(at.x, at.y, at.z, Block::Stone);
    assert_eq!(
        w.data.cell_fluid(at.x, at.y, at.z),
        None,
        "a full block displaces it"
    );

    let flow = IVec3::new(8, FLOOR_Y + 1, 8);
    assert!(w.set_fluid_world(flow, brine(), flowing(2)));
    w.set_block_world(flow.x, flow.y, flow.z, Block::OakSlab);
    assert_eq!(held(&w, flow), None, "flowing fluid is no source to keep");

    let broken = IVec3::new(12, FLOOR_Y, 12);
    w.set_block_world(broken.x, broken.y, broken.z, Block::OakFence);
    assert!(w.clear_broken_block(broken).is_some());
    assert!(
        w.data.is_fluid_source_world(broken, brine()),
        "breaking it leaves its fluid"
    );

    let mut w = fluid_fixture::pool(Block::Air, FLOOR_Y - 1);
    let at = IVec3::new(4, FLOOR_Y, 4);
    w.set_block_world(at.x, at.y, at.z, Block::OakFence);
    assert!(w.set_held_fluid(at, Some(brine())));
    let feet = WorldPos::new(4.5, f64::from(FLOOR_Y), 4.5);
    assert!(
        w.data
            .body_fluid(feet, 1.8, petramond_world::fluid::Buoyancy::Swim)
            .is_some(),
        "a body standing in it is immersed"
    );
}

fn a_held_fluid_is_left_behind_and_only_taken_from_a_source() {
    let mut w = fluid_fixture::pool(brine(), FLOOR_Y + 1);
    let wet = IVec3::new(4, FLOOR_Y, 4);
    assert!(w
        .data
        .placement_support_ok(reed(), petramond_world::block::ShapeState::NONE, wet));
    let dry = IVec3::new(4, FLOOR_Y + 2, 4);
    assert!(
        !w.data
            .placement_support_ok(reed(), petramond_world::block::ShapeState::NONE, dry),
        "never conjured in open air"
    );
    assert!(w.set_fluid_world(wet + IVec3::X, brine(), flowing(2)));
    assert!(
        !w.data.placement_support_ok(
            reed(),
            petramond_world::block::ShapeState::NONE,
            wet + IVec3::X
        ),
        "flowing fluid is not a source to take"
    );
    assert!(!reed().transmits_direct_skylight());

    w.set_block_world(wet.x, wet.y, wet.z, reed());
    assert!(w.clear_broken_block(wet).is_some());
    assert!(
        w.data.is_fluid_source_world(wet, brine()),
        "breaking it leaves its fluid"
    );
}

fn a_stacking_plant_stands_on_its_own_kind() {
    let mut w = fluid_fixture::pool(brine(), FLOOR_Y + 3);
    let root = IVec3::new(8, FLOOR_Y, 8);
    for y in 0..3 {
        w.set_block_world(root.x, root.y + y, root.z, reed());
    }
    let top = root + IVec3::Y * 2;
    assert!(
        w.data.fragile_supported(top, reed()),
        "a segment holds the one above"
    );
    assert!(w.data.placement_support_ok(
        reed(),
        petramond_world::block::ShapeState::NONE,
        top + IVec3::Y
    ));
    w.set_block_world(root.x, root.y, root.z, Block::Air);
    assert!(
        !w.data.fragile_supported(root + IVec3::Y, reed()),
        "a stack still needs real ground under its foot"
    );
}
