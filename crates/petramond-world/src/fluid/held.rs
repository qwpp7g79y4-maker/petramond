//! A fluid a cell holds beside its block: a slab or a fence standing in water. The cell's
//! [`KV_KEY`] data names the fluid, and the cell acts as a source of it everywhere a loose
//! source would.

use crate::block::Block;

pub const KV_KEY: &str = "petramond:fluid";

pub fn encode(fluid: Block) -> Vec<u8> {
    crate::registry::names()
        .blocks
        .name(fluid.id())
        .unwrap_or_default()
        .as_bytes()
        .to_vec()
}

/// The fluid a cell of `block` holds, from the cell's [`KV_KEY`] data.
pub fn decode(block: Block, data: &[u8]) -> Option<Block> {
    let name = std::str::from_utf8(data).ok()?;
    let fluid = Block::from_id(crate::registry::names().blocks.id(name)?);
    block.can_hold(fluid).then_some(fluid)
}
