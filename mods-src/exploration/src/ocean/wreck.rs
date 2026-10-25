//! Shipwrecks: a sailing ship settled on the sea floor, half buried in it. A
//! stern cabin keeps the captain's chest, a hatch amidships opens on the hold
//! and its stores, the masts stand or lie snapped across the deck, and the
//! sea has had its way with the planking. When the monsters pack is on, the
//! drowned crew still walks the deck (spawn posts of the `crew` role).
//!
//! Everything a wreck writes below the waterline that is not wood is water,
//! and the column over its footprint is cleared up to the surface, so no kelp
//! the seabed grew before it is left standing in the hull.

mod hull;

use mod_sdk::build::{
    Axis, Derived, Dir, Draw, Frame, Half, Material, Noise3, Plan, Site, SiteGrid,
};
use mod_sdk::*;
use post_marker::{CampBox, Facing, PostMarker, PostRole};

use super::sited::{HostTerrain, Sited, Terrain};
use super::sunken::{loot, Sunken};
use crate::keys;
use hull::{Hull, Part, BEAM, DECK, FORECASTLE, MID};

pub const GEN_FILTER: GenFeatureFilter = GenFeatureFilter::y_band(-32, 72)
    .without_blocks()
    .without_columns();

const REACH: i32 = 18;

const GRID: SiteGrid = SiteGrid {
    salt: GenRng::salt("exploration:wreck"),
    spacing: 120,
    border: REACH,
    chance: 0.5,
    reach: REACH,
};

const LEN: (i32, i32) = (15, 21);
/// Cells the bowsprit reaches past the stem.
const BOWSPRIT: i32 = 2;
/// Water kept over the cabin roof: a wreck lies wholly under the sea.
const HEADROOM: i32 = 2;
/// The cabin's height above the deck, roof included.
const CABIN: i32 = 3;
/// Cells of the length the cabin spans, from the transom.
const CABIN_LEN: i32 = 4;
const HOLE_NOISE: f32 = 0.3;
const SAND_IN_HOLD: f32 = 0.35;
const CREW: (i32, i32) = (2, 4);

/// The positional reads a wreck is derived from.
pub struct Wrecks {
    seas: Vec<u8>,
    sites: Sited,
}

impl Wrecks {
    pub fn new() -> Option<Wrecks> {
        let seas: Vec<u8> = [
            keys::OCEAN,
            keys::DEEP_OCEAN,
            keys::KELP_FOREST,
            keys::WARM_OCEAN,
            keys::CORAL_REEF,
            keys::COLD_OCEAN,
        ]
        .iter()
        .filter_map(|key| resolve_surface_biome(key))
        .collect();
        (!seas.is_empty()).then(|| Wrecks {
            seas,
            sites: Sited::new("exploration_wreck"),
        })
    }

    pub fn generate(&mut self, ctx: &GenCtx) -> GenOutput {
        let (seed, sea, seas) = (ctx.seed(), ctx.sea_level(), &self.seas);
        self.sites.generate(&GRID, ctx, |site| {
            derive(seed, sea, site, seas, &HostTerrain)
        })
    }
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
    let hull = Hull {
        len: rng.int(LEN.0, LEN.1),
    };
    let fwd = *rng.pick(&Dir::ALL);
    let depth = hull.len + BOWSPRIT + 1;
    let [sx, sz] = Frame::world_size(BEAM, depth, fwd);
    let frame = Frame::new([cx - sx / 2, cz - sz / 2], BEAM, depth, fwd);
    let mut lowest = i32::MAX;
    for lz in 0..hull.len {
        for lx in 0..BEAM {
            let [x, z] = frame.at(lx, lz);
            lowest = lowest.min(match floor.get(x, z) {
                Some(f) => f,
                None => return Derived::Unavailable,
            });
        }
    }
    let base = lowest - rng.int(0, 1);
    if base + DECK + CABIN + HEADROOM > sea - 1 {
        return Derived::Nothing;
    }
    let wood = Wood::pick(&mut rng);
    let mut ship = Ship {
        at: Sunken::new(sea, &floor),
        frame,
        base,
        holes: Noise3(seed ^ 0x0EC_C0DE),
    };
    ship.hull(&hull, &wood, &mut rng);
    ship.cabin(&wood);
    let mast = hull.len / 2 + 1;
    ship.mast(&hull, &wood, &mut rng, mast, (7, 11));
    if hull.len >= 19 {
        ship.mast(&hull, &wood, &mut rng, hull.len - 5, (5, 8));
    }
    ship.bowsprit(&hull, &wood);
    ship.hatch_and_stores(mast - 2);
    ship.kelp(&hull, &mut rng);
    ship.crew(&hull, &mut rng, fwd);
    Derived::Plan(Box::new(ship.at.finish()))
}

struct Wood {
    planks: Material,
    slab: Material,
    fence: Material,
    log: &'static str,
}

