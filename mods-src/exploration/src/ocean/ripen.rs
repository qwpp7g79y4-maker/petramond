//! Blocks that turn into another on their own: a row whose behaviour is
//! `exploration:ripens` carries the same key as data, `{"into": "<block>",
//! "per_mille": n}`. Each random tick rolls `n` in a thousand and turns the
//! cell into `into` in place, facing kept; a neighbour update drops it once
//! its support is gone.

use mod_sdk::*;
use serde::Deserialize;

use crate::keys;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRipen {
    into: String,
    per_mille: u16,
}

pub struct Ripening {
    rows: FxHashMap<BlockId, (BlockId, u64)>,
}

impl Ripening {
    pub fn resolve() -> Option<Ripening> {
        let rows: FxHashMap<BlockId, (BlockId, u64)> =
            blocks_with_data_as::<RawRipen>(keys::RIPENS)
                .into_iter()
                .filter_map(|(block, raw)| {
                    let into = resolve_block_logged(&raw.into)?;
                    Some((block, (into, u64::from(raw.per_mille.min(1000)))))
                })
                .collect();
        (!rows.is_empty()).then_some(Ripening { rows })
    }

    pub fn on_hook(&self, kind: BlockHookKind, pos: [i32; 3]) {
        match kind {
            BlockHookKind::NeighborUpdate => {
                if !supported_at(vec![pos]).first().copied().unwrap_or(true) {
                    break_naturally(pos);
                }
            }
            BlockHookKind::RandomTick => {
                let block = get_blocks(vec![pos]).first().copied().flatten();
                let Some(&(into, per_mille)) = block.and_then(|b| self.rows.get(&b)) else {
                    return;
                };
                if rng_u64(keys::RIPENS) % 1000 < per_mille {
                    swap_block(pos, into);
                }
            }
            BlockHookKind::ScheduledTick => {}
        }
    }
}
