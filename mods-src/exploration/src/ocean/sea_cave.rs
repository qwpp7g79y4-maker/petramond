//! Sea caves: a flooded shaft dropping from the sea floor and turning into a
//! hidden chamber whose roof holds a pocket of trapped air. A diver who finds
//! the mouth has to reach the pocket on one breath; a cache waits on a ledge
//! inside it, and glowpods light the drowned floor.
//!
//! The air stays trapped by construction: the shaft enters the chamber below
//! the waterline, every water cell is a still source, and no water cell ever
//! sits beside or above the pocket's air except the flat surface under it.
//! A site whose chamber or shaft would touch any open air or other fluid in
//! the terrain around it builds nothing, so a cave can neither drain the sea
//! into a cave system nor leak its pocket.

use std::collections::BTreeSet;

use mod_sdk::build::{Derived, Dir, Draw, Heights, Material, Noise3, Plan, Site, SiteGrid};
use mod_sdk::*;

use super::sited::Sited;
use super::sunken::loot;
use crate::keys;

pub const GEN_FILTER: GenFeatureFilter = GenFeatureFilter::y_band(-16, 64)
    .without_blocks()
    .without_columns();

const REACH: i32 = 22;

const GRID: SiteGrid = SiteGrid {
    salt: GenRng::salt("exploration:sea_cave"),
    spacing: 88,
    border: REACH,
    chance: 0.4,
    reach: REACH,
};

/// Rock kept between the chamber's roof and the sea floor.
const ROOF: i32 = 3;
/// Shallower water than this hides no cave.
const MIN_WATER: i32 = 4;
const SHAFT_RADIUS: f32 = 1.6;
const GLOWPOD_PER_MILLE: u64 = 220;

/// The positional reads a sea cave is derived from.
pub trait Terrain {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights>;
    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>>;
    /// Whether each cell is solid rock (not open air, not fluid).
    fn solid(&self, cells: Vec<[i32; 3]>) -> Option<Vec<bool>>;
}

struct HostTerrain;

impl Terrain for HostTerrain {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights> {
        Heights::load(min, max)
    }

    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>> {
        mod_sdk::build::surface_biomes(columns)
    }

    fn solid(&self, cells: Vec<[i32; 3]>) -> Option<Vec<bool>> {
        let want = cells.len();
        let spaces = paged(cells, terrain_space_at);
        (spaces.len() == want).then(|| spaces.iter().map(|s| *s == TerrainSpace::Solid).collect())
    }
}

pub struct SeaCaves {
    seas: Vec<u8>,
    sites: Sited,
}

impl SeaCaves {
    pub fn new() -> Option<SeaCaves> {
        let seas: Vec<u8> = [
            keys::OCEAN,
            keys::KELP_FOREST,
            keys::WARM_OCEAN,
            keys::CORAL_REEF,
            keys::COLD_OCEAN,
        ]
        .iter()
        .filter_map(|key| resolve_surface_biome(key))
        .collect();
        (!seas.is_empty()).then(|| SeaCaves {
            seas,
            sites: Sited::new("exploration_sea_cave"),
        })
    }

    pub fn generate(&mut self, ctx: &GenCtx) -> GenOutput {
        let (seed, sea, seas) = (ctx.seed(), ctx.sea_level(), &self.seas);
        self.sites.generate(&GRID, ctx, |site| {
            derive(seed, sea, site, seas, &HostTerrain)
        })
    }
}

/// The cave's rolled form, in world cells.
struct Cave {
    water: BTreeSet<[i32; 3]>,
    air: BTreeSet<[i32; 3]>,
    /// The cell the cache sits in, on a rock ledge over the pocket's water.
    cache: [i32; 3],
    cache_facing: Dir,
}

pub fn derive(seed: u32, sea: i32, site: Site, seas: &[u8], terrain: &dyn Terrain) -> Derived {
    let [cx, cz] = site.center;
    let Some(biomes) = terrain.biomes(vec![site.center]) else {
        return Derived::Unavailable;
    };
    if !biomes.first().is_some_and(|b| seas.contains(b)) {
        return Derived::Nothing;
    }
    let Some(floor) = terrain.heights([cx - REACH, cz - REACH], [cx + REACH, cz + REACH]) else {
        return Derived::Unavailable;
    };
    let mut rng = GRID.rng(seed, site);
    let Some(cave) = Cave::roll(&mut rng, seed, site.center, sea, &floor) else {
        return Derived::Nothing;
    };
    let shell = cave.shell();
    let Some(solid) = terrain.solid(shell.clone()) else {
        return Derived::Unavailable;
    };
    let open_sea = |c: &[i32; 3]| floor.get(c[0], c[2]).is_some_and(|f| c[1] > f);
    let sealed = shell.iter().zip(solid).all(|(c, rock)| rock || open_sea(c));
    if !sealed {
        return Derived::Nothing;
    }
    Derived::Plan(Box::new(cave.build(&mut rng)))
}

