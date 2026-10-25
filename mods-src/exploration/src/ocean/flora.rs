//! Seafloor dressing: what grows on each sea biome's floor.
//!
//! SEAM CONTRACT: every decision is per COLUMN, a pure function of
//! `(seed, column, floor y, bed)`. Column data is identical for every section
//! of a column, so each section a kelp stalk or reef mound crosses re-derives
//! the same plan and writes only its own cells. Cells are written only over
//! water this section can see; a cell another section owns is that section's
//! call.

use mod_sdk::*;

use super::reef::{self, Coral, Reef, ReefField};
use super::trench::Carve;
use super::Ocean;
use crate::keys;

/// Sections that can hold any of it: the deepest floors up to the waterline.
pub const GEN_FILTER: GenFeatureFilter = GenFeatureFilter::y_band(-32, 64);

/// Water cells kept clear above the tallest growth, so nothing breaks the surface.
const TOP_GAP: i32 = 2;
const KELP_LEN: (i32, i32) = (3, 18);
const MOST_GROWTH: i32 = KELP_LEN.1 + 1;

const SALT_KELP: u64 = 0x0CEA_0000_0000_0001;
const SALT_SEAGRASS: u64 = 0x0CEA_0000_0000_0002;
const SALT_GLOW: u64 = 0x0CEA_0000_0000_0004;
const SALT_CLAM: u64 = 0x0CEA_0000_0000_0005;
/// Clams found already open on their pearl.
const OPEN_CLAM_PER_MILLE: u64 = 120;
/// Authored `facing` values, indexed by a growth's `facing`.
const FACINGS: [&str; 4] = ["north", "east", "south", "west"];
const SALT_ROLL: u64 = 0x0CEA_0000_0000_0010;

/// A reef proper: heads close together, sand channels between.
const REEF: ReefField = ReefField {
    lattice: 15,
    per_mille: 820,
    radius: (3, 7),
};
/// Open warm water: now and then a small patch reef among the seagrass.
const PATCH_REEFS: ReefField = ReefField {
    lattice: 36,
    per_mille: 260,
    radius: (2, 4),
};

/// The deep, dark floor where glowpods grow.
pub struct Deep {
    pub colonies: ColonyField,
    /// Water above the floor before it is dark enough.
    pub min_depth: i32,
}

/// What grows on one sea biome's floor. Precedence when fields overlap:
/// reef, kelp, glowpods, clams, seagrass. `trench` cuts canyons into it first; `reef` stands
/// coral heads on it ([`reef`]).
pub struct Bed {
    pub biome: &'static str,
    pub trench: bool,
    pub reef: Option<ReefField>,
    pub kelp: Option<ColonyField>,
    pub glowpods: Option<Deep>,
    pub clams: Option<ColonyField>,
    pub seagrass: Option<ColonyField>,
}

pub struct ResolvedBed {
    pub biome: u8,
    pub bed: &'static Bed,
}

pub(super) const fn colonies(
    salt: u64,
    lattice: i32,
    one_in: i32,
    radius: (i32, i32),
    core: i32,
    rim: i32,
    stray: i32,
) -> ColonyField {
    ColonyField {
        salt,
        lattice,
        one_in,
        radius,
        core,
        rim,
        stray,
    }
}

