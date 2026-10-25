use super::*;
use petramond_math::world_pos::WorldPos;
use rustc_hash::FxHashSet;

struct Stub {
    floor_y: i32,
    solid: FxHashSet<(i32, i32, i32)>,
}
impl Stub {
    fn new(floor_y: i32) -> Self {
        Stub {
            floor_y,
            solid: FxHashSet::default(),
        }
    }
    fn add(&mut self, c: IVec3) {
        self.solid.insert((c.x, c.y, c.z));
    }
    fn solid_at(&self, c: IVec3) -> bool {
        c.y < self.floor_y || self.solid.contains(&(c.x, c.y, c.z))
    }
}

fn params() -> PathParams {
    PathParams::default()
}

#[test]
fn takes_diagonals_over_open_ground() {
    let w = Stub::new(1);
    let start = IVec3::new(0, 1, 0);
    let goal = IVec3::new(3, 1, 3);
    let path = find_path(start, goal, params(), |c| w.solid_at(c), |_| false);
    assert_eq!(path.first(), Some(&start));
    assert_eq!(path.last(), Some(&goal));
    assert_eq!(
        path.len(),
        4,
        "open ground should route diagonally: {path:?}"
    );
    assert!(
        path.windows(2)
            .any(|w| (w[1].x - w[0].x).abs() == 1 && (w[1].z - w[0].z).abs() == 1),
        "expected a diagonal step: {path:?}"
    );
}

#[test]
fn does_not_cut_an_obstacle_corner() {
    let mut w = Stub::new(1);
    w.add(IVec3::new(1, 1, 0));
    w.add(IVec3::new(1, 2, 0));
    let start = IVec3::new(0, 1, 0);
    let goal = IVec3::new(1, 1, 1);
    let path = find_path(start, goal, params(), |c| w.solid_at(c), |_| false);
    assert_eq!(path.last(), Some(&goal));
    assert!(
        !path.windows(2).any(|w| w[0] == start && w[1] == goal),
        "must not cut the obstacle corner: {path:?}"
    );
    for w2 in path.windows(2) {
        let (a, b) = (w2[0], w2[1]);
        if (b.x - a.x).abs() == 1 && (b.z - a.z).abs() == 1 {
            assert!(!w.solid_at(IVec3::new(b.x, a.y, a.z)), "corner cut via X");
            assert!(!w.solid_at(IVec3::new(a.x, a.y, b.z)), "corner cut via Z");
        }
    }
}

#[test]
fn steps_up_a_one_block_rise() {
    let mut w = Stub::new(1);
    for x in 2..=4 {
        w.add(IVec3::new(x, 1, 0));
    }
    let start = IVec3::new(0, 1, 0);
    let goal = IVec3::new(4, 2, 0);
    let path = find_path(start, goal, params(), |c| w.solid_at(c), |_| false);
    assert_eq!(path.last(), Some(&goal), "reaches the raised goal");
    for w2 in path.windows(2) {
        assert!(w2[1].y - w2[0].y <= 1, "no move climbs more than one block");
    }
    assert!(path.iter().any(|c| c.y == 2), "climbs onto the step");
}

#[test]
fn a_two_high_wall_cannot_be_climbed() {
    let solid = |c: IVec3| c.y < 1 || (c.x == 2 && (c.y == 1 || c.y == 2));
    let start = IVec3::new(0, 1, 0);
    let goal = IVec3::new(4, 1, 0);
    let path = find_path(start, goal, params(), solid, |_| false);
    assert!(
        path.last().unwrap().x < 2,
        "must not climb a 2-high wall: {:?}",
        path.last()
    );
}

#[test]
fn descends_a_ledge_within_the_drop_limit() {
    let mut w = Stub::new(1);
    for x in -1..=1 {
        for y in 1..4 {
            w.add(IVec3::new(x, y, 0));
        }
    }
    let start = IVec3::new(0, 4, 0);
    let goal = IVec3::new(3, 1, 0);
    let path = find_path(start, goal, params(), |c| w.solid_at(c), |_| false);
    assert_eq!(
        path.last(),
        Some(&goal),
        "walks off the ledge to the low ground"
    );
    assert!(
        path.windows(2).any(|w2| w2[0].y - w2[1].y == 3),
        "single 3-block descent: {path:?}"
    );
}

#[test]
fn avoids_a_drop_greater_than_the_limit() {
    let solid = |c: IVec3| if c.x >= 2 { c.y < 1 } else { c.y < 5 };
    let start = IVec3::new(0, 5, 0);
    let goal = IVec3::new(3, 1, 0);
    let path = find_path(start, goal, params(), solid, |_| false);
    assert!(
        path.last().unwrap().x < 2,
        "must not take a 4-block drop: {:?}",
        path.last()
    );
}

fn deep_trench_solid(c: IVec3) -> bool {
    if (1..=3).contains(&c.x) {
        c.y <= -6
    } else {
        c.y <= 0
    }
}

