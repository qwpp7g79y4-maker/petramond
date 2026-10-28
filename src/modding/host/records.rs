//! [`CellRecord`]: the whole cell by name, in and out of the world's resolved form.

use mod_api::{CellRecord, RecordPlan, RecordStatus};
use petramond_math::math::IVec3;
use petramond_world::construction::{Plan, Record};
use petramond_world::container::Container;
use petramond_world::item::ItemStack;

use super::guards::{intern_abi_data, item_by_name, item_stack_data};
use crate::schematic::CellData;
use crate::world::cells::ResolvedCell;
use crate::world::construction::CellStatus;
use crate::world::ServerWorld;

fn cell_data(record: &CellRecord) -> CellData {
    CellData {
        block: record.block.clone(),
        state: record.state.clone(),
        state_ids: record.refs.iter().cloned().collect(),
        fluid: record.fluid,
        kv: record.data.iter().cloned().collect(),
        container: None,
    }
}

/// The record resolved against this session's registries: unknown names, malformed state or an
/// invalid stack refuse it.
pub(in crate::modding) fn record_in(record: &CellRecord) -> Result<ResolvedCell, String> {
    let mut cell = cell_data(record).resolve()?;
    if let Some(slots) = &record.container {
        if slots.len() > petramond_world::container::MAX_CONTAINER_SLOTS {
            return Err("Container has too many slots".into());
        }
        let slots = slots
            .iter()
            .map(|slot| slot.as_ref().map(stack_in).transpose())
            .collect::<Result<Vec<_>, _>>()?;
        cell.container = Some(Container { slots });
    }
    Ok(cell)
}

fn stack_in(stack: &mod_api::ItemStackData) -> Result<ItemStack, String> {
    let item = item_by_name(&stack.item).ok_or_else(|| format!("Missing item: {}", stack.item))?;
    if stack.count == 0 || stack.count > item.max_stack_size() {
        return Err(format!("Invalid stack: {}", stack.item));
    }
    let variant = intern_abi_data("CellRecord stack", &stack.data)
        .map_err(|_| format!("Invalid item data on {}", stack.item))?;
    Ok(ItemStack::with_variant(item, stack.count, variant))
}

pub(in crate::modding) fn record_out(cell: &ResolvedCell) -> CellRecord {
    let named = CellData::capture(cell);
    CellRecord {
        block: named.block,
        state: named.state,
        refs: named.state_ids.into_iter().collect(),
        fluid: named.fluid,
        data: named.kv.into_iter().collect(),
        container: cell.container.as_ref().map(|c| {
            c.slots
                .iter()
                .map(|slot| slot.map(item_stack_data))
                .collect()
        }),
    }
}

/// The portable part of a record: what placing it builds and what an item carries.
pub(in crate::modding) fn construction_record(record: &CellRecord) -> Result<Record, String> {
    cell_data(record).construction_record()
}

pub(in crate::modding) fn plan_out(record: &CellRecord) -> RecordPlan {
    let record = match construction_record(record) {
        Ok(record) => record,
        Err(reason) => return RecordPlan::Unsupported { reason },
    };
    match petramond_world::construction::plan(&record, IVec3::ZERO) {
        Plan::Air => RecordPlan::Air,
        Plan::Member(anchor) => RecordPlan::Member {
            anchor: anchor.to_array(),
        },
        Plan::Unit { cost, writes } => RecordPlan::Unit {
            cost: cost.into_iter().map(item_stack_data).collect(),
            footprint: writes.writes.iter().map(|w| w.cell.to_array()).collect(),
        },
        Plan::Unsupported(reason) => RecordPlan::Unsupported { reason },
    }
}

pub(in crate::modding) fn status(
    world: &ServerWorld,
    pos: [i32; 3],
    record: &CellRecord,
) -> RecordStatus {
    let record = match construction_record(record) {
        Ok(record) => record,
        Err(reason) => return RecordStatus::Unsupported { reason },
    };
    match world.construction_status(IVec3::from_array(pos), &record) {
        CellStatus::Unloaded => RecordStatus::Unloaded,
        CellStatus::Satisfied => RecordStatus::Satisfied,
        CellStatus::Place { missing, .. } => RecordStatus::Place {
            missing: missing.into_iter().map(item_stack_data).collect(),
        },
        CellStatus::Clear {
            at,
            block,
            footprint,
            holds_items,
        } => RecordStatus::Clear {
            at: at.to_array(),
            block: mod_api::BlockId(block.id()),
            footprint: footprint.iter().map(|c| c.to_array()).collect(),
            holds_items,
        },
        CellStatus::Pending(anchor) => RecordStatus::Pending {
            anchor: anchor.to_array(),
        },
        CellStatus::Unsupported(reason) => RecordStatus::Unsupported { reason },
    }
}
