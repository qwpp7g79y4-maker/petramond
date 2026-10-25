use crate::block::Block;
use crate::fluid_math::{fluid_height, FALLING};
use crate::mathh::IVec3;
use crate::world::test_world::TestWorld;
use petramond_math::world_pos::WorldPos;

const MARGIN: f32 = 0.1;

fn eye_at(p: IVec3, y: f32) -> WorldPos {
    WorldPos::new(p.x as f64 + 0.5, f64::from(y), p.z as f64 + 0.5)
}

fn inside(w: &TestWorld, p: IVec3, y: f32) -> bool {
    super::eye_inside(&w.data.cursor(), eye_at(p, y), Block::Water, Some(MARGIN))
}

#[test]
fn the_eye_follows_a_flowing_surface_height() {
    let mut w = TestWorld::new(1);
    let p = IVec3::new(4, 64, 4);
    w.set_fluid(p, Block::Water, 7);
    let surface = p.y as f32 + fluid_height(7, None, Block::Water);
    assert!(!inside(&w, p, p.y as f32 + 0.5));
    assert!(inside(&w, p, surface - MARGIN - 0.01));
}

#[test]
fn the_eye_waits_until_the_margin_below_an_open_surface() {
    let mut w = TestWorld::new(1);
    let p = IVec3::new(5, 64, 5);
    w.set_fluid(p, Block::Water, 0);
    let surface = p.y as f32 + fluid_height(0, None, Block::Water);
    assert!(!inside(&w, p, surface + 0.01));
    assert!(!inside(&w, p, surface - MARGIN * 0.5));
    assert!(inside(&w, p, surface - MARGIN - 0.01));
    assert!(
        super::eye_inside(
            &w.data.cursor(),
            eye_at(p, surface - MARGIN * 0.5),
            Block::Water,
            None
        ),
        "a medium without a margin counts any eye in its cell"
    );
}

#[test]
fn a_capped_or_falling_cell_is_inside_up_to_its_top() {
    let mut w = TestWorld::new(1);
    let capped = IVec3::new(6, 64, 6);
    w.set_fluid(capped, Block::Water, 0);
    w.set_fluid(capped + IVec3::Y, Block::Water, 0);
    assert!(inside(&w, capped, capped.y as f32 + 0.99));

    let falling = IVec3::new(7, 64, 7);
    w.set_fluid(falling, Block::Water, FALLING);
    assert!(inside(&w, falling, falling.y as f32 + 1.0 - MARGIN - 0.01));
}

#[test]
fn eye_fluid_names_the_enclosing_fluid_and_air_names_none() {
    let mut w = TestWorld::new(1);
    let p = IVec3::new(8, 64, 8);
    w.set_fluid(p, Block::Water, 0);
    w.set_fluid(p + IVec3::Y, Block::Water, 0);
    assert_eq!(
        w.data.eye_fluid(eye_at(p, p.y as f32 + 0.5)),
        Some(Block::Water)
    );
    assert_eq!(w.data.eye_fluid(eye_at(p, p.y as f32 + 2.5)), None);
}

#[test]
fn the_eye_finds_the_surface_it_is_in_or_just_over() {
    let mut w = TestWorld::new(1);
    let top = IVec3::new(9, 64, 9);
    for y in 0..3 {
        w.set_fluid(top - IVec3::Y * y, Block::Water, 0);
    }
    let surface = top.y as f32 + fluid_height(0, None, Block::Water);
    for y in [
        surface + 0.6,
        surface + 0.02,
        surface - 0.02,
        top.y as f32 - 0.5,
    ] {
        assert_eq!(
            w.data.eye_surface(eye_at(top, y)),
            Some((Block::Water, surface)),
            "an eye at {y}"
        );
    }
    assert_eq!(
        w.data.eye_surface(eye_at(top, top.y as f32 - 1.5)),
        None,
        "more than a cell of water over the eye's own"
    );
    let lid = top + IVec3::Y;
    w.set_block_world(lid.x, lid.y, lid.z, Block::Stone);
    assert_eq!(
        w.data.eye_surface(eye_at(top, surface - 0.02)),
        None,
        "water under rock is no surface"
    );
}

#[test]
fn a_rise_leaves_the_fluid_at_its_open_surface() {
    let mut w = TestWorld::new(1);
    let base = IVec3::new(4, 60, 4);
    for dy in 0..4 {
        w.set_fluid(base + IVec3::new(0, dy, 0), Block::Water, 0);
    }
    let top = base.y + 3;
    let surface = top as f32 + fluid_height(0, None, Block::Water);
    let from = eye_at(base, base.y as f32 + 0.2);
    assert!((w.data.fluid_ceiling(from, 8.0) - surface).abs() < 1e-4);
    assert_eq!(
        w.data.fluid_ceiling(from, 1.0),
        f32::INFINITY,
        "too deep to reach"
    );
    let dry = eye_at(base + IVec3::new(0, 6, 0), 66.5);
    assert_eq!(
        w.data.fluid_ceiling(dry, 8.0),
        66.5,
        "nothing outlives dry air"
    );
}
