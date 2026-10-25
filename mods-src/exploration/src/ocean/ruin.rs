//! Sunken ruins: the stump of an old shrine drowned with its coast. A stepped
//! platform carries a ring of columns (some standing with their lintels, some
//! snapped and lying where they fell) round a walled cella whose altar still
//! keeps an offering chest. Warm seas build them of sandstone and grow coral
//! over every top they can reach; cooler ones build in stone brick.

use mod_sdk::build::{Derived, Dir, Draw, Frame, Half, Material, Noise3, Site, SiteGrid};
use mod_sdk::*;

use super::sited::{HostTerrain, Sited, Terrain};
use super::sunken::{loot, Sunken};
use crate::keys;

pub const GEN_FILTER: GenFeatureFilter = GenFeatureFilter::y_band(-32, 72)
    .without_blocks()
    .without_columns();

const REACH: i32 = 14;

const GRID: SiteGrid = SiteGrid {
    salt: GenRng::salt("exploration:ruin"),
    spacing: 150,
    border: REACH,
    chance: 0.45,
    reach: REACH,
};

/// The platform's side, odd so the cella has a middle.
const SIDES: [i32; 3] = [9, 11, 13];
/// The cella's side, centred on the platform.
const CELLA: i32 = 5;
const COLUMN: (i32, i32) = (3, 6);
/// Water kept over the tallest column.
const HEADROOM: i32 = 2;
const DECAY: f32 = 0.34;
const CORAL_PER_TOP: f32 = 0.4;

/// One sea's way of building: its block, the slab and stairs it has (a stone without them
/// lays whole blocks there), and whether coral grows on it.
struct Style {
    block: &'static str,
    slab: Option<&'static str>,
    stairs: Option<&'static str>,
    coral: bool,
}

const SANDSTONE: Style = Style {
    block: "petramond:sandstone",
    slab: None,
    stairs: None,
    coral: true,
};
const STONE_BRICK: Style = Style {
    block: "petramond:stone_bricks",
    slab: Some("petramond:stone_bricks_slab"),
    stairs: Some("petramond:stone_bricks_stairs"),
    coral: false,
};

/// The seas ruins stand in, and which of them are warm.
pub struct Seas {
    pub cool: Vec<u8>,
    pub warm: Vec<u8>,
}

pub struct Ruins {
    seas: Seas,
    corals: Vec<(String, String)>,
    sites: Sited,
}

impl Ruins {
    pub fn new() -> Option<Ruins> {
        let resolve = |keys: &[&str]| -> Vec<u8> {
            keys.iter()
                .filter_map(|k| resolve_surface_biome(k))
                .collect()
        };
        let seas = Seas {
            cool: resolve(&[keys::OCEAN, keys::KELP_FOREST, keys::COLD_OCEAN]),
            warm: resolve(&[keys::WARM_OCEAN, keys::CORAL_REEF]),
        };
        let corals = keys::CORALS
            .iter()
            .map(|(block, fan)| (block.to_string(), fan.to_string()))
            .collect();
        (!seas.cool.is_empty() || !seas.warm.is_empty()).then(|| Ruins {
            seas,
            corals,
            sites: Sited::new("exploration_ruin"),
        })
    }

    pub fn generate(&mut self, ctx: &GenCtx) -> GenOutput {
        let (seed, sea, seas, corals) = (ctx.seed(), ctx.sea_level(), &self.seas, &self.corals);
        self.sites.generate(&GRID, ctx, |site| {
            derive(seed, sea, site, seas, corals, &HostTerrain)
        })
    }
}

