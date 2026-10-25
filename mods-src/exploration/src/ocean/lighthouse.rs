//! Lighthouses: a banded round tower on a stone plinth at the top of a beach,
//! its door turned from the sea. A ladder climbs the inside to a gallery
//! ringed with a rail, a glazed lantern room keeps the light burning (a sea
//! lantern) and the keeper's chest, and a cap closes the top. One stands on a
//! beach only where open water lies within sight of it.

use mod_sdk::build::{Derived, Dir, Draw, Heights, Material, Plan, Site, SiteGrid};
use mod_sdk::*;

use super::sited::{HostTerrain, Sited, Terrain};
use super::sunken::loot;
use crate::keys;

pub const GEN_FILTER: GenFeatureFilter = GenFeatureFilter::y_band(48, 128)
    .without_blocks()
    .without_columns()
    .with_claims();

const REACH: i32 = 14;

const GRID: SiteGrid = SiteGrid {
    salt: GenRng::salt("exploration:lighthouse"),
    spacing: 260,
    border: REACH,
    chance: 0.6,
    reach: REACH,
};

/// The tower's radius: a 5-wide round shaft around a 3-wide well.
const RADIUS: f32 = 2.6;
const WELL: f32 = 1.5;
/// The gallery's radius, one wider than the shaft all round.
const GALLERY: f32 = 3.6;
const HEIGHT: (i32, i32) = (13, 19);
const PLINTH: i32 = 2;
/// Courses to a colour band.
const BAND: i32 = 3;
/// The highest beach a tower stands on, above the sea.
const ABOVE_SEA: i32 = 6;
/// Water this deep or deeper, this close, makes a coast worth a light.
const OPEN_WATER: i32 = 3;
const SIGHT: i32 = 12;

const DOOR: [[i32; 3]; 2] = [[0, 0, 0], [0, 1, 0]];

pub struct Lighthouses {
    beach: u8,
    sites: Sited,
}

impl Lighthouses {
    pub fn new() -> Option<Lighthouses> {
        Some(Lighthouses {
            beach: resolve_surface_biome(keys::BEACH)?,
            sites: Sited::new("exploration_lighthouse"),
        })
    }

    pub fn generate(&mut self, ctx: &GenCtx) -> GenOutput {
        let (seed, sea, beach) = (ctx.seed(), ctx.sea_level(), self.beach);
        self.sites.generate(&GRID, ctx, |site| {
            derive(seed, sea, site, beach, &HostTerrain)
        })
    }
}

pub fn derive(seed: u32, sea: i32, site: Site, beach: u8, terrain: &dyn Terrain) -> Derived {
    let [cx, cz] = site.center;
    let Some(biomes) = terrain.biomes(vec![site.center]) else {
        return Derived::Unavailable;
    };
    if biomes.first() != Some(&beach) {
        return Derived::Nothing;
    }
    let Some(ground) = terrain.heights([cx - REACH, cz - REACH], [cx + REACH, cz + REACH]) else {
        return Derived::Unavailable;
    };
    let Some(seaward) = seaward(&ground, site.center, sea) else {
        return Derived::Nothing;
    };
    let footprint = disc(GALLERY);
    let mut top = i32::MIN;
    for [dx, dz] in &footprint {
        match ground.get(cx + dx, cz + dz) {
            Some(g) => top = top.max(g),
            None => return Derived::Unavailable,
        }
    }
    if !(sea..=sea + ABOVE_SEA).contains(&top) {
        return Derived::Nothing;
    }
    let mut rng = GRID.rng(seed, site);
    let tower = Tower {
        center: site.center,
        base: top + 1,
        height: rng.int(HEIGHT.0, HEIGHT.1),
        seaward,
        shaft: disc(RADIUS),
        well: disc(WELL),
    };
    let mut plan = Plan::new();
    // Trees stay out of the tower and its gallery's shadow.
    let reach = GALLERY.ceil() as i32;
    let bounds = ColumnBox {
        min: [cx - reach, cz - reach],
        max: [cx + reach, cz + reach],
    };
    let gallery = disc(GALLERY);
    plan.claim(ColumnMask::from_fn(bounds, |[x, z]| {
        gallery.contains(&[x - cx, z - cz])
    }));
    tower.shaft(&mut plan, &ground);
    tower.entrance(&mut plan);
    tower.lantern_room(&mut plan);
    Derived::Plan(Box::new(plan))
}

const WHITE: &str = "petramond:white_terracotta";
const RED: &str = "petramond:red_terracotta";
const BRICK: &str = "petramond:stone_bricks";

struct Tower {
    center: [i32; 2],
    /// The plinth's bottom course.
    base: i32,
    height: i32,
    seaward: Dir,
    shaft: Vec<[i32; 2]>,
    well: Vec<[i32; 2]>,
}

impl Tower {
    fn at(&self, [dx, dz]: [i32; 2], y: i32) -> [i32; 3] {
        [self.center[0] + dx, y, self.center[1] + dz]
    }

