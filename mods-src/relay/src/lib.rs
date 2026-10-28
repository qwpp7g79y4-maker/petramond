//! Relay mod: a block that carries items from the container above it into the container below it.
//!
//! The block is plain pack data plus the `relay:relay` behavior. Every relay row carries a
//! `relay:transfer` data row (`interval` ticks between moves, `stack` items per move), so a pack
//! adds a faster or slower relay by adding a block row, with no code change.
//!
//! A relay steps on its scheduled tick and schedules the next one, so it needs no world sweep.
//! Placing one starts it; a random tick restarts one whose pending tick was lost. The move is one
//! `container_transfer`, which the engine applies atomically against the target's admission rules:
//! a full or picky container just leaves the items where they were.

use mod_sdk::*;

mod keys {
    mod_sdk::pack_keys! {
        pub RELAY_BEHAVIOR: Behavior = "relay:relay";
        pub RELAY_BLOCK: Block = "relay:relay";
        pub RELAY_ITEM: Item = "relay:relay";
        pub RELAY_RECIPE: Recipe = "relay:relay";
    }
}

const RELAY_HOOK: u32 = 1;
const ON_BLOCK_PLACED: u32 = 1;
const TRANSFER_KEY: &str = "relay:transfer";

#[derive(serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Transfer {
    /// Ticks between two moves.
    interval: u64,
    /// Most items one move carries.
    stack: u8,
}

/// The slot of the first non-empty stack, with how many items of it one move takes.
fn next_move(slots: &[Option<ItemStackData>], stack: u8) -> Option<(u32, u8)> {
    slots.iter().enumerate().find_map(|(slot, s)| {
        let s = s.as_ref().filter(|s| s.count > 0)?;
        Some((slot as u32, s.count.min(stack)))
    })
}

#[derive(Default)]
struct Relay {
    rows: Vec<(BlockId, Transfer)>,
}

impl Relay {
    fn row(&self, block: BlockId) -> Option<Transfer> {
        self.rows.iter().find(|(b, _)| *b == block).map(|(_, t)| *t)
    }

    fn step(&self, pos: [i32; 3], transfer: Transfer) {
        let [x, y, z] = pos;
        let (above, below) = ([x, y + 1, z], [x, y - 1, z]);
        let ends = container_get_many(vec![above.into(), below.into()]);
        if let [Some(source), Some(_)] = ends.as_slice() {
            if let Some((slot, count)) = next_move(source, transfer.stack) {
                container_transfer(above.into(), slot, below.into(), count);
            }
        }
        schedule_tick(pos, transfer.interval);
    }
}

impl Mod for Relay {
    fn init(&mut self) {
        self.rows = blocks_with_data_as::<Transfer>(TRANSFER_KEY)
            .into_iter()
            .filter(|(_, t)| {
                let ok = t.interval > 0 && t.stack > 0;
                if !ok {
                    log("relay: a relay row needs interval > 0 and stack > 0; skipped");
                }
                ok
            })
            .collect();
        if self.rows.is_empty() {
            log("relay: no relay blocks registered; the mod stays idle");
            return;
        }
        register_block_behavior(keys::RELAY_BEHAVIOR, RELAY_HOOK);
        register_event_handler(EventKind::BlockPlaced, 0, ON_BLOCK_PLACED);
    }

    fn handle_event(&mut self, handler_id: u32, payload: &mut EventPayload) -> Outcome {
        if let (ON_BLOCK_PLACED, EventPayload::BlockPlaced { pos, block }) = (handler_id, &*payload)
        {
            if let Some(transfer) = self.row(*block) {
                schedule_tick(*pos, transfer.interval);
            }
        }
        Outcome::Continue
    }

    fn block_hooks(&mut self, callback_id: u32, hooks: &[(BlockHookKind, [i32; 3])]) {
        if callback_id != RELAY_HOOK {
            return;
        }
        let cells = get_blocks(hooks.iter().map(|(_, pos)| *pos).collect());
        for ((kind, pos), block) in hooks.iter().zip(cells) {
            let Some(transfer) = block.and_then(|b| self.row(b)) else {
                continue;
            };
            match kind {
                BlockHookKind::ScheduledTick => self.step(*pos, transfer),
                BlockHookKind::RandomTick => schedule_tick(*pos, transfer.interval),
                _ => {}
            }
        }
    }
}

register_mod!(Relay);

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(count: u8) -> Option<ItemStackData> {
        Some(ItemStackData {
            item: "petramond:cobblestone".into(),
            count,
            data: Vec::new(),
        })
    }

    #[test]
    fn every_declared_pack_id_is_shipped() {
        pack_check::assert_declared(&[keys::PACK_KEYS]);
    }

    #[test]
    fn shipped_relay_rows_parse() {
        let rows = pack_rows_with_data(include_str!("../pack/blocks.json"), "blocks", TRANSFER_KEY);
        assert!(!rows.is_empty());
        for (block, raw) in &rows {
            let t = parse_row_data::<Transfer>(raw).unwrap_or_else(|e| panic!("{block}: {e}"));
            assert!(t.interval > 0 && t.stack > 0, "{block}");
        }
    }

    #[test]
    fn a_move_takes_the_first_stack_capped_at_the_row() {
        assert_eq!(next_move(&[None, stack(40), stack(3)], 16), Some((1, 16)));
        assert_eq!(next_move(&[None, stack(5)], 16), Some((1, 5)));
    }

    #[test]
    fn an_empty_container_moves_nothing() {
        assert_eq!(next_move(&[], 16), None);
        assert_eq!(next_move(&[None, None], 16), None);
        assert_eq!(next_move(&[stack(0)], 16), None);
    }
}