/// A ship's timber: planks, slab, fence, log.
const WOODS: [[&str; 4]; 2] = [
    [
        "petramond:spruce_planks",
        "petramond:spruce_slab",
        "petramond:spruce_fence",
        "petramond:spruce_log",
    ],
    [
        "petramond:oak_planks",
        "petramond:oak_slab",
        "petramond:oak_fence",
        "petramond:oak_log",
    ],
];

impl Wood {
    fn pick(rng: &mut GenRng) -> Wood {
        let [planks, slab, fence, log] = WOODS[usize::from(!rng.roll(0.7))];
        Wood {
            planks: Material::named(planks),
            slab: Material::named(slab).half(Half::Bottom),
            fence: Material::named(fence),
            log,
        }
    }

    fn log(&self, axis: Axis) -> Material {
        Material::named(self.log).axis(axis)
    }
}

struct Ship<'a> {
    at: Sunken<'a>,
    frame: Frame,
    base: i32,
    holes: Noise3,
}

impl Ship<'_> {
    fn world(&self, lx: i32, y: i32, lz: i32) -> [i32; 3] {
        let [x, z] = self.frame.at(lx, lz);
        [x, self.base + y, z]
    }

    fn put(&mut self, lx: i32, y: i32, lz: i32, m: &Material) {
        let c = self.world(lx, y, lz);
        self.at.put(c, m);
    }

    fn flood(&mut self, lx: i32, y: i32, lz: i32) {
        let c = self.world(lx, y, lz);
        self.at.flood(c);
    }

    fn rotten(&self, lx: i32, y: i32, lz: i32) -> bool {
        let [x, y, z] = self.world(lx, y, lz);
        self.holes
            .at(x as f32 * 0.35, y as f32 * 0.5, z as f32 * 0.35)
            > HOLE_NOISE
    }

    fn hull(&mut self, hull: &Hull, wood: &Wood, rng: &mut GenRng) {
        let sand = Material::named(keys::SAND);
        let wale = wood.log(self.frame.z_axis());
        // Where the sea broke in: a gash low in one side, about amidships.
        let breach_side = if rng.roll(0.5) { 0 } else { BEAM - 1 };
        let breach_at = rng.int(hull.len / 3, hull.len * 2 / 3);
        let breach_len = rng.int(2, 4);
        for lz in 0..hull.len {
            for lx in 0..BEAM {
                for y in 0..=hull.deck_at(lz) + 1 {
                    let Some(part) = hull.part(lx, y, lz) else {
                        continue;
                    };
                    let breached = (lx - breach_side).abs() <= 1
                        && (breach_at..breach_at + breach_len).contains(&lz)
                        && (1..=DECK).contains(&y);
                    let broken =
                        part != Part::Hold && y > 0 && (breached || self.rotten(lx, y, lz));
                    match part {
                        Part::Hold if y == 1 && rng.roll(SAND_IN_HOLD) => {
                            self.put(lx, y, lz, &sand)
                        }
                        Part::Hold => self.flood(lx, y, lz),
                        _ if broken => self.flood(lx, y, lz),
                        Part::Rail if lz < CABIN_LEN + 1 && lz > 0 => {}
                        Part::Rail if rng.roll(0.25) => self.flood(lx, y, lz),
                        Part::Rail => self.put(lx, y, lz, &wood.fence),
                        Part::Wale => self.put(lx, y, lz, &wale),
                        Part::Shell | Part::Deck => self.put(lx, y, lz, &wood.planks),
                    }
                }
            }
        }
    }

    /// The stern castle: a cabin over the aft deck with a door forward, side windows and the
    /// captain's chest against the transom.
    fn cabin(&mut self, wood: &Wood) {
        let glass = Material::named("petramond:glass_pane");
        for lz in 1..=CABIN_LEN {
            for lx in 1..BEAM - 1 {
                let wall = lx == 1 || lx == BEAM - 2 || lz == 1 || lz == CABIN_LEN;
                for y in DECK + 1..DECK + CABIN {
                    let door = lz == CABIN_LEN && lx == MID;
                    let window = (lx == 1 || lx == BEAM - 2) && lz == 2 && y == DECK + 2;
                    if wall && window {
                        self.put(lx, y, lz, &glass);
                    } else if wall && !door && !self.rotten(lx, y, lz) {
                        self.put(lx, y, lz, &wood.planks);
                    } else {
                        self.flood(lx, y, lz);
                    }
                }
                if !self.rotten(lx, DECK + CABIN, lz) {
                    self.put(lx, DECK + CABIN, lz, &wood.slab);
                }
            }
        }
        let chest = Material::named("petramond:chest").facing(self.frame.fwd());
        self.put(MID, DECK + 1, 2, &chest);
        let at = self.world(MID, DECK + 1, 2);
        loot(&mut self.at.plan, at, keys::WRECK_TREASURE);
    }

    /// A mast at `lz`: standing with its yard, nest and the rags of a sail, or snapped low with
    /// its top lying across the deck and over the side onto the sea floor.
    fn mast(&mut self, hull: &Hull, wood: &Wood, rng: &mut GenRng, lz: i32, tall: (i32, i32)) {
        let up = wood.log(Axis::Y);
        if rng.roll(0.55) {
            let stump = rng.int(1, 3);
            for y in DECK + 1..=DECK + stump {
                self.put(MID, y, lz, &up);
            }
            let side = if rng.roll(0.5) { 1 } else { -1 };
            let across = wood.log(self.frame.x_axis());
            let lying = rng.int(5, 7);
            let mut y = DECK + 1;
            for i in 1..=lying {
                let lx = MID + side * i;
                if lx.abs_diff(MID) as i32 > hull.half_width(lz) {
                    let [x, z] = self.frame.at(lx, lz);
                    let ground = self.at.ground(x, z).map_or(y, |f| f - self.base + 1);
                    // Over the side it falls away to the sea floor, two cells a step.
                    y = (y - 2).max(ground);
                    if y == ground && i > MID + 3 {
                        break;
                    }
                }
                self.put(lx, y, lz, &across);
            }
            return;
        }
        let h = rng.int(tall.0, tall.1);
        for y in DECK + 1..=DECK + h {
            self.put(MID, y, lz, &up);
        }
        let yard_y = DECK + h - 2;
        let yard = wood.log(self.frame.x_axis());
        let cloth = Material::named("petramond:wool_block");
        for lx in 1..BEAM - 1 {
            if lx != MID {
                self.put(lx, yard_y, lz, &yard);
            }
            for drop in 1..=2 {
                if rng.roll(0.55 - 0.2 * drop as f32) {
                    self.put(lx, yard_y - drop, lz, &cloth);
                }
            }
        }
        for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
            self.put(MID + dx, DECK + h, lz + dz, &wood.slab);
        }
    }

    fn bowsprit(&mut self, hull: &Hull, wood: &Wood) {
        let along = wood.log(self.frame.z_axis());
        for i in 0..BOWSPRIT {
            self.put(
                MID,
                hull.deck_at(hull.len - 1) + 1 + i / 2,
                hull.len + i,
                &along,
            );
        }
    }

    /// An open hatch amidships over the hold, and the hold's stores below it.
    fn hatch_and_stores(&mut self, lz: i32) {
        for dx in [0, 1] {
            self.flood(MID + dx, DECK, lz);
        }
        let chest = Material::named("petramond:chest").facing(self.frame.back());
        self.put(MID, 1, lz, &chest);
        let at = self.world(MID, 1, lz);
        loot(&mut self.at.plan, at, keys::WRECK_SUPPLIES);
    }

    /// A few kelp stalks the deck has grown since it sank.
    fn kelp(&mut self, hull: &Hull, rng: &mut GenRng) {
        let kelp = Material::named(keys::KELP);
        for _ in 0..rng.int(0, 3) {
            let lz = rng.int(CABIN_LEN + 2, hull.len - 3);
            let lx = rng.int(1, BEAM - 2);
            let deck = self.at.plan.get(self.world(lx, DECK, lz)).copied();
            let above = self.at.plan.get(self.world(lx, DECK + 1, lz)).copied();
            let rooted = deck.is_some_and(|m| m.solid_top());
            if lx == MID || !rooted || above.is_some_and(|m| m.occupies()) {
                continue;
            }
            let len = rng.int(3, 8);
            for y in DECK + 1..=DECK + len {
                if self.base + y >= self.at.sea - 2 {
                    break;
                }
                self.put(lx, y, lz, &kelp);
            }
        }
    }

    /// Posts for the drowned crew on open deck, a few cells apart.
    fn crew(&mut self, hull: &Hull, rng: &mut GenRng, fwd: Dir) {
        let water = Material::named(keys::WATER);
        let open = |plan: &Plan, c: [i32; 3]| plan.get(c).is_none_or(|m| *m == water);
        let mut decks: Vec<[i32; 3]> = Vec::new();
        for lz in CABIN_LEN + 1..hull.len - FORECASTLE {
            for lx in 1..BEAM - 1 {
                let c = self.world(lx, DECK, lz);
                let solid = self
                    .at
                    .plan
                    .get(c)
                    .is_some_and(|m| !m.is_air() && *m != water);
                let clear = (1..=2).all(|dy| open(&self.at.plan, [c[0], c[1] + dy, c[2]]));
                if solid && clear {
                    decks.push(c);
                }
            }
        }
        let (min, max) = self.at.bounds();
        let want = rng.int(CREW.0, CREW.1) as usize;
        let mut posts: Vec<[i32; 3]> = Vec::new();
        while posts.len() < want && !decks.is_empty() {
            let c = decks.swap_remove(rng.int(0, decks.len() as i32 - 1) as usize);
            if posts
                .iter()
                .any(|p| (p[0] - c[0]).abs() + (p[2] - c[2]).abs() < 3)
            {
                continue;
            }
            let [dx, dz] = fwd.offset();
            let marker = PostMarker {
                role: PostRole::Crew,
                facing: Facing::along(dx as f32, dz as f32),
                camp: CampBox { min, max },
            };
            self.at.plan.data(c, post_marker::KEY, marker.encode());
            posts.push(c);
        }
    }
}

#[cfg(test)]
mod tests;
