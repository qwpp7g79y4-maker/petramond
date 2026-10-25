use super::*;

const SEA: i32 = 63;
const BEACH: u8 = 9;

/// Beach rising inland to the west, open sea from `shore` east.
struct Coast {
    shore: i32,
}

impl Terrain for Coast {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights> {
        let shore = self.shore;
        Some(Heights::from_fn(min, max, move |x, _| {
            if x >= shore {
                SEA - 8
            } else {
                SEA + 1
            }
        }))
    }

    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>> {
        Some(vec![BEACH; columns.len()])
    }
}

fn towers(shore_from_site: i32) -> Vec<(Site, mod_sdk::build::Plan)> {
    (0..40)
        .filter_map(|i| {
            let site = GRID.site(2, [i, i + 3])?;
            let coast = Coast {
                shore: site.center[0] + shore_from_site,
            };
            match derive(2, SEA, site, BEACH, &coast) {
                Derived::Plan(plan) => Some((site, *plan)),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn a_lighthouse_turns_its_door_from_the_sea_and_its_ladder_reaches_the_light() {
    let built = towers(6);
    assert!(!built.is_empty(), "a beach by open water raises a light");
    let ladder = Material::named(ladder_facing(Dir::West));
    for (site, plan) in &built {
        let [cx, cz] = site.center;
        let door = plan
            .cells()
            .find(|(_, m)| m.block.as_str() == "petramond:oak_door")
            .map(|(c, m)| (c, m.get_facing()));
        let (at, facing) = door.expect("a door");
        assert_eq!(facing, Some(Dir::West), "the door looks inland");
        assert!(at[0] < cx, "and stands on the landward side");
        let rungs: Vec<i32> = plan
            .cells()
            .filter(|(_, m)| **m == ladder)
            .map(|(c, _)| c[1])
            .collect();
        let (lo, hi) = (*rungs.iter().min().unwrap(), *rungs.iter().max().unwrap());
        assert_eq!(rungs.len() as i32, hi - lo + 1, "an unbroken ladder");
        let light = Material::named(keys::SEA_LANTERN);
        assert_eq!(
            plan.get([cx, hi + 1, cz]),
            Some(&light),
            "the ladder tops out by the light"
        );
    }
}

#[test]
fn no_light_stands_where_no_open_water_lies_in_sight() {
    assert!(towers(SIGHT + 4).is_empty());
}

/// Writes one lighthouse's cells as `x y z block` lines to `$LIGHT_DUMP`, for looking at.
#[test]
#[ignore]
fn zz_lighthouse_dump() {
    let Ok(out) = std::env::var("LIGHT_DUMP") else {
        return;
    };
    let (_, plan) = &towers(6)[0];
    let lines: Vec<String> = plan
        .cells()
        .filter(|(_, m)| !m.is_air())
        .map(|(c, m)| format!("{} {} {} {}", c[0], c[1], c[2], m.block.as_str()))
        .collect();
    std::fs::write(out, lines.join("\n")).unwrap();
}