pub const BEDS: &[Bed] = &[
    Bed {
        biome: keys::KELP_FOREST,
        trench: false,
        reef: None,
        kelp: Some(colonies(SALT_KELP, 14, 1, (5, 10), 520, 140, 25)),
        glowpods: None,
        clams: Some(colonies(SALT_CLAM, 26, 3, (2, 4), 160, 30, 0)),
        seagrass: Some(colonies(SALT_SEAGRASS, 10, 2, (3, 6), 400, 50, 10)),
    },
    Bed {
        biome: keys::OCEAN,
        trench: false,
        reef: None,
        kelp: Some(colonies(SALT_KELP ^ 1, 22, 3, (4, 8), 360, 60, 0)),
        glowpods: None,
        clams: Some(colonies(SALT_CLAM ^ 1, 20, 2, (2, 5), 220, 40, 0)),
        seagrass: Some(colonies(SALT_SEAGRASS ^ 1, 12, 2, (4, 8), 440, 60, 8)),
    },
    Bed {
        biome: keys::COLD_OCEAN,
        trench: false,
        reef: None,
        kelp: Some(colonies(SALT_KELP ^ 2, 20, 2, (4, 8), 300, 50, 4)),
        glowpods: None,
        clams: None,
        seagrass: Some(colonies(SALT_SEAGRASS ^ 2, 14, 3, (3, 6), 260, 40, 0)),
    },
    Bed {
        biome: keys::WARM_OCEAN,
        trench: false,
        reef: Some(PATCH_REEFS),
        kelp: Some(colonies(SALT_KELP ^ 3, 40, 5, (3, 5), 300, 40, 0)),
        glowpods: None,
        clams: Some(colonies(SALT_CLAM ^ 3, 18, 2, (2, 5), 260, 50, 0)),
        seagrass: Some(colonies(SALT_SEAGRASS ^ 3, 11, 1, (4, 8), 560, 90, 12)),
    },
    Bed {
        biome: keys::CORAL_REEF,
        trench: false,
        reef: Some(REEF),
        kelp: None,
        glowpods: None,
        clams: Some(colonies(SALT_CLAM ^ 4, 20, 2, (2, 4), 220, 40, 0)),
        seagrass: Some(colonies(SALT_SEAGRASS ^ 4, 16, 3, (2, 4), 300, 40, 0)),
    },
    Bed {
        biome: keys::DEEP_OCEAN,
        trench: true,
        reef: None,
        kelp: Some(colonies(SALT_KELP ^ 5, 28, 4, (3, 6), 320, 50, 0)),
        glowpods: Some(Deep {
            colonies: colonies(SALT_GLOW, 18, 2, (3, 7), 260, 40, 2),
            min_depth: 20,
        }),
        clams: None,
        seagrass: Some(colonies(SALT_SEAGRASS ^ 5, 18, 3, (3, 6), 380, 50, 0)),
    },
    Bed {
        biome: keys::FROZEN_OCEAN,
        trench: false,
        reef: None,
        kelp: Some(colonies(SALT_KELP ^ 6, 24, 3, (3, 7), 320, 50, 0)),
        glowpods: None,
        clams: None,
        seagrass: Some(colonies(SALT_SEAGRASS ^ 6, 16, 3, (3, 5), 300, 40, 0)),
    },
    Bed {
        biome: keys::DEEP_FROZEN_OCEAN,
        trench: false,
        reef: None,
        kelp: Some(colonies(SALT_KELP ^ 7, 30, 4, (3, 6), 300, 40, 0)),
        glowpods: None,
        clams: None,
        seagrass: Some(colonies(SALT_SEAGRASS ^ 7, 20, 4, (2, 5), 300, 40, 0)),
    },
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Growth {
    Bare,
    Seagrass,
    Kelp { len: i32 },
    Reef(Reef),
    Glowpod { kind: usize },
    Clam { open: bool, facing: u8 },
}

impl Growth {
    /// Cells above the floor this growth fills.
    pub fn height(self) -> i32 {
        match self {
            Growth::Bare => 0,
            Growth::Seagrass | Growth::Glowpod { .. } | Growth::Clam { .. } => 1,
            Growth::Kelp { len } => len,
            Growth::Reef(Reef::Head(column)) => column.height(),
            Growth::Reef(Reef::Fringe(_)) => 1,
        }
    }
}

/// What a column's growth can choose from: the coral palette, how many glowpod kinds there are,
/// and `reef` when the reef's own blocks are there to build heads from.
#[derive(Copy, Clone)]
pub struct Palette<'a> {
    pub corals: &'a [Coral],
    pub glowpods: usize,
    pub reef: bool,
}

pub(super) fn rolls(seed: u32, salt: u64, wx: i32, wz: i32, per_mille: i32) -> Option<GenRng> {
    let mut rng = GenRng::positional(seed, salt ^ SALT_ROLL, wx, 0, wz);
    ((rng.next_u64() % 1000) < per_mille.max(0) as u64).then_some(rng)
}

