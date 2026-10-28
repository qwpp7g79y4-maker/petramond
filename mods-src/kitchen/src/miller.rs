//! Hand mill grinds one input into its `kitchen:milling` product over [`MILL_TICKS`]. No fuel, no
//! heat, just time.
//!
//! Same split as the oven. Machine, GUI doc, recipe CLASS: kitchen owns these. Grain and flour:
//! whoever ships them owns those. Farming just adds `wheat -> flour` as a data row plus the
//! `kitchen:millable` tag. No code needed on that end.
//!
//! Progress: section cell KV at the anchor, u32 LE. Drops to zero if there's no valid job (nothing
//! millable, output blocked); a mill has no heat to lose anyway. The flour cube shows when OUTPUT
//! has anything: the step asks for `kitchen:miller` or `kitchen:miller_full` from what OUTPUT
//! holds, so the row self-heals instead of tracking transitions.

use mod_sdk::*;

use machine_core::{
    consume_one, merge_output, output_accepts, Caches, Machine, MachineSpec, Presentation, StepCtx,
};

use crate::keys;

const STATE_KEY: &str = "kitchen:mill_state";

const SLOT_INPUT: usize = 0;
const SLOT_OUTPUT: usize = 1;

const MILL_TICKS: u32 = 200;

pub type Miller = Machine<MillerSpec>;

#[derive(Default)]
pub struct MillerSpec;

impl MachineSpec for MillerSpec {
    const BLOCK_KEY: &'static str = keys::MILLER_BLOCK;
    const VARIANT_KEYS: &'static [&'static str] = &[keys::MILLER_FULL_BLOCK];
    const STATE_KEY: &'static str = STATE_KEY;

    fn step(
        &mut self,
        ctx: &StepCtx<'_>,
        caches: &mut Caches,
        slots: &mut Vec<Option<ItemStackData>>,
        stored: &mut Vec<u8>,
        out: &mut Presentation,
    ) {
        slots.resize(2, None);
        let mut progress = ByteReader::new(stored).u32().unwrap_or(0);
        let before_progress = progress;

        let result = slots[SLOT_INPUT]
            .as_ref()
            .filter(|s| s.count > 0)
            .map(|s| s.item.clone())
            .and_then(|k| caches.recipe_for(keys::MILLING_CLASS, &k));
        let can_mill = result
            .as_ref()
            .is_some_and(|r| output_accepts(caches, &slots[SLOT_OUTPUT], r));

        if can_mill {
            progress += 1;
            if progress >= MILL_TICKS {
                progress = 0;
                let result = result.expect("can_mill implies a result");
                merge_output(&mut slots[SLOT_OUTPUT], &result);
                consume_one(&mut slots[SLOT_INPUT]);
            }
        } else {
            progress = 0;
        }

        if progress != before_progress {
            *stored = progress.to_le_bytes().to_vec();
        }
        if slots[SLOT_OUTPUT].is_some() {
            out.row(ctx.pos, ctx.variant_or_base(0));
        } else {
            out.row(ctx.pos, ctx.block);
        }
        if ctx.gui_open() {
            ctx.publish(
                keys::MILL01,
                GuiValue::F32(progress as f32 / MILL_TICKS as f32),
            );
        }
    }
}
