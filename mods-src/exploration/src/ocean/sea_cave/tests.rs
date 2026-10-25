use super::*;
use std::collections::VecDeque;

const SEA: i32 = 63;
const OCEAN: u8 = 3;
const FLOOR: i32 = SEA - 12;

/// A flat sea floor over solid rock, with `holes` of open air in the rock.
struct Seabed {
    holes: Vec<[i32; 3]>,
}

impl Terrain for Seabed {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights> {
        Some(Heights::from_fn(min, max, |_, _| FLOOR))
    }

    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>> {
        Some(vec![OCEAN; columns.len()])
    }

    fn solid(&self, cells: Vec<[i32; 3]>) -> Option<Vec<bool>> {
        Some(
            cells
                .iter()
                .map(|c| c[1] <= FLOOR && !self.holes.contains(c))
                .collect(),
        )
    }
}

fn caves(terrain: &Seabed) -> Vec<(Site, Plan)> {
    (0..60)
        .filter_map(|i| {
            let site = GRID.site(3, [i, 2 * i])?;
            match derive(3, SEA, site, &[OCEAN], terrain) {
                Derived::Plan(plan) => Some((site, *plan)),
                _ => None,
            }
        })
        .collect()
}

fn named(plan: &Plan, block: &str) -> BTreeSet<[i32; 3]> {
    plan.cells()
        .filter(|(_, m)| m.block.as_str() == block)
        .map(|(c, _)| c)
        .collect()
}

/// Cells holding water: open water and the glowpods that grow in it.
fn flooded(plan: &Plan) -> BTreeSet<[i32; 3]> {
    plan.cells()
        .filter(|(_, m)| {
            m.block.as_str() == "petramond:water" || m.block.as_str().contains("glowpod")
        })
        .map(|(c, _)| c)
        .collect()
}

#[test]
fn the_pocket_is_trapped_air_with_room_to_breathe_and_a_cache() {
    let found = caves(&Seabed { holes: Vec::new() });
    assert!(!found.is_empty(), "a sealed sea floor hides caves");
    for (_, plan) in found {
        let wet = flooded(&plan);
        let air: BTreeSet<[i32; 3]> = plan
            .cells()
            .filter(|(_, m)| m.is_air())
            .map(|(c, _)| c)
            .collect();
        let (low, high) = (
            air.iter().map(|c| c[1]).min().unwrap(),
            air.iter().map(|c| c[1]).max().unwrap(),
        );
        assert!(high > low, "the pocket is at least two cells tall");
        for c in &air {
            assert!(
                !wet.contains(&[c[0], c[1] + 1, c[2]]),
                "{c:?} has water over the pocket"
            );
            for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
                assert!(
                    !wet.contains(&[c[0] + dx, c[1], c[2] + dz]),
                    "water beside the pocket at {c:?}"
                );
            }
        }
        let chest = named(&plan, "petramond:chest");
        assert_eq!(chest.len(), 1);
        let c = *chest.iter().next().unwrap();
        let reachable = [[0, 1, 0], [1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]]
            .iter()
            .any(|d| air.contains(&[c[0] + d[0], c[1] + d[1], c[2] + d[2]]));
        assert!(reachable, "the cache opens onto the pocket");
    }
}

#[test]
fn the_shaft_joins_the_open_sea_to_the_chamber() {
    for (_, plan) in caves(&Seabed { holes: Vec::new() }) {
        let water = flooded(&plan);
        let start = *water
            .iter()
            .find(|c| c[1] == FLOOR)
            .expect("the shaft opens through the floor");
        let mut seen = BTreeSet::from([start]);
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
                if water.contains(&n) && seen.insert(n) {
                    queue.push_back(n);
                }
            }
        }
        assert_eq!(
            seen.len(),
            water.len(),
            "every flooded cell is one body reached from the sea"
        );
    }
}

#[test]
fn open_air_near_a_cave_vetoes_the_site() {
    let sealed = caves(&Seabed { holes: Vec::new() });
    let (site, plan) = sealed.first().expect("a cave to perturb");
    let near = plan
        .cells()
        .map(|(c, _)| c)
        .find(|c| c[1] < FLOOR - 2)
        .map(|c| [c[0], c[1] - 1, c[2]])
        .expect("a cell below the floor");
    let holed = Seabed { holes: vec![near] };
    let near_plan = plan
        .cells()
        .any(|(c, _)| c == [near[0], near[1] + 1, near[2]]);
    assert!(near_plan);
    assert!(
        !matches!(derive(3, SEA, *site, &[OCEAN], &holed), Derived::Plan(_)),
        "a hole beside the cave must veto it"
    );
}