pub fn derive(
    seed: u32,
    sea: i32,
    site: Site,
    seas: &Seas,
    corals: &[(String, String)],
    terrain: &dyn Terrain,
) -> Derived {
    let [cx, cz] = site.center;
    let Some(biomes) = terrain.biomes(vec![site.center]) else {
        return Derived::Unavailable;
    };
    let style = match biomes.first() {
        Some(b) if seas.warm.contains(b) => &SANDSTONE,
        Some(b) if seas.cool.contains(b) => &STONE_BRICK,
        _ => return Derived::Nothing,
    };
    let Some(floor) = terrain.heights([cx - REACH, cz - REACH], [cx + REACH, cz + REACH]) else {
        return Derived::Unavailable;
    };
    let mut rng = GRID.rng(seed, site);
    let side = *rng.pick(&SIDES);
    let door = *rng.pick(&Dir::ALL);
    let frame = Frame::new([cx - side / 2, cz - side / 2], side, side, door);
    let mut lowest = i32::MAX;
    for lz in 0..side {
        for lx in 0..side {
            let [x, z] = frame.at(lx, lz);
            lowest = lowest.min(match floor.get(x, z) {
                Some(f) => f,
                None => return Derived::Unavailable,
            });
        }
    }
    let base = lowest + 1;
    if base + 1 + COLUMN.1 + HEADROOM > sea - 1 {
        return Derived::Nothing;
    }
    let mut ruin = Ruin {
        at: Sunken::new(sea, &floor),
        frame,
        side,
        base,
        style,
        decay: Noise3(seed ^ 0x0001_2B1E),
    };
    ruin.platform(&mut rng);
    ruin.cella(&mut rng);
    ruin.columns(&mut rng);
    ruin.drifts(&mut rng);
    if style.coral && !corals.is_empty() {
        ruin.coral(&mut rng, corals);
    }
    Derived::Plan(Box::new(ruin.at.finish()))
}

struct Ruin<'a> {
    at: Sunken<'a>,
    frame: Frame,
    side: i32,
    /// The platform's top layer.
    base: i32,
    style: &'static Style,
    decay: Noise3,
}

