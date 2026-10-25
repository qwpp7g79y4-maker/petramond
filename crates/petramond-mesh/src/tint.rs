use petramond_world::biome::{Biome, TINT_HALO_RADIUS};
use petramond_world::chunk::{CHUNK_SX, CHUNK_SZ};
use petramond_world::tile::TileTint;

const COLUMNS: usize = CHUNK_SX * CHUNK_SZ;
const HALO: usize = TINT_HALO_RADIUS;
/// Grass and foliage blend over 5x5 columns. Water blends over the whole halo, so the seas of
/// neighbouring biomes shade into each other instead of meeting at a line.
const LAND_BLEND_RADIUS: usize = 2;
const WATER_BLEND_RADIUS: usize = HALO;
const BIOME_PAD_X: usize = CHUNK_SX + HALO * 2;
const BIOME_PAD_Z: usize = CHUNK_SZ + HALO * 2;
const BIOME_PAD_AREA: usize = BIOME_PAD_X * BIOME_PAD_Z;
const SUM_X: usize = BIOME_PAD_X + 1;
const SUM_Z: usize = BIOME_PAD_Z + 1;
const SUM_CHANNELS: usize = 9;

pub(super) const NO_TINT: [f32; 3] = [1.0, 1.0, 1.0];

pub(super) struct BiomeTints {
    pub grass: [[f32; 3]; COLUMNS],
    pub foliage: [[f32; 3]; COLUMNS],
    pub water: [[f32; 3]; COLUMNS],
}

impl BiomeTints {
    #[inline]
    fn filled(biome: Biome) -> Self {
        Self {
            grass: [biome.grass_color(); COLUMNS],
            foliage: [biome.foliage_color(); COLUMNS],
            water: [biome.water_color(); COLUMNS],
        }
    }

    #[inline]
    pub(super) fn tile(&self, kind: Option<TileTint>, ci: usize) -> [f32; 3] {
        match kind {
            Some(TileTint::Grass) => self.grass[ci],
            Some(TileTint::Foliage) => self.foliage[ci],
            Some(TileTint::Water) => self.water[ci],
            Some(TileTint::Fixed(rgb)) => rgb.map(|c| f32::from(c) / 255.0),
            None => NO_TINT,
        }
    }
}

pub(super) fn biome_window(
    ox: i32,
    oz: i32,
    neighbour_biome: impl Fn(i32, i32) -> u8,
) -> BiomeTints {
    let mut ids = [0u8; BIOME_PAD_AREA];
    let mut first = 0u8;
    let mut uniform = true;
    for z in 0..BIOME_PAD_Z {
        for x in 0..BIOME_PAD_X {
            let id = neighbour_biome(ox + x as i32 - HALO as i32, oz + z as i32 - HALO as i32);
            if x == 0 && z == 0 {
                first = id;
            } else if id != first {
                uniform = false;
            }
            ids[z * BIOME_PAD_X + x] = id;
        }
    }
    if uniform {
        return BiomeTints::filled(Biome::from_id(first));
    }

    let mut sums = [[[0f32; SUM_CHANNELS]; SUM_X]; SUM_Z];
    for z in 0..BIOME_PAD_Z {
        for x in 0..BIOME_PAD_X {
            let b = Biome::from_id(ids[z * BIOME_PAD_X + x]);
            let bg = b.grass_color();
            let bf = b.foliage_color();
            let bw = b.water_color();
            let v = [
                bg[0], bg[1], bg[2], bf[0], bf[1], bf[2], bw[0], bw[1], bw[2],
            ];
            for c in 0..SUM_CHANNELS {
                sums[z + 1][x + 1][c] =
                    v[c] + sums[z][x + 1][c] + sums[z + 1][x][c] - sums[z][x][c];
            }
        }
    }

    // The mean of channels `c` over the square of `radius` columns around column (x, z).
    let mean = |x: usize, z: usize, radius: usize, c: std::ops::Range<usize>| -> [f32; 3] {
        let (x0, z0) = (x + HALO - radius, z + HALO - radius);
        let (x1, z1) = (x0 + 2 * radius + 1, z0 + 2 * radius + 1);
        let area = ((2 * radius + 1) * (2 * radius + 1)) as f32;
        let mut out = [0f32; 3];
        for (o, c) in out.iter_mut().zip(c) {
            *o = (sums[z1][x1][c] + sums[z0][x0][c] - sums[z0][x1][c] - sums[z1][x0][c]) / area;
        }
        out
    };
    let mut grass = [[0f32; 3]; COLUMNS];
    let mut foliage = [[0f32; 3]; COLUMNS];
    let mut water = [[0f32; 3]; COLUMNS];
    for z in 0..CHUNK_SZ {
        for x in 0..CHUNK_SX {
            let i = z * CHUNK_SX + x;
            grass[i] = mean(x, z, LAND_BLEND_RADIUS, 0..3);
            foliage[i] = mean(x, z, LAND_BLEND_RADIUS, 3..6);
            water[i] = mean(x, z, WATER_BLEND_RADIUS, 6..9);
        }
    }
    BiomeTints {
        grass,
        foliage,
        water,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_window(ox: i32, oz: i32, neighbour_biome: impl Fn(i32, i32) -> u8) -> BiomeTints {
        let mean = |wx: i32, wz: i32, radius: usize, color: fn(Biome) -> [f32; 3]| {
            let r = radius as i32;
            let n = ((2 * r + 1) * (2 * r + 1)) as f32;
            let mut sum = [0f32; 3];
            for dz in -r..=r {
                for dx in -r..=r {
                    let c = color(Biome::from_id(neighbour_biome(wx + dx, wz + dz)));
                    for i in 0..3 {
                        sum[i] += c[i];
                    }
                }
            }
            sum.map(|v| v / n)
        };
        let mut grass = [[0f32; 3]; COLUMNS];
        let mut foliage = [[0f32; 3]; COLUMNS];
        let mut water = [[0f32; 3]; COLUMNS];
        for z in 0..CHUNK_SZ {
            for x in 0..CHUNK_SX {
                let wx = ox + x as i32;
                let wz = oz + z as i32;
                let i = z * CHUNK_SX + x;
                grass[i] = mean(wx, wz, LAND_BLEND_RADIUS, Biome::grass_color);
                foliage[i] = mean(wx, wz, LAND_BLEND_RADIUS, Biome::foliage_color);
                water[i] = mean(wx, wz, WATER_BLEND_RADIUS, Biome::water_color);
            }
        }
        BiomeTints {
            grass,
            foliage,
            water,
        }
    }

    fn assert_close(a: [f32; 3], b: [f32; 3]) {
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 0.0001, "{a:?} != {b:?}");
        }
    }

    #[test]
    fn summed_area_tint_matches_reference_window() {
        let biome = |wx: i32, wz: i32| -> u8 {
            match (wx.div_euclid(3) + wz.div_euclid(5)).rem_euclid(5) {
                0 => Biome::PLAINS.id(),
                1 => Biome::FOREST.id(),
                2 => Biome::SWAMP.id(),
                3 => Biome::DESERT.id(),
                _ => Biome::TAIGA.id(),
            }
        };

        let fast = biome_window(-16, 32, biome);
        let reference = reference_window(-16, 32, biome);
        for i in 0..COLUMNS {
            assert_close(fast.grass[i], reference.grass[i]);
            assert_close(fast.foliage[i], reference.foliage[i]);
            assert_close(fast.water[i], reference.water[i]);
        }
    }
}
