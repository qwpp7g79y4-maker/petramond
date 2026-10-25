//! Icebergs: packed-ice masses adrift in the frozen seas. They gather in drifting fields with
//! open water between them, mostly small pieces with now and then a big berg calved off a shelf.
//! A berg is tabular, domed, pinnacled or wedged, with most of its bulk below the waterline
//! spreading into a submerged skirt; it is capped with snow, banded with clear ice, and sheds a
//! few small pieces around it. A site's plan is derived once from positional reads (the floor
//! under it, the biome at its centre) and clipped into every section it reaches.

use mod_sdk::build::{Derived, Draw, Material, Noise2, Plan, Site, SiteGrid};
use mod_sdk::*;

use super::sited::{HostTerrain, Sited, Terrain};
use crate::keys;

/// Sections an iceberg can write: its keel below the waterline, its top above.
pub const GEN_FILTER: GenFeatureFilter = GenFeatureFilter::y_band(16, 104)
    .without_blocks()
    .without_columns();

/// The farthest a site's ice reaches from its centre: the widest berg's skirt (radius × outline
/// wobble × skirt) plus the farthest fragment and its own width.
const REACH: i32 = 36;

/// Candidate sites, fully jittered so no lattice shows; the ice field decides which hold ice.
const GRID: SiteGrid = SiteGrid {
    salt: GenRng::salt("exploration:iceberg"),
    spacing: 36,
    border: 2,
    chance: 1.0,
    reach: REACH,
};

/// The ice field: low-frequency noise of where drifting ice crowds together.
const SALT_FIELD: u32 = 0x1CEF_1E1D;
const FIELD_SCALE: f32 = 1.0 / 260.0;
/// Field strength from which a site may hold ice, and at which it is as crowded as it gets.
const FIELD_OPEN: f32 = 0.0;
const FIELD_FULL: f32 = 0.55;
/// The most crowded field's share of sites holding a berg.
const CROWDED: f32 = 0.8;

/// Water kept between a keel and the floor.
const KEEL_CLEARANCE: i32 = 2;
/// Shallower water than this grows no iceberg.
const MIN_DEPTH: i32 = 6;
/// How far the submerged skirt reaches past the berg's waterline outline.
const SKIRT: f32 = 1.12;

/// One frozen sea's ice: how often a site's berg is medium or large, and how tall it stands.
#[derive(Clone, Copy)]
pub struct Scale {
    pub medium: f32,
    pub large: f32,
    pub height: f32,
}

/// The frozen seas that resolve, each with the size of what floats there.
pub fn frozen_seas() -> Vec<(u8, Scale)> {
    SEAS.into_iter()
        .filter_map(|(key, scale)| Some((resolve_surface_biome(key)?, scale)))
        .collect()
}

const SEAS: [(&str, Scale); 2] = [
    (
        keys::FROZEN_OCEAN,
        Scale {
            medium: 0.18,
            large: 0.04,
            height: 1.0,
        },
    ),
    (
        keys::DEEP_FROZEN_OCEAN,
        Scale {
            medium: 0.26,
            large: 0.12,
            height: 1.4,
        },
    ),
];

#[cfg(test)]
fn frozen_scales() -> Vec<Scale> {
    SEAS.iter().map(|(_, scale)| *scale).collect()
}

pub struct Icebergs {
    /// Frozen sea biomes and the size of what floats in each.
    seas: Vec<(u8, Scale)>,
    sites: Sited,
}

/// The positional reads an iceberg is derived from.
impl Icebergs {
    pub fn new(seas: Vec<(u8, Scale)>) -> Option<Icebergs> {
        (!seas.is_empty()).then(|| Icebergs {
            seas,
            sites: Sited::new("exploration_iceberg"),
        })
    }

    pub fn generate(&mut self, ctx: &GenCtx) -> GenOutput {
        let (seed, sea, seas) = (ctx.seed(), ctx.sea_level(), &self.seas);
        self.sites.generate(&GRID, ctx, |site| {
            derive(seed, sea, site, seas, &HostTerrain)
        })
    }
}

