use super::*;

const SEA: i32 = 63;
const BIOME: u8 = 200;
const STONE: BlockId = BlockId(1);
const WATER: BlockId = BlockId(2);
const KELP: BlockId = BlockId(3);
const SEAGRASS: BlockId = BlockId(4);
const CORAL: BlockId = BlockId(5);
const FAN: BlockId = BlockId(6);
const POD: BlockId = BlockId(7);
const CLAM: BlockId = BlockId(8);
const CLAM_OPEN: BlockId = BlockId(9);

const ROCK: BlockId = BlockId(10);
const BRAIN: BlockId = BlockId(11);
const STAGHORN: BlockId = BlockId(12);
const ANEMONE: BlockId = BlockId(13);
const LANTERN: BlockId = BlockId(14);
const REEF: super::reef::ReefBlocks = super::reef::ReefBlocks {
    rock: ROCK,
    staghorn: STAGHORN,
    anemone: ANEMONE,
    lantern: LANTERN,
};

const REEF_FIELD: ReefField = ReefField {
    lattice: 15,
    per_mille: 820,
    radius: (3, 7),
};

fn coral() -> Coral {
    let mut coral = Coral::default();
    coral.set(super::super::reef::Form::Block, Some(CORAL));
    coral.set(super::super::reef::Form::Fan, Some(FAN));
    coral.set(super::super::reef::Form::Brain, Some(BRAIN));
    coral
}

const EVERYWHERE: ColonyField = colonies(0xBEEF, 8, 1, (8, 8), 1000, 1000, 1000);

static KELP_EVERYWHERE: Bed = Bed {
    biome: "test:kelp",
    trench: false,
    reef: None,
    kelp: Some(EVERYWHERE),
    glowpods: None,
    clams: None,
    seagrass: None,
};

static CLAMS_EVERYWHERE: Bed = Bed {
    biome: "test:clams",
    trench: false,
    reef: None,
    kelp: None,
    glowpods: None,
    clams: Some(EVERYWHERE),
    seagrass: None,
};

static EVERY_BED: Bed = Bed {
    biome: "test:all",
    trench: false,
    reef: Some(REEF_FIELD),
    kelp: Some(colonies(2, 9, 1, (3, 6), 500, 100, 20)),
    glowpods: Some(Deep {
        colonies: colonies(3, 7, 1, (2, 5), 400, 50, 10),
        min_depth: 6,
    }),
    clams: Some(colonies(5, 8, 1, (2, 4), 500, 100, 10)),
    seagrass: Some(colonies(4, 5, 1, (2, 4), 600, 100, 50)),
};

fn ocean(bed: &'static Bed) -> Ocean {
    Ocean {
        water: WATER,
        trench_floor: STONE,
        kelp: KELP,
        seagrass: SEAGRASS,
        corals: vec![coral()],
        glowpods: vec![POD],
        reef: Some(REEF),
        clams: [CLAM, CLAM_OPEN],
        beds: vec![ResolvedBed { biome: BIOME, bed }],
    }
}

struct NoAir;

impl super::super::trench::Beyond for NoAir {
    fn air(&self, cells: Vec<[i32; 3]>) -> Option<Vec<bool>> {
        Some(vec![false; cells.len()])
    }
}

fn grow(ocean: &Ocean, ctx: &GenCtx) -> GenOutput {
    let carve = super::super::trench::carve(ctx, WATER, STONE, |_| false, &[], &NoAir);
    let mut out = GenOutput::from(carve.writes.clone());
    generate(ocean, ctx, &carve, &mut out);
    out
}

/// Every cell the output writes, plain or authored.
fn cells(out: &GenOutput) -> Vec<GenWrite> {
    let palette: Vec<BlockId> = out
        .authored
        .palette
        .iter()
        .map(|e| e.unwrap().block)
        .collect();
    let authored = out
        .authored
        .cells
        .iter()
        .map(|(c, m)| (c, palette[m as usize]));
    out.blocks.iter().copied().chain(authored).collect()
}

fn section(cy: i32, floor: i32) -> GenCtx {
    let blocks = (0..4096)
        .map(|i| {
            let y = cy * 16 + i / 256;
            if y <= floor {
                STONE.0
            } else if y < SEA {
                WATER.0
            } else {
                0
            }
        })
        .collect();
    GenCtx::for_test(
        [0, cy, 0],
        7,
        blocks,
        vec![floor; 256],
        vec![BIOME; 256],
        SEA,
    )
}

#[test]
fn growth_never_breaks_the_surface_or_fills_a_shallow_floor() {
    let corals = vec![coral(); 4];
    let palette = Palette {
        corals: &corals,
        glowpods: 2,
        reef: true,
    };
    for floor in 20..SEA {
        for wx in -40..40 {
            for wz in [-17, 3, 29] {
                let growth = plan(9, wx, wz, floor, SEA, &EVERY_BED, palette);
                assert!(
                    growth == Growth::Bare || floor + growth.height() <= SEA - 1 - TOP_GAP,
                    "{growth:?} on a floor at {floor} reaches the waterline"
                );
            }
        }
    }
}

#[test]
fn sections_assemble_one_seamless_stalk_per_column() {
    let floor = 40;
    let ocean = ocean(&KELP_EVERYWHERE);
    let mut writes: Vec<GenWrite> = Vec::new();
    for cy in [3, 2] {
        writes.extend(cells(&grow(&ocean, &section(cy, floor))));
    }
    let corals = vec![coral()];
    let palette = Palette {
        corals: &corals,
        glowpods: 1,
        reef: true,
    };
    for wz in 0..16 {
        for wx in 0..16 {
            let Growth::Kelp { len } = plan(7, wx, wz, floor, SEA, &KELP_EVERYWHERE, palette)
            else {
                panic!("the dense bed grows kelp in every column");
            };
            let mut ys: Vec<i32> = writes
                .iter()
                .filter(|(c, b)| c[0] == wx && c[2] == wz && *b == KELP)
                .map(|(c, _)| c[1])
                .collect();
            ys.sort_unstable();
            let want: Vec<i32> = (floor + 1..=floor + len).collect();
            assert_eq!(ys, want, "column {wx},{wz}: one stalk from the floor up");
        }
    }
}

#[test]
fn growth_writes_only_over_water() {
    let ocean = ocean(&EVERY_BED);
    let ctx = section(2, 40);
    for (c, _) in cells(&grow(&ocean, &ctx)) {
        assert_eq!(ctx.block(c), Some(WATER), "{c:?} was not open water");
    }
}

#[test]
fn clams_lie_every_way() {
    let out = grow(&ocean(&CLAMS_EVERYWHERE), &section(2, 40));
    assert!(
        out.blocks.iter().all(|&(_, b)| b != CLAM && b != CLAM_OPEN),
        "a clam is written with its facing"
    );
    let mut facings = std::collections::BTreeSet::new();
    for entry in out.authored.palette.iter() {
        let entry = entry.unwrap();
        assert!(entry.block == CLAM || entry.block == CLAM_OPEN);
        facings.extend(entry.state().map(|(_, facing)| facing.to_owned()));
    }
    assert_eq!(facings.len(), 4, "{facings:?}");
}
