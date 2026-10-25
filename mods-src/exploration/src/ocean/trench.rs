//! Trenches: winding canyons cut into the deep sea floor, where the light
//! never reaches. A trench follows the zero line of a slow noise field,
//! fades in and out along its length, and shallows out where the floor rises,
//! so it never cuts a coast.
//!
//! Depth is a pure function of `(seed, column, water depth)`; the flora in the
//! same dispatch plants on the carved floor through [`Carve::floor`]. A cave
//! the trench crosses stays dry: its air is never cut, and a cell that would
//! put water beside open air stays rock, so a trench never drains the sea into
//! a cave system.

use mod_sdk::build::Noise2;
use mod_sdk::*;

/// Water above the original floor before a trench can open.
const MIN_WATER: i32 = 22;
/// Water depth over which a trench grows to its full cut.
const RAMP: i32 = 10;
const MAX_CUT: f32 = 22.0;
/// Half-width of the trench in noise units around the ridge line.
const HALF_WIDTH: f32 = 0.11;
const RIDGE_SCALE: f32 = 1.0 / 95.0;
const FADE_SCALE: f32 = 1.0 / 340.0;
/// Fade noise below this closes the trench.
const FADE_OPEN: f32 = 0.05;

const SALT_RIDGE: u32 = 0x7E1C_4A01;
const SALT_FADE: u32 = 0x7E1C_4A02;

/// Blocks to cut below the original floor of a column with `water` water cells above it.
pub fn cut(seed: u32, wx: i32, wz: i32, water: i32) -> i32 {
    if water < MIN_WATER {
        return 0;
    }
    let (x, z) = (wx as f32, wz as f32);
    let fade = Noise2(seed ^ SALT_FADE).at(x * FADE_SCALE, z * FADE_SCALE);
    if fade < FADE_OPEN {
        return 0;
    }
    let ridge = Noise2(seed ^ SALT_RIDGE)
        .at(x * RIDGE_SCALE, z * RIDGE_SCALE)
        .abs();
    if ridge >= HALF_WIDTH {
        return 0;
    }
    let across = 1.0 - (ridge / HALF_WIDTH).powi(2);
    let along = ((fade - FADE_OPEN) / 0.25).clamp(0.0, 1.0);
    let ramp = ((water - MIN_WATER) as f32 / RAMP as f32).clamp(0.0, 1.0);
    (MAX_CUT * across * along * ramp).round() as i32
}

/// One section's carving: which cells became water and the floor each column ends on.
pub struct Carve {
    pub writes: Vec<GenWrite>,
    floors: Vec<i32>,
    carved: FxHashSet<[i32; 3]>,
}

impl Carve {
    /// The floor a column ends on after carving (its original floor where nothing was cut).
    pub fn floor(&self, ctx: &GenCtx, wx: i32, wz: i32) -> Option<i32> {
        let [ox, _, oz] = ctx.origin_world();
        let (lx, lz) = (wx - ox, wz - oz);
        ((0..16).contains(&lx) && (0..16).contains(&lz))
            .then(|| self.floors.get((lz * 16 + lx) as usize).copied())
            .flatten()
    }

    /// Whether this section's snapshot shows `cell` as water once carved.
    pub fn water(&self, ctx: &GenCtx, water: BlockId, cell: [i32; 3]) -> Option<bool> {
        if self.carved.contains(&cell) {
            return Some(true);
        }
        ctx.block(cell).map(|b| b == water)
    }
}

/// What a section reads of cells outside it: whether each is open air.
pub trait Beyond {
    fn air(&self, cells: Vec<[i32; 3]>) -> Option<Vec<bool>>;
}

pub struct HostBeyond;

impl Beyond for HostBeyond {
    fn air(&self, cells: Vec<[i32; 3]>) -> Option<Vec<bool>> {
        let want = cells.len();
        let spaces = paged(cells, terrain_space_at);
        (spaces.len() == want).then(|| spaces.iter().map(|s| *s == TerrainSpace::Air).collect())
    }
}