    fn deck(&self) -> i32 {
        self.base + self.height
    }

    /// The foundation down to the sand under every shaft column, the plinth and the banded
    /// shaft round its hollow well.
    fn shaft(&self, plan: &mut Plan, ground: &Heights) {
        let [white, red, brick] = [WHITE, RED, BRICK].map(Material::named);
        for &d in &self.shaft {
            let foot = self.at(d, 0);
            let floor = ground.get(foot[0], foot[2]).unwrap_or(self.base - 1);
            for y in floor + 1..self.base {
                plan.set(self.at(d, y), &brick);
            }
            let wall = !self.well.contains(&d);
            for k in 0..self.height {
                let m = if k < PLINTH {
                    &brick
                } else if (k - PLINTH) / BAND % 2 == 0 {
                    &white
                } else {
                    &red
                };
                let c = self.at(d, self.base + k);
                if wall || k == 0 {
                    plan.set(c, m);
                } else {
                    plan.set(c, &Material::air());
                }
            }
        }
    }

    /// The door, turned from the sea, and the ladder up the far side of the well and on
    /// through the hatch in the gallery floor.
    fn entrance(&self, plan: &mut Plan) {
        let landward = self.seaward.opposite();
        let [lx, lz] = landward.offset();
        let door = Material::named("petramond:oak_door")
            .facing(landward)
            .open(false);
        plan.object(self.at([lx * 2, lz * 2], self.base + 1), &door, &DOOR);
        let ladder = Material::named(ladder_facing(landward));
        let rung = self.seaward.offset();
        for y in self.base + 1..=self.deck() {
            plan.set(self.at(rung, y), &ladder);
        }
    }

    /// The gallery and its rail, the glazed lantern room with the light and the keeper's
    /// chest, and the cap.
    fn lantern_room(&self, plan: &mut Plan) {
        let deck = self.deck();
        let [brick, red, slab, glass, fence, lantern] = [
            BRICK,
            RED,
            "petramond:stone_bricks_slab",
            "petramond:glass_pane",
            "petramond:spruce_fence",
            keys::SEA_LANTERN,
        ]
        .map(Material::named);
        let rail = disc(GALLERY - 1.0);
        let hatch = self.seaward.offset();
        for d in disc(GALLERY) {
            if d != hatch {
                plan.set(self.at(d, deck), &brick);
            }
            if !rail.contains(&d) {
                plan.set(self.at(d, deck + 1), &fence);
            }
        }
        let air = Material::air();
        for &d in &self.shaft {
            let inner = self.well.contains(&d);
            for k in 1..=2 {
                plan.set(self.at(d, deck + k), if inner { &air } else { &glass });
            }
            plan.set(self.at(d, deck + 3), &red);
        }
        for &d in &self.well {
            plan.set(self.at(d, deck + 4), if d == [0, 0] { &red } else { &slab });
        }
        plan.set(self.at([0, 0], deck + 1), &lantern);
        plan.set(self.at([0, 0], deck + 2), &lantern);
        let chest_at = self.at(self.seaward.opposite().offset(), deck + 1);
        plan.set(
            chest_at,
            &Material::named("petramond:chest").facing(self.seaward),
        );
        loot(plan, chest_at, keys::LIGHTHOUSE_KEEPER);
    }
}

/// The way to the open water nearest `center`, if any lies within sight.
fn seaward(ground: &Heights, center: [i32; 2], sea: i32) -> Option<Dir> {
    let mut sum = [0.0f32; 2];
    let mut seen = 0;
    for dz in -SIGHT..=SIGHT {
        for dx in -SIGHT..=SIGHT {
            let g = ground.get(center[0] + dx, center[1] + dz)?;
            if sea - 1 - g >= OPEN_WATER {
                let d = ((dx * dx + dz * dz) as f32).sqrt().max(1.0);
                sum[0] += dx as f32 / d;
                sum[1] += dz as f32 / d;
                seen += 1;
            }
        }
    }
    (seen > 0).then(|| Dir::of(sum[0], sum[1]))
}

/// The cells of a disc of `radius` about the origin.
fn disc(radius: f32) -> Vec<[i32; 2]> {
    let r = radius.ceil() as i32;
    (-r..=r)
        .flat_map(|dz| (-r..=r).map(move |dx| [dx, dz]))
        .filter(|[dx, dz]| ((dx * dx + dz * dz) as f32) <= radius * radius)
        .collect()
}

/// The ladder row hung on the wall opposite `facing`'s side, facing that way.
fn ladder_facing(facing: Dir) -> &'static str {
    match facing {
        Dir::North => "petramond:ladder",
        Dir::South => "petramond:ladder_south",
        Dir::West => "petramond:ladder_west",
        Dir::East => "petramond:ladder_east",
    }
}

#[cfg(test)]
mod tests;