#[test]
fn crosses_deep_fluid_at_the_surface() {
    let fluid = |c: IVec3| (1..=3).contains(&c.x) && (-5..=0).contains(&c.y);
    let start = IVec3::new(0, 1, 0);
    let goal = IVec3::new(4, 1, 0);
    let path = find_path(start, goal, params(), deep_trench_solid, fluid);
    assert_eq!(path.first(), Some(&start));
    assert_eq!(
        path.last(),
        Some(&goal),
        "reaches the far shore across the fluid"
    );
    assert!(
        path.iter().all(|c| c.y == 1),
        "crosses at the surface level: {path:?}"
    );
}

#[test]
fn swimming_cell_snaps_to_the_fluid_surface() {
    let solid = |c: IVec3| c.y <= -1;
    let fluid = |c: IVec3| (0..=3).contains(&c.y);
    let cell = swimming_cell(WorldPos::new(0.5, 1.2, 0.5), 0.25, 1, &solid, &fluid);
    assert_eq!(
        cell,
        Some(IVec3::new(0, 4, 0)),
        "a swimming mob paths from the surface, not its submerged feet cell"
    );
}

#[test]
fn a_wading_mob_in_one_deep_fluid_paths_from_the_fluid_surface_cell() {
    let solid = |c: IVec3| c.y < 0 || (c.x >= 3 && c.y == 0);
    let fluid = |c: IVec3| c.x <= 2 && c.y == 0;
    let wading = WorldPos::new(1.5, 0.3, 0.5);

    let feet = IVec3::new(1, 0, 0);
    assert!(
        find_path(feet, IVec3::new(4, 1, 0), params(), solid, fluid).is_empty(),
        "the submerged feet cell is rejected as a path start"
    );

    let cell = navigation_cell(wading, 0.25, 1, true, &solid, &fluid)
        .expect("a wading mob still has a navigation cell");
    assert_eq!(cell, IVec3::new(1, 1, 0), "starts at the fluid surface");
    let path = find_path(cell, IVec3::new(4, 1, 0), params(), solid, fluid);
    assert_eq!(
        path.last(),
        Some(&IVec3::new(4, 1, 0)),
        "the shore is reachable from the surface cell: {path:?}"
    );
}

#[test]
fn unreachable_goal_returns_closest_partial_path() {
    let w = Stub::new(1);
    let start = IVec3::new(0, 1, 0);
    let goal = IVec3::new(20, 50, 0);
    let path = find_path(start, goal, params(), |c| w.solid_at(c), |_| false);
    assert_eq!(path.first(), Some(&start));
    assert!(path.len() > 1, "makes progress toward the goal");
    assert!(path.last().unwrap().x > 0);
    assert!(path.last().unwrap().y < 50);
}

#[test]
fn standing_cell_finds_the_block_under_an_overhanging_centre() {
    let solid = |c: IVec3| c == IVec3::new(0, 0, 0);
    let cell = standing_cell(WorldPos::new(1.1, 1.0, 0.5), 0.25, 1, &solid);
    assert_eq!(
        cell,
        Some(IVec3::new(0, 1, 0)),
        "edge overhang resolves to the block"
    );
    let on = standing_cell(WorldPos::new(0.5, 1.0, 0.5), 0.25, 1, &solid);
    assert_eq!(on, Some(IVec3::new(0, 1, 0)));
    let off = standing_cell(WorldPos::new(5.0, 1.0, 5.0), 0.25, 1, &solid);
    assert_eq!(off, None);
}

#[test]
fn standing_on_a_partial_solid_resolves_to_the_cell_above_it() {
    let solid = |c: IVec3| c.y == 0 || c == IVec3::new(0, 1, 0);
    let on_bed = standing_cell(WorldPos::new(0.5, 1.56, 0.5), 0.25, 1, &solid);
    assert_eq!(on_bed, Some(IVec3::new(0, 2, 0)));

    let pillar = |c: IVec3| c == IVec3::new(0, 0, 0) || c == IVec3::new(0, 1, 0);
    let edge = standing_cell(WorldPos::new(1.1, 1.56, 0.5), 0.25, 1, &pillar);
    assert_eq!(edge, Some(IVec3::new(0, 2, 0)));
}

#[test]
fn start_equals_goal_is_a_singleton() {
    let w = Stub::new(1);
    let c = IVec3::new(5, 1, 5);
    assert_eq!(
        find_path(c, c, params(), |p| w.solid_at(p), |_| false),
        vec![c]
    );
}

#[test]
fn wide_body_does_not_fit_through_a_one_cell_gap() {
    let solid = |c: IVec3| c.y < 1 || (c.x == 2 && (c.y == 1 || c.y == 2) && c.z != 0);
    let start = IVec3::new(0, 1, 0);
    let goal = IVec3::new(4, 1, 0);
    let path = find_path(start, goal, PathParams::for_body(1, 0.55), solid, |_| false);
    assert!(
        path.last().unwrap().x < 2,
        "wide body must not squeeze through the one-cell gap: {path:?}"
    );
    assert!(
        !path.contains(&IVec3::new(2, 1, 0)),
        "the gap cell itself is not a valid wide-body foothold: {path:?}"
    );
}
