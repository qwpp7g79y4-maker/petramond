//! Deep open water for rendering. The skylight flood loses `DECAY` per block of water and runs
//! out about 15 blocks down, where open water and a sealed cave both read 0. A dark cell that
//! holds a fluid and lies under open water presents [`DEEP_WATER_SKY`] instead, so the
//! underwater light curve can keep lighting the open sea floor and leave caves dark.

use crate::chunk::SKY_FULL;

use super::DECAY;

/// The skylight a dark cell in deep open water presents to rendering: the smallest step, below
/// anything the flood leaves in water.
pub const DEEP_WATER_SKY: u8 = 1;

/// A cell straight under a light-blocking cell with at least this much skylight lies just
/// under the surface: the blocker is a lid on the water (a dock, a raft), not a roof.
const LID_LIGHT: u8 = SKY_FULL - 2 * DECAY;

/// Whether a cell of this block stops the walk: it blocks light.
pub fn blocks_light(id: u16) -> bool {
    let word = crate::block::light_cells()
        .get(id as usize)
        .copied()
        .unwrap_or(0);
    word & crate::block::LIGHT_APERTURES_OPEN == 0
}

/// Walks one column top-down from the highest cell its reader can see. A cell is under open
/// water while every cell above it is clear and the skylight above climbs back by exactly
/// `DECAY` per block, the straight descent from the surface; light that came in sideways (a
/// cave mouth) climbs differently. Past the top of what the reader sees, a fluid or a lit cell
/// is taken as open.
#[derive(Default)]
pub struct DeepWaterColumn {
    above: Option<(u8, bool)>,
    open: bool,
}

impl DeepWaterColumn {
    /// The walk lost sight of the column (an absent section): the next cell starts afresh.
    pub fn lose_sight(&mut self) {
        self.above = None;
    }

    /// Steps down onto the next cell; true when it is a dark fluid cell in deep open water.
    pub fn step(&mut self, sky: u8, blocks_light: bool, fluid: bool) -> bool {
        self.open = match self.above {
            _ if blocks_light => false,
            None => sky > 0 || fluid,
            Some((_, true)) => sky >= LID_LIGHT,
            Some(_) if !self.open => false,
            Some((up, false)) if sky == 0 => up <= DECAY,
            Some((up, false)) => (up == SKY_FULL && sky == SKY_FULL) || sky + DECAY == up,
        };
        self.above = Some((sky, blocks_light));
        self.open && sky == 0 && fluid
    }
}

#[cfg(test)]
mod tests;