/// The ice at `site`: a berg and the pieces it shed, or nothing where the field leaves open
/// water or the sea is shallow or unfrozen.
pub fn derive(
    seed: u32,
    sea: i32,
    site: Site,
    seas: &[(u8, Scale)],
    terrain: &dyn Terrain,
) -> Derived {
    let [cx, cz] = site.center;
    let mut rng = GRID.rng(seed, site);
    let field = Noise2(seed ^ SALT_FIELD).at(cx as f32 * FIELD_SCALE, cz as f32 * FIELD_SCALE);
    let crowd = ((field - FIELD_OPEN) / (FIELD_FULL - FIELD_OPEN)).clamp(0.0, 1.0);
    if !rng.roll(crowd * CROWDED) {
        return Derived::Nothing;
    }
    let Some(biomes) = terrain.biomes(vec![site.center]) else {
        return Derived::Unavailable;
    };
    let Some(&(_, scale)) = seas.iter().find(|(b, _)| Some(b) == biomes.first()) else {
        return Derived::Nothing;
    };
    let Some(floor) = terrain.heights([cx - REACH, cz - REACH], [cx + REACH, cz + REACH]) else {
        return Derived::Unavailable;
    };
    let waterline = sea - 1;
    if floor.get(cx, cz).is_none_or(|f| waterline - f < MIN_DEPTH) {
        return Derived::Nothing;
    }
    let size = Size::roll(&mut rng, scale);
    let berg = Berg::roll(&mut rng, size, scale, seed, site.center);
    let mut pieces = vec![berg.cells(waterline, |x, z| floor.get(x, z))];
    for _ in 0..berg.size.fragments(&mut rng) {
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let away = berg.radius + rng.range(3.0, 12.0);
        let at = [
            cx + (angle.cos() * away).round() as i32,
            cz + (angle.sin() * away).round() as i32,
        ];
        let size = if rng.roll(0.7) {
            Size::Bit
        } else {
            Size::Small
        };
        let piece = Berg::roll(&mut rng, size, scale, seed, at);
        pieces.push(piece.cells(waterline, |x, z| floor.get(x, z)));
    }
    let mut cells: Vec<[i32; 3]> = pieces.into_iter().flatten().collect();
    cells.sort_unstable();
    cells.dedup();
    let mut plan = Plan::new();
    let packed = Material::named("petramond:packed_ice");
    let clear = Material::named("petramond:ice");
    let snow = Material::named("petramond:snow_block");
    for &[x, y, z] in &cells {
        let top = cells.binary_search(&[x, y + 1, z]).is_err();
        let material = if top && y > waterline {
            &snow
        } else if berg.band(x, y, z) {
            &clear
        } else {
            &packed
        };
        plan.set([x, y, z], material);
    }
    if plan.is_empty() {
        Derived::Nothing
    } else {
        Derived::Plan(Box::new(plan))
    }
}

/// How big a piece of ice is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Size {
    /// A growler or bergy bit, barely clear of the water.
    Bit,
    Small,
    Medium,
    Large,
}

impl Size {
    fn roll(rng: &mut GenRng, scale: Scale) -> Size {
        let r = rng.range(0.0, 1.0);
        if r < scale.large {
            Size::Large
        } else if r < scale.large + scale.medium {
            Size::Medium
        } else if r < scale.large + scale.medium + 0.35 {
            Size::Small
        } else {
            Size::Bit
        }
    }

    /// Waterline radius and height above the water.
    fn dimensions(self, rng: &mut GenRng) -> (f32, f32) {
        match self {
            Size::Bit => (rng.range(1.5, 3.0), rng.range(1.0, 2.0)),
            Size::Small => (rng.range(3.0, 5.0), rng.range(2.0, 5.0)),
            Size::Medium => (rng.range(5.0, 8.0), rng.range(4.0, 9.0)),
            Size::Large => (rng.range(8.0, 14.0), rng.range(7.0, 15.0)),
        }
    }

    /// The pieces a berg this size sheds around it.
    fn fragments(self, rng: &mut GenRng) -> i32 {
        match self {
            Size::Bit => 0,
            Size::Small => rng.int(0, 1),
            Size::Medium => rng.int(1, 3),
            Size::Large => rng.int(2, 4),
        }
    }

    fn form(self, rng: &mut GenRng) -> Form {
        let r = rng.range(0.0, 1.0);
        match self {
            Size::Bit => Form::Dome,
            Size::Small if r < 0.5 => Form::Dome,
            Size::Small if r < 0.75 => Form::Pinnacle,
            Size::Small => Form::Wedge,
            _ if r < 0.45 => Form::Tabular,
            _ if r < 0.65 => Form::Dome,
            _ if r < 0.85 => Form::Pinnacle,
            _ => Form::Wedge,
        }
    }
}

/// The shape of a berg's top above the water.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Form {
    /// Flat top, cliff sides: a slab broken off a shelf.
    Tabular,
    Dome,
    /// A broad peak rising off one side of a lower mass.
    Pinnacle,
    /// A top sloping up to a cliff on one side.
    Wedge,
}

/// One rolled piece of ice.
struct Berg {
    size: Size,
    form: Form,
    center: [i32; 2],
    radius: f32,
    height: f32,
    keel: f32,
    lobes: f32,
    phase: f32,
    /// Pinnacle: the peak's offset from the centre; wedge: the way the top rises (unit).
    lean: [f32; 2],
    lumps: Noise2,
}

impl Berg {
    fn roll(rng: &mut GenRng, size: Size, scale: Scale, seed: u32, center: [i32; 2]) -> Berg {
        let (radius, height) = size.dimensions(rng);
        let form = size.form(rng);
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let lean = match form {
            Form::Pinnacle => {
                let off = radius * rng.range(0.1, 0.4);
                [angle.cos() * off, angle.sin() * off]
            }
            _ => [angle.cos(), angle.sin()],
        };
        let height = height * scale.height;
        Berg {
            size,
            form,
            center,
            radius,
            height,
            keel: height * rng.range(1.5, 2.2),
            lobes: rng.int(2, 5) as f32,
            phase: rng.range(0.0, std::f32::consts::TAU),
            lean,
            lumps: Noise2(seed ^ 0x1CEB_E269),
        }
    }