impl Ruin<'_> {
    fn world(&self, lx: i32, y: i32, lz: i32) -> [i32; 3] {
        let [x, z] = self.frame.at(lx, lz);
        [x, self.base + y, z]
    }

    fn put(&mut self, lx: i32, y: i32, lz: i32, m: &Material) {
        let c = self.world(lx, y, lz);
        self.at.put(c, m);
    }

    fn crumbled(&self, lx: i32, y: i32, lz: i32) -> bool {
        let [x, y, z] = self.world(lx, y, lz);
        self.decay
            .at(x as f32 * 0.4, y as f32 * 0.55, z as f32 * 0.4)
            > DECAY
    }

    fn block(&self) -> Material {
        Material::named(self.style.block)
    }

    /// The platform, its rim stepped down in stairs, on a foundation down to the sea floor.
    fn platform(&mut self, rng: &mut GenRng) {
        let block = self.block();
        for lz in 0..self.side {
            for lx in 0..self.side {
                let c = self.world(lx, 0, lz);
                let ground = self.at.ground(c[0], c[2]).unwrap_or(c[1] - 1);
                for y in ground + 1..c[1] {
                    self.at.put([c[0], y, c[2]], &block);
                }
                let rim = [
                    (lx == 0, self.frame.left()),
                    (lx == self.side - 1, self.frame.right()),
                ]
                .into_iter()
                .chain([
                    (lz == 0, self.frame.back()),
                    (lz == self.side - 1, self.frame.fwd()),
                ])
                .find(|(on, _)| *on)
                .map(|(_, out)| out);
                match rim {
                    Some(_) if rng.roll(0.2) => {}
                    Some(out) => {
                        let step = self
                            .style
                            .stairs
                            .map_or(block, |s| Material::named(s).facing(out.opposite()));
                        self.put(lx, 0, lz, &step);
                    }
                    None => self.put(lx, 0, lz, &block),
                }
            }
        }
    }

    /// The walled cella in the middle, its doorway facing the frame's front, the altar and its
    /// chest at the back.
    fn cella(&mut self, rng: &mut GenRng) {
        let block = self.block();
        let lo = (self.side - CELLA) / 2;
        let hi = lo + CELLA - 1;
        let mid = self.side / 2;
        let height = rng.int(2, 3);
        for lz in lo..=hi {
            for lx in lo..=hi {
                let wall = lx == lo || lx == hi || lz == lo || lz == hi;
                for y in 1..=height {
                    let doorway = lz == hi && lx == mid && y <= 2;
                    let c = self.world(lx, y, lz);
                    if wall && !doorway && !self.crumbled(lx, y, lz) {
                        self.put(lx, y, lz, &block);
                    } else {
                        self.at.flood(c);
                    }
                }
            }
        }
        let altar = lo + 1;
        self.put(mid, 1, altar, &block);
        let chest = Material::named("petramond:chest").facing(self.frame.fwd());
        self.put(mid, 2, altar, &chest);
        let at = self.world(mid, 2, altar);
        loot(&mut self.at.plan, at, keys::RUIN_OFFERING);
    }

    /// Columns on the platform's outer ring, every other cell; a standing neighbour pair of
    /// one height carries a slab lintel, a snapped one has its top lying beside it.
    fn columns(&mut self, rng: &mut GenRng) {
        let block = self.block();
        let lintel = self
            .style
            .slab
            .map_or(block, |s| Material::named(s).half(Half::Bottom));
        let ring = 1;
        let far = self.side - 2;
        let mut spots: Vec<[i32; 2]> = Vec::new();
        for i in (ring..=far).step_by(2) {
            spots.extend([[i, ring], [i, far], [ring, i], [far, i]]);
        }
        spots.sort_unstable();
        spots.dedup();
        let mut heights: Vec<([i32; 2], i32)> = Vec::new();
        for [lx, lz] in spots {
            if rng.roll(0.15) {
                continue;
            }
            let snapped = rng.roll(0.4);
            let h = if snapped {
                rng.int(1, 2)
            } else {
                rng.int(COLUMN.0, COLUMN.1)
            };
            for y in 1..=h {
                self.put(lx, y, lz, &block);
            }
            if snapped {
                let fell = *rng.pick(&Dir::ALL);
                let [dx, dz] = self.frame_step(fell);
                for i in 1..=rng.int(2, 3) {
                    let (fx, fz) = (lx + dx * i, lz + dz * i);
                    if (0..self.side).contains(&fx) && (0..self.side).contains(&fz) {
                        self.put(fx, 1, fz, &block);
                    }
                }
            } else {
                heights.push(([lx, lz], h));
            }
        }
        for (i, &([ax, az], ha)) in heights.iter().enumerate() {
            for &([bx, bz], hb) in &heights[i + 1..] {
                let adjacent =
                    (ax == bx && (az - bz).abs() == 2) || (az == bz && (ax - bx).abs() == 2);
                if adjacent && ha == hb && rng.roll(0.6) {
                    for (lx, lz) in [(ax, az), ((ax + bx) / 2, (az + bz) / 2), (bx, bz)] {
                        self.put(lx, ha + 1, lz, &lintel);
                    }
                }
            }
        }
    }

    /// A local step along `dir` in the frame's own axes.
    fn frame_step(&self, dir: Dir) -> [i32; 2] {
        match dir {
            Dir::North => [0, -1],
            Dir::South => [0, 1],
            Dir::East => [1, 0],
            Dir::West => [-1, 0],
        }
    }

    /// Sand the currents have heaped against the platform, one cell deep.
    fn drifts(&mut self, rng: &mut GenRng) {
        let sand = Material::named(keys::SAND);
        let windward = rng.int(0, 1) * (self.side - 1);
        for lz in 0..self.side {
            for lx in 0..self.side {
                let reach = (lx - windward).abs();
                let c = self.world(lx, 1, lz);
                if reach < 3 && !self.at.plan.occupied(c) && rng.roll(0.7 - 0.2 * reach as f32) {
                    self.put(lx, 1, lz, &sand);
                }
            }
        }
    }

    /// Coral over the ruin's open tops: the top block turned to coral, a fan on some of them.
    fn coral(&mut self, rng: &mut GenRng, corals: &[(String, String)]) {
        let block = self.block();
        let tops: Vec<[i32; 3]> = self
            .at
            .plan
            .cells()
            .filter(|(c, m)| **m == block && !self.at.plan.occupied([c[0], c[1] + 1, c[2]]))
            .map(|(c, _)| c)
            .collect();
        let (coral, fan) = rng.pick(corals).clone();
        let (coral, fan) = (Material::named(&coral), Material::named(&fan));
        for c in tops {
            if c[1] + 1 >= self.at.sea - 1 || !rng.roll(CORAL_PER_TOP) {
                continue;
            }
            self.at.put(c, &coral);
            if rng.roll(0.5) {
                self.at.put([c[0], c[1] + 1, c[2]], &fan);
            }
        }
    }
}

#[cfg(test)]
mod tests;
