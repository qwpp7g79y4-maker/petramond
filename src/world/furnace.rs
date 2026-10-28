//! The furnace: a native behavior built from the same pieces a pack machine uses. Its slots are
//! the cell's container (laid out by its GUI document), its counters live in the cell's data, it
//! runs on scheduled ticks while it has work and wakes when its container changes, swaps between
//! its idle and burning rows (row data `petramond:furnace: {idle, burning}`), and publishes its
//! gauges to whoever is viewing it.

use crate::world::ServerWorld;
use petramond_math::math::IVec3;
use petramond_world::block::Block;
use petramond_world::furnace::{Furnace, FURNACE_SLOTS, KV_KEY};
use petramond_world::gui_state::GuiValue;

pub struct FurnaceBehavior;

impl crate::world::engine_behavior::EngineBlockBehavior for FurnaceBehavior {
    fn container_changed(&self, world: &mut ServerWorld, pos: IVec3) {
        world.schedule_tick(pos, 1);
    }

    fn scheduled_tick(&self, world: &mut ServerWorld, pos: IVec3) {
        world.step_furnace(pos);
    }

    fn viewers_changed(&self, world: &mut ServerWorld, pos: IVec3) {
        let state = world.furnace_state(pos);
        world.publish_gui_values(pos, gauges(state));
    }
}

pub static FURNACE: FurnaceBehavior = FurnaceBehavior;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRows {
    idle: String,
    burning: String,
}

/// Each block's (idle, burning) rows, from its `petramond:furnace` row data.
static ROWS: petramond_world::content::Slot<Vec<Option<(Block, Block)>>> =
    petramond_world::content::Slot::new(
        "furnace rows",
        &[petramond_world::content::stage::BLOCKS],
        |_| Ok(furnace_rows()),
    );

fn furnace_rows() -> Vec<Option<(Block, Block)>> {
    Block::all()
        .iter()
        .map(|b| {
            let raw: RawRows = serde_json::from_str(b.data_value(KV_KEY)?)
                .map_err(|e| log::warn!("{KV_KEY} on {b:?}: {e}"))
                .ok()?;
            let named = |name: &str| {
                petramond_world::registry::names()
                    .blocks
                    .id(name)
                    .map(Block::from_id)
            };
            Some((named(&raw.idle)?, named(&raw.burning)?))
        })
        .collect()
}

fn rows(block: Block) -> Option<(Block, Block)> {
    ROWS.current().get(block.id() as usize).copied().flatten()
}

fn gauges(state: Furnace) -> Vec<(String, GuiValue)> {
    let burn = if state.burn_max == 0 {
        0.0
    } else {
        f32::from(state.burn_remaining) / f32::from(state.burn_max)
    };
    let cook = f32::from(state.cook_progress) / f32::from(petramond_world::furnace::COOK_TICKS);
    vec![
        ("cook01".to_owned(), GuiValue::F32(cook)),
        ("burn01".to_owned(), GuiValue::F32(burn)),
    ]
}

impl ServerWorld {
    fn furnace_state(&self, pos: IVec3) -> Furnace {
        self.data
            .cell_kv_get(pos.x, pos.y, pos.z, KV_KEY)
            .and_then(Furnace::from_bytes)
            .unwrap_or_default()
    }

    fn step_furnace(&mut self, pos: IVec3) {
        let block = Block::from_id(self.data.chunk_block(pos.x, pos.y, pos.z));
        let Some((idle, burning)) = rows(block) else {
            return;
        };
        let Some(container) = self.container_at(pos) else {
            return;
        };
        if container.slots.len() < FURNACE_SLOTS {
            return;
        }
        let mut slots = container.slots.clone();
        let mut state = self.furnace_state(pos);
        let recipes = self.recipes();
        let before = state;
        if state.tick(&mut slots, |item| recipes.smelt(item)) {
            if let Some(container) = self.container_at_mut(pos) {
                container.slots = slots;
            }
            if state != before {
                self.cell_kv_set(pos.x, pos.y, pos.z, KV_KEY.to_owned(), state.to_bytes());
            }
        }
        let desired = if state.is_lit() { burning } else { idle };
        if desired != block {
            self.swap_block(pos, desired);
        }
        self.publish_gui_values(pos, gauges(state));
        if state.is_lit() || state.cook_progress > 0 {
            self.schedule_tick(pos, 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use petramond_world::chunk::{Chunk, ChunkPos};
    use petramond_world::crafting::{ProcessingRecipe, Recipes, SMELTING_CLASS};
    use petramond_world::furnace::{SLOT_FUEL, SLOT_INPUT, SLOT_OUTPUT};
    use petramond_world::item::{ItemStack, ItemType};

    fn world_with_furnace() -> (ServerWorld, IVec3) {
        let mut w = ServerWorld::new(1, 4);
        w.insert_chunk_for_test(ChunkPos::new(0, 0), Chunk::new(0, 0));
        w.set_recipes(std::sync::Arc::new(Recipes::new(
            Vec::new(),
            vec![ProcessingRecipe {
                key: "petramond:test_smelt".to_owned(),
                class: SMELTING_CLASS.to_owned(),
                input: ItemType::RawIron,
                result: ItemStack::new(ItemType::IronIngot, 1),
            }],
        )));
        let pos = IVec3::new(8, 64, 8);
        assert!(w.set_block_world(pos.x, pos.y, pos.z, Block::Furnace));
        assert!(w.ensure_container(pos, FURNACE_SLOTS));
        (w, pos)
    }

    fn block(w: &ServerWorld, p: IVec3) -> Block {
        Block::from_id(w.data.chunk_block(p.x, p.y, p.z))
    }

    #[test]
    fn a_filled_furnace_wakes_lights_smelts_and_goes_out() {
        let (mut w, pos) = world_with_furnace();
        {
            let slots = &mut w.container_at_mut(pos).unwrap().slots;
            slots[SLOT_INPUT] = Some(ItemStack::new(ItemType::RawIron, 1));
            slots[SLOT_FUEL] = Some(ItemStack::new(ItemType::Coal, 1));
        }
        w.game_tick();
        w.game_tick();
        assert_eq!(block(&w, pos), Block::FurnaceLit, "the fuel lit it");
        for _ in 0..petramond_world::furnace::COOK_TICKS {
            w.game_tick();
        }
        assert_eq!(
            w.container_at(pos).unwrap().slots[SLOT_OUTPUT],
            Some(ItemStack::new(ItemType::IronIngot, 1))
        );
        for _ in 0..ItemType::Coal.fuel_burn_ticks() {
            w.game_tick();
        }
        w.game_tick();
        assert_eq!(block(&w, pos), Block::Furnace, "burnt out, back to idle");
        assert!(
            !w.data.sim.scheduled.contains(pos),
            "an idle furnace stops ticking"
        );
    }
}
