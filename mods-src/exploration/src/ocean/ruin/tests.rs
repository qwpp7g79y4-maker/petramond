use super::*;
use mod_sdk::build::Heights;
use mod_sdk::build::Plan;

const SEA: i32 = 63;
const COOL: u8 = 3;
const WARM: u8 = 4;

struct Seabed {
    depth: i32,
    biome: u8,
}

impl Terrain for Seabed {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights> {
        Some(Heights::from_fn(min, max, |x, z| {
            SEA - self.depth + ((x / 4 + z / 6) % 2)
        }))
    }

    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>> {
        Some(vec![self.biome; columns.len()])
    }
}

fn ruins(depth: i32, biome: u8) -> Vec<Plan> {
    let seas = Seas {
        cool: vec![COOL],
        warm: vec![WARM],
    };
    let corals = vec![("fx:coral".to_string(), "fx:fan".to_string())];
    (0..80)
        .filter_map(|i| {
            let site = GRID.site(5, [i, 2 * i + 1])?;
            match derive(5, SEA, site, &seas, &corals, &Seabed { depth, biome }) {
                Derived::Plan(plan) => Some(*plan),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn a_ruin_writes_no_air_under_the_sea_and_its_chest_stands_on_the_altar() {
    for biome in [COOL, WARM] {
        let plans = ruins(20, biome);
        assert!(plans.len() > 10, "deep water holds ruins: {}", plans.len());
        let chest = Material::named("petramond:chest");
        for plan in &plans {
            let mut chests = 0;
            for (c, m) in plan.cells() {
                assert!(!(m.is_air() && c[1] < SEA), "air at {c:?} under the sea");
                if Dir::ALL.iter().any(|d| *m == chest.facing(*d)) {
                    chests += 1;
                    let below = plan.get([c[0], c[1] - 1, c[2]]).copied();
                    assert!(
                        below.is_some_and(|b| b.solid_top()),
                        "the chest at {c:?} floats"
                    );
                }
            }
            assert_eq!(chests, 1);
        }
    }
}

/// Writes one ruin's cells as `x y z block` lines to `$RUIN_DUMP`, for looking at.
#[test]
#[ignore]
fn zz_ruin_dump() {
    let Ok(out) = std::env::var("RUIN_DUMP") else {
        return;
    };
    let pick: usize = std::env::var("RUIN_PICK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let biome = if pick.is_multiple_of(2) { WARM } else { COOL };
    let plans = ruins(20, biome);
    let plan = &plans[pick % plans.len()];
    let water = Material::named(keys::WATER);
    let lines: Vec<String> = plan
        .cells()
        .filter(|(_, m)| **m != water && !m.is_air())
        .map(|(c, m)| format!("{} {} {} {}", c[0], c[1], c[2], m.block.as_str()))
        .collect();
    std::fs::write(out, lines.join("\n")).unwrap();
}
