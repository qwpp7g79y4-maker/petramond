use crate::block::Block;
use crate::fluid::held;

use super::Section;

impl Section {
    /// The fluid the cell holds beside its block, as a slab stands in water.
    pub fn held_fluid(&self, x: usize, y: usize, z: usize) -> Option<Block> {
        let block = self.block(x, y, z);
        if !block.flags().floodable() {
            return None;
        }
        let fluid = held::decode(block, self.cell_kv_get(x, y, z, held::KV_KEY)?)?;
        block
            .can_hold_in(fluid, self.cell_state(x, y, z))
            .then_some(fluid)
    }

    /// The fluid filling the cell: a fluid block, the fluid its row contains, or the one it holds.
    #[inline]
    pub fn cell_fluid(&self, x: usize, y: usize, z: usize) -> Option<Block> {
        let block = self.block(x, y, z);
        block.fluid().or_else(|| self.held_fluid(x, y, z))
    }

    /// Writes a cell data entry. A [`held::KV_KEY`] entry naming a fluid the cell's block
    /// cannot hold is dropped.
    pub fn set_cell_data(&mut self, x: usize, y: usize, z: usize, key: String, value: Vec<u8>) {
        if key == held::KV_KEY && held::decode(self.block(x, y, z), &value).is_none() {
            return;
        }
        self.cell_kv_set(x, y, z, key, value);
    }

    /// Every cell holding a fluid beside its block, as `(cell index, fluid)`.
    pub fn held_fluids(&self) -> impl Iterator<Item = (u16, Block)> + '_ {
        self.cell_kv().iter().filter_map(|(&idx, map)| {
            let block = Block::from_id(self.block_at_idx(idx as usize));
            let fluid = held::decode(block, map.get(held::KV_KEY)?)?;
            let (x, y, z) = crate::chunk::section_local(idx as usize);
            block
                .can_hold_in(fluid, self.cell_state(x, y, z))
                .then_some((idx, fluid))
        })
    }

    /// Fills the cell's block with `fluid`, or empties it. False when nothing changed or the
    /// block cannot hold that fluid.
    pub fn set_held_fluid(&mut self, x: usize, y: usize, z: usize, fluid: Option<Block>) -> bool {
        if self.held_fluid(x, y, z) == fluid {
            return false;
        }
        match fluid {
            Some(fluid)
                if !self
                    .block(x, y, z)
                    .can_hold_in(fluid, self.cell_state(x, y, z)) =>
            {
                return false
            }
            Some(fluid) => {
                self.cell_kv_set(x, y, z, held::KV_KEY.to_owned(), held::encode(fluid));
            }
            None => {
                self.cell_kv_remove(x, y, z, held::KV_KEY);
            }
        }
        self.modified = true;
        self.dirty = true;
        true
    }
}
