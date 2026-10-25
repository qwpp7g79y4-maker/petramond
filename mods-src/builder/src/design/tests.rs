use super::*;
use crate::testing::{Session, ASSET};

fn wall_around(lintel: bool) -> (Design, usize) {
    let mut design = Design::new([0; 32], [0, 0, 0], 0);
    let block = |footprint| Plan::Build {
        footprint,
        attachment: false,
        late: false,
        fragile: false,
        glazing: false,
    };
    design.plans = vec![block(vec![[0, 0, 0]]), block(vec![[0, 0, 0], [0, 1, 0]])];
    for x in 0..5 {
        for y in -1..3 {
            let door = x == 2 && (0..2).contains(&y);
            if door || (x == 2 && y == 2 && !lintel) {
                continue;
            }
            design.units.push(Unit {
                pos: [x, y, 0],
                record: 0,
            });
        }
    }
    design.units.push(Unit {
        pos: [2, 0, 0],
        record: 1,
    });
    for unit in design.units.clone() {
        for cell in design.cells(unit) {
            design.filled.insert(cell);
        }
    }
    design.finish();
    let door = design.unit_at([2, 0, 0]).unwrap();
    (design, door)
}

#[test]
fn a_house_keeps_its_rooms_and_the_space_over_its_hedge_empty() {
    let mut design = Design::new([0; 32], [0, 0, 0], 0);
    design.max = [6, 3, 4];
    for x in 0..5 {
        for y in 0..4 {
            for z in 0..5 {
                let shell = x == 0 || x == 4 || y == 0 || y == 3 || z == 0 || z == 4;
                if shell {
                    design.filled.insert([x, y, z]);
                }
            }
        }
    }
    design.filled.insert([5, 0, 2]);
    design.filled.insert([5, 3, 2]);
    design.governed = design.filled.clone();
    let rooms: HashSet<[i32; 3]> = design.rooms().into_iter().collect();
    assert!(rooms.contains(&[2, 1, 2]), "the room inside the walls");
    assert!(rooms.contains(&[5, 1, 2]), "between the hedge and the eave");
    assert!(
        !rooms.contains(&[5, 1, 1]),
        "beside the hedge, under nothing"
    );
    assert!(
        !rooms.contains(&[6, 1, 2]),
        "open ground the design builds nothing on"
    );
    assert!(rooms.iter().all(|c| !design.filled.contains(c)));
}

#[test]
fn a_tall_object_framed_by_wall_is_a_way_in() {
    let (design, door) = wall_around(true);
    assert!(design.passage(door));
    assert_eq!(
        (0..design.units.len())
            .filter(|i| design.passage(*i))
            .count(),
        1
    );
    let (design, door) = wall_around(false);
    assert!(!design.passage(door), "no lintel: an object on a wall top");
}

const STRIP: [([i32; 3], &str); 5] = [
    ([0, 0, 0], "petramond:glass_pane"),
    ([1, 0, 0], "petramond:stone"),
    ([2, 0, 0], "petramond:oak_leaves"),
    ([0, 1, 0], "petramond:stone"),
    ([1, 1, 0], "petramond:torch"),
];

fn strip(per: usize) -> Session {
    let session = Session::flat(8);
    session
        .world
        .schematic(ASSET, "Strip", [3, 2, 1], &STRIP, per);
    session
}

fn compiled(turns: u8) -> Design {
    let _session = strip(64);
    let mut design = Design::new(ASSET, [10, 0, 5], turns);
    assert!(matches!(design.compile(64), Progress::Ready));
    design
}

#[test]
fn units_go_bottom_up_whole_blocks_before_what_hangs_on_them() {
    let design = compiled(0);
    let order: Vec<[i32; 3]> = design.units.iter().map(|u| u.pos).collect();
    assert_eq!(
        order,
        vec![
            [11, 0, 5],
            [10, 0, 5],
            [12, 0, 5],
            [12, 1, 5],
            [10, 1, 5],
            [11, 1, 5],
        ]
    );
    let at = |pos| design.units[design.unit_at(pos).expect("a unit there")];
    assert!(matches!(design.plan(at([12, 1, 5])), Plan::Air));
    assert!(design.glazing(design.unit_at([10, 0, 5]).unwrap()));
    assert!(!design.glazing(design.unit_at([11, 0, 5]).unwrap()));
    assert!(design.late(at([12, 0, 5])));
    assert!(design.fragile(at([11, 1, 5])));
    let cost = design.cost(at([11, 0, 5]));
    assert_eq!((cost.len(), cost[0].item.as_str()), (1, "petramond:stone"));
    assert!(design.cost(at([12, 1, 5])).is_empty());
    assert!(
        !design.governed.contains(&[12, 1, 5]),
        "a room is somewhere to stand"
    );
    assert!((0..design.units.len()).all(|i| !design.passage(i)));
}

#[test]
fn a_turned_design_stays_in_its_turned_box() {
    let design = compiled(1);
    assert_eq!((design.min, design.max), ([10, 0, 5], [10, 1, 7]));
    let inside = |c: &&[i32; 3]| (0..3).all(|i| (design.min[i]..=design.max[i]).contains(&c[i]));
    assert_eq!(design.governed.iter().filter(inside).count(), 5);
    assert_eq!(design.governed.len(), 5);
}
