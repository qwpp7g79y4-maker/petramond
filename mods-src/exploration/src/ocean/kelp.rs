//! Kelp behaviour. A stalk is one `run` row whose forms (holdfast, stem,
//! tip) the engine resolves from its neighbours, and which `stacks` on
//! itself. The pack adds two things: support goes through the engine's own
//! rule (`supported_at` / `break_naturally`, so a cut stalk unzips upward and
//! leaves its water behind), and the free tip grows into the water above.

use mod_sdk::*;

use super::Ocean;

/// Chance a random tick on a tip grows it (one random tick per cell ≈ 68 s).
const GROW_PER_MILLE: u64 = 330;
/// Longest stalk a tip grows into.
pub const MAX_LEN: i32 = 18;
/// Water kept above a grown tip, so a stalk never breaks the surface.
const HEADROOM: i32 = 2;

pub fn on_hook(ocean: &Ocean, kind: BlockHookKind, pos: [i32; 3]) {
    if kind == BlockHookKind::RandomTick && rng_u64("exploration:kelp") % 1000 < GROW_PER_MILLE {
        grow(ocean, pos);
    }
}

fn grow(ocean: &Ocean, pos: [i32; 3]) {
    let [x, y, z] = pos;
    let column: Vec<[i32; 3]> = (1..=HEADROOM + 1)
        .map(|dy| [x, y + dy, z])
        .chain((1..MAX_LEN).map(|dy| [x, y - dy, z]))
        .collect();
    let cells = get_blocks(column);
    let (above, below) = cells.split_at((HEADROOM + 1) as usize);
    if above.iter().any(|b| *b != Some(ocean.water)) {
        return;
    }
    let stalk = 1 + below.iter().take_while(|b| **b == Some(ocean.kelp)).count() as i32;
    if stalk >= MAX_LEN || below.get(stalk as usize - 1).is_some_and(Option::is_none) {
        return;
    }
    set_block([x, y + 1, z], ocean.kelp);
}

#[cfg(test)]
mod tests;
