use super::*;
use mod_sdk::build::Heights;

const SEA: i32 = 63;
const OCEAN: u8 = 3;

/// A sea floor `depth` under the surface, with a gentle swell.
struct Seabed {
    depth: i32,
}

impl Terrain for Seabed {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights> {
        Some(Heights::from_fn(min, max, |x, z| {
            SEA - self.depth + ((x / 5 + z / 7) % 2)
        }))
    }

    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>> {
        Some(vec![OCEAN; columns.len()])
    }
}

fn wrecks(depth: i32) -> Vec<Plan> {
    (0..80)
        .filter_map(|i| {
            let site = GRID.site(11, [i, 3 * i])?;
            match derive(11, SEA, site, &[OCEAN], &Seabed { depth }) {
                Derived::Plan(plan) => Some(*plan),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn a_wreck_writes_no_air_under_the_sea_and_stays_within_its_reach() {
    let plans = wrecks(22);
    assert!(plans.len() > 10, "deep water holds wrecks: {}", plans.len());
    for plan in &plans {
        for (c, m) in plan.cells() {
            assert!(
                !(m.is_air() && c[1] < SEA),
                "air at {c:?} would open a pocket in the sea"
            );
        }
    }
}

#[test]
fn every_chest_rests_on_wood_and_names_a_loot_table() {
    for plan in wrecks(22) {
        let chest = Material::named("petramond:chest");
        let chests: Vec<[i32; 3]> = plan
            .cells()
            .filter(|(_, m)| Dir::ALL.iter().any(|d| **m == chest.facing(*d)))
            .map(|(c, _)| c)
            .collect();
        assert_eq!(chests.len(), 2, "a captain's chest and the hold's stores");
        for c in chests {
            let below = plan.get([c[0], c[1] - 1, c[2]]).copied();
            assert!(
                below.is_some_and(|m| m.solid_top()),
                "the chest at {c:?} would float"
            );
        }
    }
}

#[test]
fn shallow_water_hides_no_wreck() {
    assert!(wrecks(6).is_empty(), "a wreck needs water over its cabin");
}

/// Writes one wreck's cells as `x y z block` lines to `$WRECK_DUMP`, for looking at.
#[test]
#[ignore]
fn zz_wreck_dump() {
    let Ok(out) = std::env::var("WRECK_DUMP") else {
        return;
    };
    let pick: usize = std::env::var("WRECK_PICK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let plans = wrecks(22);
    let plan = &plans[pick % plans.len()];
    let water = Material::named(keys::WATER);
    let lines: Vec<String> = plan
        .cells()
        .filter(|(_, m)| **m != water && !m.is_air())
        .map(|(c, m)| {
            let state: Vec<String> = m.state_pairs().map(|(k, v)| format!("{k}={v}")).collect();
            format!(
                "{} {} {} {} {}",
                c[0],
                c[1],
                c[2],
                m.block.as_str(),
                state.join(",")
            )
        })
        .collect();
    std::fs::write(out, lines.join("\n")).unwrap();
}
