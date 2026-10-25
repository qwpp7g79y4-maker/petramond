use super::*;
use mod_sdk::build::Heights;
use std::collections::{BTreeSet, VecDeque};

const SEA: i32 = 63;
const FROZEN: u8 = 40;
const SCALE: Scale = Scale {
    medium: 0.3,
    large: 0.2,
    height: 1.2,
};

struct Flat {
    floor: i32,
    biome: u8,
}

impl Terrain for Flat {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights> {
        Some(Heights::from_fn(min, max, |_, _| self.floor))
    }

    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>> {
        Some(vec![self.biome; columns.len()])
    }
}

fn plan_at(seed: u32, cell: [i32; 2], terrain: &Flat) -> Option<Plan> {
    let site = GRID.site(seed, cell)?;
    match derive(seed, SEA, site, &[(FROZEN, SCALE)], terrain) {
        Derived::Plan(plan) => Some(*plan),
        _ => None,
    }
}

/// Up to 24 plans from a wide square of sites, enough to cross both open water and ice fields.
fn some_icebergs(terrain: &Flat) -> Vec<Plan> {
    (0..40)
        .flat_map(|z| (0..40).map(move |x| [x, z]))
        .filter_map(|cell| plan_at(11, cell, terrain))
        .take(24)
        .collect()
}

#[test]
fn an_iceberg_floats_clear_of_the_floor_and_wears_snow_above_water() {
    let terrain = Flat {
        floor: SEA - 1 - 18,
        biome: FROZEN,
    };
    let plans = some_icebergs(&terrain);
    assert!(!plans.is_empty(), "frozen deep water grows icebergs");
    for plan in plans {
        let cells: BTreeSet<[i32; 3]> = plan.cells().map(|(c, _)| c).collect();
        assert!(cells.iter().any(|c| c[1] > SEA - 1), "breaks the surface");
        for (c, m) in plan.cells() {
            assert!(
                c[1] > terrain.floor + KEEL_CLEARANCE,
                "{c:?} grounds the keel"
            );
            let exposed = !cells.contains(&[c[0], c[1] + 1, c[2]]);
            if exposed && c[1] > SEA - 1 {
                assert_eq!(m.block.as_str(), "petramond:snow_block", "{c:?}");
            }
        }
    }
}

/// A berg and the pieces it shed are separate bodies, but every one of them floats: each holds
/// cells at the waterline, so none hangs in the air or sinks loose.
#[test]
fn every_piece_of_ice_floats_at_the_waterline() {
    let terrain = Flat {
        floor: SEA - 1 - 30,
        biome: FROZEN,
    };
    let plans = some_icebergs(&terrain);
    assert!(!plans.is_empty());
    let mut pieces = 0;
    for plan in plans {
        let mut left: BTreeSet<[i32; 3]> = plan.cells().map(|(c, _)| c).collect();
        while let Some(start) = left.pop_first() {
            pieces += 1;
            let mut body = vec![start];
            let mut queue = VecDeque::from([start]);
            while let Some([x, y, z]) = queue.pop_front() {
                for n in [
                    [x + 1, y, z],
                    [x - 1, y, z],
                    [x, y + 1, z],
                    [x, y - 1, z],
                    [x, y, z + 1],
                    [x, y, z - 1],
                ] {
                    if left.remove(&n) {
                        body.push(n);
                        queue.push_back(n);
                    }
                }
            }
            assert!(
                body.iter().any(|c| c[1] == SEA - 1),
                "a piece of {} cells never meets the waterline",
                body.len()
            );
        }
    }
    assert!(pieces > 24, "bergs shed pieces around them");
}

#[test]
fn shallow_or_unfrozen_seas_grow_none() {
    let shallow = Flat {
        floor: SEA - 1 - (MIN_DEPTH - 1),
        biome: FROZEN,
    };
    assert!(some_icebergs(&shallow).is_empty());
    let warm = Flat {
        floor: SEA - 30,
        biome: FROZEN + 1,
    };
    assert!(some_icebergs(&warm).is_empty());
}

#[test]
fn the_widest_iceberg_stays_inside_its_site_reach() {
    let terrain = Flat {
        floor: SEA - 1 - 40,
        biome: FROZEN,
    };
    let widest = Scale {
        large: 1.0,
        ..frozen_scales()
            .into_iter()
            .fold(SCALE, |a, b| if b.height > a.height { b } else { a })
    };
    for i in 0..400 {
        let Some(site) = GRID.site(5, [i % 20, i / 20]) else {
            continue;
        };
        let Derived::Plan(plan) = derive(5, SEA, site, &[(FROZEN, widest)], &terrain) else {
            continue;
        };
        for (c, _) in plan.cells() {
            let (dx, dz) = (c[0] - site.center[0], c[2] - site.center[1]);
            assert!(
                dx.abs() <= REACH && dz.abs() <= REACH,
                "{c:?} escapes {site:?}"
            );
        }
    }
}
