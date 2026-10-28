use super::*;
use crate::chunk::Chunk;
use crate::world::test_world::TestWorld;

fn world_with_chunk() -> TestWorld {
    TestWorld::with_chunk(1, Chunk::new(0, 0))
}

#[test]
fn grass_under_solid_dies_to_dirt() {
    let mut w = world_with_chunk();
    let p = IVec3::new(8, 70, 8);
    w.set_block_world(p.x, p.y, p.z, Block::Grass);
    w.set_block_world(p.x, p.y + 1, p.z, Block::Stone);
    COVER.random_tick(&mut w, p);
    assert_eq!(w.data.block_if_loaded(p.x, p.y, p.z), Some(Block::Dirt));
}

#[test]
fn grass_under_no_grass_decay_cover_survives() {
    let mut w = world_with_chunk();
    let p = IVec3::new(8, 70, 8);
    w.set_block_world(p.x, p.y, p.z, Block::Grass);
    w.set_block_world(p.x, p.y + 1, p.z, Block::OakLeaves);
    COVER.random_tick(&mut w, p);
    assert_eq!(w.data.block_if_loaded(p.x, p.y, p.z), Some(Block::Grass));
}

#[test]
fn flooded_grass_dies_to_dirt() {
    let mut w = world_with_chunk();
    let p = IVec3::new(8, 70, 8);
    w.set_block_world(p.x, p.y, p.z, Block::Grass);
    w.set_block_world(p.x, p.y + 1, p.z, Block::Water);
    COVER.random_tick(&mut w, p);
    assert_eq!(w.data.block_if_loaded(p.x, p.y, p.z), Some(Block::Dirt));
}

#[test]
fn a_cover_spreads_exactly_as_far_as_its_row_says() {
    let spread = COVERS.current().onto[Block::Dirt.id() as usize][0].1;
    let mut w = world_with_chunk();
    let p = IVec3::new(8, 70, 8);
    w.set_block_world(p.x + spread, p.y, p.z, Block::Grass);
    assert_eq!(
        cover_within(
            &w.data,
            p,
            &COVERS.current().onto[Block::Dirt.id() as usize]
        ),
        Some(Block::Grass)
    );
    w.set_block_world(p.x + spread, p.y, p.z, Block::Stone);
    w.set_block_world(p.x + spread + 1, p.y, p.z, Block::Grass);
    assert_eq!(
        cover_within(
            &w.data,
            p,
            &COVERS.current().onto[Block::Dirt.id() as usize]
        ),
        None
    );
}

#[test]
fn dirt_with_grass_in_range_greens_over() {
    let mut w = world_with_chunk();
    let p = IVec3::new(8, 70, 8);
    w.set_block_world(p.x, p.y, p.z, Block::Dirt);
    w.set_block_world(p.x + 1, p.y, p.z, Block::Grass);
    BARE.random_tick(&mut w, p);
    assert_eq!(w.data.block_if_loaded(p.x, p.y, p.z), Some(Block::Grass));
}

#[test]
fn covered_dirt_does_not_green_even_with_grass_in_range() {
    let mut w = world_with_chunk();
    let p = IVec3::new(8, 70, 8);
    w.set_block_world(p.x, p.y, p.z, Block::Dirt);
    w.set_block_world(p.x + 1, p.y, p.z, Block::Grass);
    w.set_block_world(p.x, p.y + 1, p.z, Block::Stone);
    BARE.random_tick(&mut w, p);
    assert_eq!(w.data.block_if_loaded(p.x, p.y, p.z), Some(Block::Dirt));
}

#[test]
fn submerged_dirt_does_not_green_even_with_grass_in_range() {
    let mut w = world_with_chunk();
    let p = IVec3::new(8, 70, 8);
    w.set_block_world(p.x, p.y, p.z, Block::Dirt);
    w.set_block_world(p.x + 1, p.y, p.z, Block::Grass);
    w.set_block_world(p.x, p.y + 1, p.z, Block::Water);
    BARE.random_tick(&mut w, p);
    assert_eq!(w.data.block_if_loaded(p.x, p.y, p.z), Some(Block::Dirt));
}
