use super::*;

const SEA: i32 = 63;
const DEEP: u8 = 9;
const STONE: BlockId = BlockId(1);
const WATER: BlockId = BlockId(2);
const GRAVEL: BlockId = BlockId(3);
const SEED: u32 = 21;
/// Deep enough water for a full cut.
const FLOOR: i32 = SEA - 1 - (MIN_WATER + RAMP);

struct Open(FxHashSet<[i32; 3]>);

impl Beyond for Open {
    fn air(&self, cells: Vec<[i32; 3]>) -> Option<Vec<bool>> {
        Some(cells.iter().map(|c| self.0.contains(c)).collect())
    }
}

/// The section at `at` over a flat floor, with `air` cells hollowed out.
fn section(at: [i32; 3], air: &[[i32; 3]]) -> GenCtx {
    let blocks = (0..4096)
        .map(|i| {
            let c = [
                at[0] * 16 + i % 16,
                at[1] * 16 + i / 256,
                at[2] * 16 + (i / 16) % 16,
            ];
            if air.contains(&c) {
                0
            } else if c[1] <= FLOOR {
                STONE.0
            } else if c[1] < SEA {
                WATER.0
            } else {
                0
            }
        })
        .collect();
    GenCtx::for_test(at, SEED, blocks, vec![FLOOR; 256], vec![DEEP; 256], SEA)
}

/// A section column where some trench cuts deep.
fn trench_section() -> [i32; 2] {
    (0..3600)
        .map(|i| [i % 60 - 30, i / 60 - 30])
        .find(|s| {
            (0..256).any(|c| {
                cut(
                    SEED,
                    s[0] * 16 + c % 16,
                    s[1] * 16 + c / 16,
                    SEA - 1 - FLOOR,
                ) > 8
            })
        })
        .expect("a trench crosses the sampled sea floor")
}

#[test]
fn shallow_water_is_never_cut() {
    for x in -200..200 {
        assert_eq!(cut(SEED, x, x / 3, MIN_WATER - 1), 0);
    }
}

#[test]
fn stacked_sections_cut_each_trench_column_whole() {
    let [sx, sz] = trench_section();
    let mut cells: Vec<[i32; 3]> = Vec::new();
    for cy in -1..=3 {
        let ctx = section([sx, cy, sz], &[]);
        let carve = carve(
            &ctx,
            WATER,
            GRAVEL,
            |b| b == DEEP,
            &[],
            &Open(FxHashSet::default()),
        );
        cells.extend(
            carve
                .writes
                .iter()
                .filter(|(_, b)| *b == WATER)
                .map(|(c, _)| *c),
        );
    }
    for c in 0..256 {
        let (wx, wz) = (sx * 16 + c % 16, sz * 16 + c / 16);
        let depth = cut(SEED, wx, wz, SEA - 1 - FLOOR);
        let mut ys: Vec<i32> = cells
            .iter()
            .filter(|p| p[0] == wx && p[2] == wz)
            .map(|p| p[1])
            .collect();
        ys.sort_unstable();
        let want: Vec<i32> = (FLOOR - depth + 1..=FLOOR).collect();
        assert_eq!(ys, want, "column {wx},{wz}");
    }
}

#[test]
fn a_trench_never_opens_water_beside_a_cave() {
    let [sx, sz] = trench_section();
    let cy = (FLOOR - 4).div_euclid(16);
    let probe = section([sx, cy, sz], &[]);
    let base = [sx * 16, cy * 16, sz * 16];
    let first = carve(
        &probe,
        WATER,
        GRAVEL,
        |b| b == DEEP,
        &[],
        &Open(FxHashSet::default()),
    );
    let (wet, _) = *first
        .writes
        .iter()
        .find(|(c, b)| *b == WATER && c[1] < FLOOR && c[0] > base[0] && c[0] < base[0] + 15)
        .expect("the trench cuts below the floor here");
    let cave_inside = [wet[0] + 1, wet[1], wet[2]];
    let cave_beyond = [base[0] - 1, wet[1], wet[2]];
    let ctx = section([sx, cy, sz], &[cave_inside]);
    let beyond = Open([cave_beyond].into_iter().collect());
    let carved = carve(&ctx, WATER, GRAVEL, |b| b == DEEP, &[], &beyond);
    assert!(
        !carved.writes.is_empty(),
        "the rest of the trench still cuts"
    );
    for (c, b) in &carved.writes {
        if *b != WATER {
            continue;
        }
        assert_ne!(*c, cave_inside, "the cave itself is flooded");
        for f in FACES {
            let n = [c[0] + f[0], c[1] + f[1], c[2] + f[2]];
            assert!(
                n != cave_inside && n != cave_beyond,
                "{c:?} opens water onto the cave at {n:?}"
            );
        }
    }
}

/// How much deep floor the trenches cut, for tuning (`--ignored --nocapture`).
#[test]
#[ignore]
fn zz_trench_census() {
    for seed in [1u32, 7, 21, 42] {
        let (mut any, mut deep, mut n) = (0, 0, 0);
        for z in (-2048..2048).step_by(4) {
            for x in (-2048..2048).step_by(4) {
                n += 1;
                let c = cut(seed, x, z, 40);
                any += i32::from(c > 0);
                deep += i32::from(c > 8);
            }
        }
        let share = |k: i32| 100.0 * k as f64 / n as f64;
        eprintln!(
            "seed {seed}: cut {:.2}% deep {:.2}%",
            share(any),
            share(deep)
        );
    }
}
