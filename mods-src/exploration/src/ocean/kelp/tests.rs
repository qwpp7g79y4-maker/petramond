use super::*;
use mod_sdk::testing::MockHost;

const WATER: BlockId = BlockId(2);
const KELP: BlockId = BlockId(3);
const SAND: BlockId = BlockId(4);

fn ocean() -> Ocean {
    Ocean {
        water: WATER,
        trench_floor: SAND,
        kelp: KELP,
        seagrass: BlockId(5),
        corals: Vec::new(),
        glowpods: Vec::new(),
        reef: None,
        clams: [BlockId(41), BlockId(42)],
        beds: Vec::new(),
    }
}

/// A stalk of `len` kelp rooted on sand at y = 0 with water up to y = 40.
fn stalk(host: &MockHost, len: i32) {
    host.set_block([0, 0, 0], SAND);
    for y in 1..40 {
        host.set_block([0, y, 0], if y <= len { KELP } else { WATER });
    }
}

const GROW: u64 = 0;
const IDLE: u64 = 999;

#[test]
fn a_tip_grows_into_open_water_until_the_stalk_is_full_grown() {
    let host = MockHost::new();
    let _guard = host.install();
    stalk(&host, 3);
    host.queue_rng([GROW]);
    on_hook(&ocean(), BlockHookKind::RandomTick, [0, 3, 0]);
    assert_eq!(host.block([0, 4, 0]), Some(KELP));

    host.queue_rng([IDLE]);
    on_hook(&ocean(), BlockHookKind::RandomTick, [0, 4, 0]);
    assert_eq!(
        host.block([0, 5, 0]),
        Some(WATER),
        "an idle roll grows nothing"
    );

    stalk(&host, MAX_LEN);
    host.queue_rng([GROW]);
    on_hook(&ocean(), BlockHookKind::RandomTick, [0, MAX_LEN, 0]);
    assert_eq!(host.block([0, MAX_LEN + 1, 0]), Some(WATER), "full grown");
}

#[test]
fn a_tip_never_grows_toward_the_surface_or_from_mid_stalk() {
    let host = MockHost::new();
    let _guard = host.install();
    stalk(&host, 3);
    host.set_block([0, 6, 0], BlockId::AIR);
    host.queue_rng([GROW]);
    on_hook(&ocean(), BlockHookKind::RandomTick, [0, 3, 0]);
    assert_eq!(
        host.block([0, 4, 0]),
        Some(WATER),
        "the surface is too close"
    );

    stalk(&host, 3);
    host.queue_rng([GROW]);
    on_hook(&ocean(), BlockHookKind::RandomTick, [0, 2, 0]);
    assert_eq!(
        host.block([0, 3, 0]),
        Some(KELP),
        "a segment under the tip stays"
    );
    assert_eq!(host.block([0, 4, 0]), Some(WATER));
}