const FACES: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// Carves this section's share of every trench column, and the `worn` cells (cut without moving
/// any floor). `deep` says which biomes trenches open in; `bed_floor` re-skins the new floor.
pub fn carve(
    ctx: &GenCtx,
    water: BlockId,
    bed_floor: BlockId,
    deep: impl Fn(u8) -> bool,
    worn: &[[i32; 3]],
    beyond: &dyn Beyond,
) -> Carve {
    let [ox, oy, oz] = ctx.origin_world();
    let mut floors = vec![i32::MIN; 256];
    let mut cut_cells: Vec<[i32; 3]> = Vec::new();
    let mut new_floors: Vec<[i32; 3]> = Vec::new();
    for lz in 0..16 {
        for lx in 0..16 {
            let (wx, wz) = (ox + lx, oz + lz);
            let (Some(floor), Some(biome)) = (ctx.surface_y(wx, wz), ctx.biome(wx, wz)) else {
                continue;
            };
            floors[(lz * 16 + lx) as usize] = floor;
            if !deep(biome) {
                continue;
            }
            let depth = cut(ctx.seed(), wx, wz, ctx.sea_level() - 1 - floor);
            if depth == 0 {
                continue;
            }
            floors[(lz * 16 + lx) as usize] = floor - depth;
            for y in (floor - depth + 1).max(oy)..=floor.min(oy + 15) {
                if ctx.block([wx, y, wz]) != Some(BlockId::AIR) {
                    cut_cells.push([wx, y, wz]);
                }
            }
            let bottom = [wx, floor - depth, wz];
            if ctx.block(bottom).is_some() {
                new_floors.push(bottom);
            }
        }
    }
    let mut carved: FxHashSet<[i32; 3]> = cut_cells.iter().copied().collect();
    for &c in worn {
        if ctx
            .block(c)
            .is_some_and(|b| b != BlockId::AIR && b != water)
            && carved.insert(c)
        {
            cut_cells.push(c);
        }
    }
    let mut outside: Vec<[i32; 3]> = Vec::new();
    for c in &cut_cells {
        for f in FACES {
            let n = [c[0] + f[0], c[1] + f[1], c[2] + f[2]];
            if ctx.block(n).is_none() && !carved.contains(&n) {
                outside.push(n);
            }
        }
    }
    outside.sort_unstable();
    outside.dedup();
    let open_beyond: FxHashSet<[i32; 3]> = match beyond.air(outside.clone()) {
        Some(air) => outside
            .into_iter()
            .zip(air)
            .filter(|(_, a)| *a)
            .map(|(c, _)| c)
            .collect(),
        None => {
            return Carve {
                writes: Vec::new(),
                floors: (0..256)
                    .map(|i| ctx.surface_y(ox + i % 16, oz + i / 16).unwrap_or(i32::MIN))
                    .collect(),
                carved: FxHashSet::default(),
            }
        }
    };
    let beside_air = |c: &[i32; 3]| {
        FACES.iter().any(|f| {
            let n = [c[0] + f[0], c[1] + f[1], c[2] + f[2]];
            !carved.contains(&n) && (ctx.block(n) == Some(BlockId::AIR) || open_beyond.contains(&n))
        })
    };
    let sealed: FxHashSet<[i32; 3]> = cut_cells.iter().copied().filter(beside_air).collect();
    let mut writes: Vec<GenWrite> = cut_cells
        .iter()
        .filter(|c| !sealed.contains(*c))
        .map(|&c| (c, water))
        .collect();
    writes.extend(
        new_floors
            .into_iter()
            .filter(|c| ctx.block(*c) != Some(BlockId::AIR))
            .map(|c| (c, bed_floor)),
    );
    let carved = carved.difference(&sealed).copied().collect();
    Carve {
        writes,
        floors,
        carved,
    }
}

#[cfg(test)]
mod tests;
