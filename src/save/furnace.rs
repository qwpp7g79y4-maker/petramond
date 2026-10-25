use crate::save::codec::{get_indexed, put_indexed, Reader};
use crate::save::wire::{wire_struct, Wire};
use petramond_world::furnace::Furnace;
use petramond_world::section::CellMap;

const FURNACE_BYTES: usize = 2 + 6;

wire_struct!(Furnace {
    cook_progress,
    burn_remaining,
    burn_max,
});

pub fn put_furnaces(buf: &mut Vec<u8>, furnaces: &CellMap<Furnace>) {
    put_indexed(buf, furnaces, FURNACE_BYTES, |buf, f| f.put(buf));
}

pub fn get_furnaces(r: &mut Reader) -> Option<CellMap<Furnace>> {
    get_indexed(r, Furnace::get)
}