impl Cave {
    fn roll(
        rng: &mut GenRng,
        seed: u32,
        center: [i32; 2],
        sea: i32,
        floor: &Heights,
    ) -> Option<Cave> {
        let water_over = sea - 1 - floor.get(center[0], center[1])?;
        if water_over < MIN_WATER {
            return None;
        }
        let r = [
            rng.int(5, 8) as f32,
            rng.int(3, 5) as f32,
            rng.int(5, 8) as f32,
        ];
        let span = r[0].max(r[2]).ceil() as i32 + 1;
        let lowest = (-span..=span)
            .flat_map(|dz| (-span..=span).map(move |dx| [center[0] + dx, center[1] + dz]))
            .filter_map(|[x, z]| floor.get(x, z))
            .min()?;
        let cy = lowest - ROOF - r[1].ceil() as i32 - 1;
        let waterline = cy + rng.int(-1, 0);
        let wobble = Noise3(seed ^ 0x5EA_CA7E);
        let mut water = BTreeSet::new();
        let mut air = BTreeSet::new();
        for dz in -span..=span {
            for dx in -span..=span {
                for dy in -(r[1] as i32 + 1)..=(r[1] as i32 + 1) {
                    let [x, y, z] = [center[0] + dx, cy + dy, center[1] + dz];
                    let q = (dx as f32 / r[0]).powi(2)
                        + (dy as f32 / r[1]).powi(2)
                        + (dz as f32 / r[2]).powi(2);
                    let rough =
                        1.0 + 0.18 * wobble.at(x as f32 * 0.3, y as f32 * 0.3, z as f32 * 0.3);
                    if q > rough {
                        continue;
                    }
                    if y > waterline {
                        air.insert([x, y, z]);
                    } else {
                        water.insert([x, y, z]);
                    }
                }
            }
        }
        if air.iter().map(|c| c[1]).max()? - waterline < 2 {
            return None;
        }
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let out = r[0].max(r[2]) + 4.0;
        let mouth = [
            center[0] + (angle.cos() * out).round() as i32,
            center[1] + (angle.sin() * out).round() as i32,
        ];
        let mouth_floor = floor.get(mouth[0], mouth[1])?;
        let turn = cy - r[1] as i32 + 2;
        if turn >= waterline || sea - 1 - mouth_floor < MIN_WATER {
            return None;
        }
        let mut tube = |p: [f32; 3]| {
            let reach = SHAFT_RADIUS.ceil() as i32;
            for dz in -reach..=reach {
                for dy in -reach..=reach {
                    for dx in -reach..=reach {
                        if ((dx * dx + dy * dy + dz * dz) as f32).sqrt() <= SHAFT_RADIUS {
                            let c = [
                                p[0].round() as i32 + dx,
                                p[1].round() as i32 + dy,
                                p[2].round() as i32 + dz,
                            ];
                            if !air.contains(&c) {
                                water.insert(c);
                            }
                        }
                    }
                }
            }
        };
        for y in turn..=mouth_floor {
            tube([mouth[0] as f32, y as f32, mouth[1] as f32]);
        }
        let steps = (out * 2.0) as i32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = mouth[0] as f32 + (center[0] - mouth[0]) as f32 * t;
            let z = mouth[1] as f32 + (center[1] - mouth[1]) as f32 * t;
            tube([x, turn as f32, z]);
        }
        if water
            .iter()
            .any(|c| air.contains(&[c[0], c[1] - 1, c[2]]) || Self::beside(&air, *c))
        {
            return None;
        }
        let ledge = air
            .iter()
            .filter(|c| c[1] == waterline + 1)
            .max_by_key(|c| (c[0] - center[0]).pow(2) + (c[2] - center[1]).pow(2))
            .copied()?;
        let toward = Dir::of((center[0] - ledge[0]) as f32, (center[1] - ledge[2]) as f32);
        Some(Cave {
            water,
            air,
            cache: ledge,
            cache_facing: toward,
        })
    }

    /// Whether `c` has a pocket cell beside it on its own level (water must only meet air from below).
    fn beside(air: &BTreeSet<[i32; 3]>, c: [i32; 3]) -> bool {
        [[1, 0], [-1, 0], [0, 1], [0, -1]]
            .iter()
            .any(|[dx, dz]| air.contains(&[c[0] + dx, c[1], c[2] + dz]))
    }

    /// Every rock cell touching the cave, which must all stay rock.
    fn shell(&self) -> Vec<[i32; 3]> {
        let inside = |c: &[i32; 3]| self.water.contains(c) || self.air.contains(c);
        let mut out: BTreeSet<[i32; 3]> = BTreeSet::new();
        for c in self.water.iter().chain(&self.air) {
            for [dx, dy, dz] in [
                [1, 0, 0],
                [-1, 0, 0],
                [0, 1, 0],
                [0, -1, 0],
                [0, 0, 1],
                [0, 0, -1],
            ] {
                let n = [c[0] + dx, c[1] + dy, c[2] + dz];
                if !inside(&n) {
                    out.insert(n);
                }
            }
        }
        out.into_iter().collect()
    }

    fn build(&self, rng: &mut GenRng) -> Plan {
        let mut plan = Plan::new();
        let water = Material::named("petramond:water");
        let air = Material::air();
        let rock = Material::named("petramond:stone");
        for c in &self.water {
            plan.set(*c, &water);
        }
        for c in &self.air {
            plan.set(*c, &air);
        }
        let below = [self.cache[0], self.cache[1] - 1, self.cache[2]];
        plan.set(below, &rock);
        plan.set(
            self.cache,
            &Material::named("petramond:chest").facing(self.cache_facing),
        );
        loot(&mut plan, self.cache, keys::SEA_CAVE_LOOT);
        let pods = [
            Material::named(keys::GLOWPOD_TEAL),
            Material::named(keys::GLOWPOD_VIOLET),
        ];
        for c in &self.water {
            let under = [c[0], c[1] - 1, c[2]];
            let over = [c[0], c[1] + 1, c[2]];
            if self.water.contains(&under) || !self.water.contains(&over) {
                continue;
            }
            if (rng.next_u64() % 1000) < GLOWPOD_PER_MILLE {
                plan.set(*c, rng.pick(&pods));
            }
        }
        plan
    }
}

#[cfg(test)]
mod tests;
