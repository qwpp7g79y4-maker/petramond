use super::*;
use crate::block::Block;
use crate::chunk::Chunk;
use crate::world::test_world::TestWorld;

fn world() -> TestWorld {
    TestWorld::with_chunk(0, Chunk::new(0, 0))
}

#[test]
fn pane_connects_to_full_cubes_and_panes_but_not_tagged_irregulars() {
    let mut w = world();
    let p = IVec3::new(8, 64, 8);
    assert!(w.set_block_world(p.x, p.y, p.z, Block::GlassPane));
    assert_eq!(w.data.pane_mask_at(p), 0, "isolated pane is a bare post");

    w.set_block_world(7, 64, 8, Block::Stone);
    w.set_block_world(9, 64, 8, Block::GlassPane);
    assert_eq!(
        w.data.pane_mask_at(p),
        crate::pane::WEST | crate::pane::EAST
    );

    w.set_block_world(8, 64, 7, Block::Chest);
    w.set_block_world(8, 64, 9, Block::Cactus);
    assert_eq!(
        w.data.pane_mask_at(p),
        crate::pane::WEST | crate::pane::EAST,
        "no_pane_connect blocks must not add arms"
    );
}
