//! Kitchen mod: craftable food machines on the mod container-slot API, one module per machine.
//!
//! - [`oven`]: like the furnace but runs its own `kitchen:cooking` recipe class, so it never smelts
//!   ore. Flips to `kitchen:oven_lit` row while burning (fire cube, glow, particles).
//! - [`vessels`]: bowl left behind after eating, hooked into `ItemUseEvent::Eaten`.
//! - [`oven_draw`]: draws the oven's input and output items each tick via `set_block_draw`.
//! - [`miller`]: grinds input into `kitchen:milling` product every 200 ticks, no fuel. Flips to
//!   `kitchen:miller_full` row while there's output waiting.
//!
//! Both are `MachineSpec`s over `machine-core`, which steps a machine when its block behavior
//! wakes, reschedules it while it works, and owns the session caches and slot arithmetic.
//!
//! This crate is only the machine logic; blocks/items/recipes/models/GUI are pack data. The recipe
//! classes are the extension point: a pack adds `kitchen:cooking`/`kitchen:milling` rows plus the
//! matching slot-filter tags, no code change needed here or in the engine. Farming pack's
//! dough→bread and wheat→flour recipes work this way.
//!
//! Each machine row lists its kind's behavior (`kitchen:oven`, `kitchen:miller`); the tick's hooks
//! for a kind arrive together and are stepped through batched calls, writing back only what
//! changed.

mod keys;
mod miller;
mod oven;
mod oven_draw;
mod vessels;

use machine_core::Caches;
use miller::Miller;
use mod_sdk::*;
use oven::Oven;
use vessels::Vessels;

const OVEN_HOOK: u32 = 1;
const MILLER_HOOK: u32 = 2;
const ON_BLOCK_BROKEN: u32 = 1;
const ON_ITEM_USED: u32 = 3;

#[derive(Default)]
struct Kitchen {
    oven: Oven,
    miller: Miller,
    caches: Caches,
    vessels: Vessels,
}

impl Mod for Kitchen {
    fn init(&mut self) {
        let oven_ok = self.oven.init();
        let miller_ok = self.miller.init();
        if !oven_ok && !miller_ok {
            log("kitchen: no machine blocks registered; the mod stays idle");
            return;
        }
        self.vessels.init();
        if oven_ok {
            register_block_behavior(keys::OVEN_BEHAVIOR, OVEN_HOOK);
        }
        if miller_ok {
            register_block_behavior(keys::MILLER_BEHAVIOR, MILLER_HOOK);
        }
        register_event_handler(EventKind::BlockBroken, 0, ON_BLOCK_BROKEN);
        register_event_handler(EventKind::ItemUsed, 0, ON_ITEM_USED);
    }

    fn handle_event(&mut self, handler_id: u32, payload: &mut EventPayload) -> Outcome {
        match (handler_id, &*payload) {
            (
                ON_BLOCK_BROKEN,
                EventPayload::BlockBroken {
                    pos,
                    block,
                    group,
                    data,
                    ..
                },
            ) => {
                self.oven.on_broken(*pos, *block, group.as_ref(), data);
                self.miller.on_broken(*pos, *block, group.as_ref(), data);
            }
            (ON_ITEM_USED, EventPayload::ItemUsed { player, item, kind }) => {
                self.vessels.on_item_used(*player, *item, *kind)
            }
            _ => {}
        }
        Outcome::Continue
    }

    fn block_hooks(&mut self, callback_id: u32, hooks: &[(BlockHookKind, [i32; 3])]) {
        match callback_id {
            OVEN_HOOK => self.oven.on_hooks(&mut self.caches, hooks),
            MILLER_HOOK => self.miller.on_hooks(&mut self.caches, hooks),
            _ => {}
        }
    }
}

register_mod!(Kitchen);