/// The column's growth: a pure function of `(seed, column, floor, bed)`.
pub fn plan(
    seed: u32,
    wx: i32,
    wz: i32,
    floor: i32,
    sea_level: i32,
    bed: &Bed,
    palette: Palette<'_>,
) -> Growth {
    let room = sea_level - 1 - floor - TOP_GAP;
    if room < 1 {
        return Growth::Bare;
    }
    if let (Some(field), true, false) = (&bed.reef, palette.reef, palette.corals.is_empty()) {
        if let Some(reef) = reef::plan(seed, wx, wz, room, field, palette.corals) {
            return Growth::Reef(reef);
        }
    }
    if let Some(field) = &bed.kelp {
        let (density, _) = field.densest(seed, wx, wz, |_| ());
        if let Some(mut rng) = rolls(seed, field.salt, wx, wz, density) {
            let longest = KELP_LEN.1.min(room);
            if longest >= KELP_LEN.0 {
                return Growth::Kelp {
                    len: rng.next_i32(KELP_LEN.0, longest),
                };
            }
        }
    }
    if let (Some(deep), true) = (&bed.glowpods, palette.glowpods > 0) {
        if sea_level - 1 - floor >= deep.min_depth {
            let kind = |rng: &mut GenRng| rng.next_i32(0, palette.glowpods as i32 - 1) as usize;
            let (density, colony) = deep.colonies.densest(seed, wx, wz, kind);
            if let (Some(colony), Some(_)) =
                (colony, rolls(seed, deep.colonies.salt, wx, wz, density))
            {
                return Growth::Glowpod {
                    kind: colony.traits,
                };
            }
        }
    }
    if let Some(field) = &bed.clams {
        let (density, _) = field.densest(seed, wx, wz, |_| ());
        if let Some(mut rng) = rolls(seed, field.salt, wx, wz, density) {
            return Growth::Clam {
                open: rng.next_u64() % 1000 < OPEN_CLAM_PER_MILLE,
                facing: (rng.next_u64() % 4) as u8,
            };
        }
    }
    if let Some(field) = &bed.seagrass {
        let (density, _) = field.densest(seed, wx, wz, |_| ());
        if rolls(seed, field.salt, wx, wz, density).is_some() {
            return Growth::Seagrass;
        }
    }
    Growth::Bare
}

/// Plants this section's share of every column's growth on the floor `carve`
/// left it, over water this section can see.
pub fn generate(ocean: &Ocean, ctx: &GenCtx, carve: &Carve, out: &mut GenOutput) {
    if !ctx.has_columns() {
        return;
    }
    let [ox, oy, oz] = ctx.origin_world();
    let palette = Palette {
        corals: &ocean.corals,
        glowpods: ocean.glowpods.len(),
        reef: ocean.reef.is_some(),
    };
    for wz in oz..oz + 16 {
        for wx in ox..ox + 16 {
            let (Some(biome), Some(floor)) = (ctx.biome(wx, wz), carve.floor(ctx, wx, wz)) else {
                continue;
            };
            if floor + MOST_GROWTH < oy || floor >= oy + 16 {
                continue;
            }
            let Some(bed) = ocean.bed(biome) else {
                continue;
            };
            if carve.water(ctx, ocean.water, [wx, floor, wz]) == Some(true)
                || ctx.block([wx, floor, wz]) == Some(BlockId::AIR)
            {
                continue;
            }
            let growth = plan(ctx.seed(), wx, wz, floor, ctx.sea_level(), bed, palette);
            emit(ocean, ctx, carve, [wx, floor, wz], growth, out);
        }
    }
}

fn emit(
    ocean: &Ocean,
    ctx: &GenCtx,
    carve: &Carve,
    floor: [i32; 3],
    growth: Growth,
    out: &mut GenOutput,
) {
    let cell = |dy: i32| [floor[0], floor[1] + dy, floor[2]];
    for dy in 1..=growth.height() {
        let block = match growth {
            Growth::Bare => return,
            Growth::Seagrass => ocean.seagrass,
            Growth::Kelp { .. } => ocean.kelp,
            Growth::Glowpod { kind } => ocean.glowpods[kind],
            Growth::Clam { open, .. } => ocean.clams[usize::from(open)],
            Growth::Reef(reef) => {
                let Some(blocks) = &ocean.reef else {
                    return;
                };
                let block = match reef {
                    Reef::Head(column) => reef::head_cell(column, dy, blocks, &ocean.corals),
                    Reef::Fringe(fringe) => reef::fringe_cell(fringe, blocks, &ocean.corals),
                };
                // Water under a cap stays water; a form the palette lacks grows nothing.
                let Some(block) = block else {
                    continue;
                };
                block
            }
        };
        match (carve.water(ctx, ocean.water, cell(dy)), growth) {
            (Some(true), Growth::Clam { facing, .. }) => {
                author_facing(out, cell(dy), block, facing)
            }
            (Some(true), _) => out.blocks.push((cell(dy), block)),
            (Some(false), _) => return,
            (None, _) => {}
        }
    }
}

/// `block` turned to `FACINGS[facing]`, sharing the output's palette entry for that pair.
fn author_facing(out: &mut GenOutput, pos: [i32; 3], block: BlockId, facing: u8) {
    let entry = AuthoredPalette::encode(block, [("facing", FACINGS[usize::from(facing)])]);
    let palette = &mut out.authored.palette;
    let shared = palette
        .iter()
        .position(|e| e.is_ok_and(|e| e.bytes == entry));
    let material = match shared {
        Some(i) => i as u32,
        None => palette.push_encoded(&entry),
    };
    out.authored.cells.push(pos, material as u16);
}

#[cfg(test)]
mod tests;