    /// The waterline outline's radius at bearing `angle`, wobbling with lobes and lumps.
    fn outline(&self, angle: f32, x: i32, z: i32) -> f32 {
        let lobe = 1.0 + 0.13 * (self.lobes * angle + self.phase).sin();
        let lump = 1.0 + 0.22 * self.lumps.at(x as f32 * 0.37, z as f32 * 0.37);
        self.radius * lobe * lump
    }

    /// Height of the top above the water at offset (dx, dz), `t` = distance / outline (0..1).
    fn top(&self, dx: f32, dz: f32, t: f32, x: i32, z: i32) -> f32 {
        // Two scales of lumps, so no top steps up in even rings; a slab's top stays nearly flat.
        let lumps = 0.65 * self.lumps.at(x as f32 * 0.45 + 17.0, z as f32 * 0.45)
            + 0.35 * self.lumps.at(x as f32 * 0.9 - 31.0, z as f32 * 0.9);
        let rough = lumps
            * match self.form {
                Form::Tabular => 0.5,
                Form::Dome => 1.6,
                Form::Pinnacle => 1.2,
                Form::Wedge => 1.0,
            };
        let h = self.height;
        let shaped = match self.form {
            Form::Tabular => h * (1.0 - 0.7 * smoothstep01(((t - 0.8) / 0.2).clamp(0.0, 1.0))),
            Form::Dome => h * (1.0 - t.powf(1.8)),
            Form::Pinnacle => {
                let base = 0.45 * h * (1.0 - t * t);
                let (px, pz) = (dx - self.lean[0], dz - self.lean[1]);
                let reach = self.radius * 0.6;
                let near = ((px * px + pz * pz).sqrt() / reach).min(1.0);
                base.max(1.25 * h * (1.0 - near.powf(1.3)))
            }
            Form::Wedge => {
                let along = (dx * self.lean[0] + dz * self.lean[1]) / self.radius;
                let rise = 0.35 + 0.65 * (0.5 + 0.5 * along.clamp(-1.0, 1.0));
                h * rise * (1.0 - 0.6 * smoothstep01(((t - 0.85) / 0.15).clamp(0.0, 1.0)))
            }
        };
        (shaped + rough).max(0.0)
    }

    /// Clear-ice banding through the packed mass, tilted and wavy.
    fn band(&self, x: i32, y: i32, z: i32) -> bool {
        let wave = self.lumps.at(x as f32 * 0.17 + 40.0, z as f32 * 0.17) * 2.5;
        (y as f32 + wave + (x as f32) * 0.18).rem_euclid(7.0) < 1.0
    }

    /// Every cell of the piece, never deeper than the floor allows, all of one body.
    fn cells(&self, waterline: i32, floor: impl Fn(i32, i32) -> Option<i32>) -> Vec<[i32; 3]> {
        let mut out = Vec::new();
        let span = (self.radius * 1.38 * SKIRT).ceil() as i32 + 1;
        for dz in -span..=span {
            for dx in -span..=span {
                let (x, z) = (self.center[0] + dx, self.center[1] + dz);
                let Some(ground) = floor(x, z) else {
                    continue;
                };
                let d = ((dx * dx + dz * dz) as f32).sqrt();
                let edge = self.outline((dz as f32).atan2(dx as f32), x, z);
                let t = d / edge;
                if t > SKIRT {
                    continue;
                }
                // Below the water the mass spreads into a skirt past the waterline outline.
                let under = (1.0 - (t / SKIRT).powi(2)).max(0.0).powf(0.7);
                let deepest = (waterline as f32 - self.keel * under).round() as i32;
                let lowest = deepest.max(ground + 1 + KEEL_CLEARANCE);
                let highest = if t <= 1.0 {
                    waterline + self.top(dx as f32, dz as f32, t, x, z).round() as i32
                } else {
                    waterline - 1
                };
                for y in lowest..=highest {
                    out.push([x, y, z]);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        attached(out, [self.center[0], waterline, self.center[1]])
    }
}

/// The cells of sorted `cells` joined face to face with `core` (no piece floats free).
fn attached(cells: Vec<[i32; 3]>, core: [i32; 3]) -> Vec<[i32; 3]> {
    if cells.binary_search(&core).is_err() {
        return Vec::new();
    }
    let mut kept = vec![false; cells.len()];
    let mut stack = vec![core];
    while let Some([x, y, z]) = stack.pop() {
        let Ok(i) = cells.binary_search(&[x, y, z]) else {
            continue;
        };
        if kept[i] {
            continue;
        }
        kept[i] = true;
        stack.extend([
            [x + 1, y, z],
            [x - 1, y, z],
            [x, y + 1, z],
            [x, y - 1, z],
            [x, y, z + 1],
            [x, y, z - 1],
        ]);
    }
    cells
        .into_iter()
        .zip(kept)
        .filter(|(_, k)| *k)
        .map(|(c, _)| c)
        .collect()
}

#[cfg(test)]
mod tests;
