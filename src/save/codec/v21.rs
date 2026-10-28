use std::collections::BTreeMap;

use petramond_persist::bytecodec::{get_indexed, get_kv_map, put_indexed, put_kv_map, Reader};

use super::{
    RecordError, FLAG2_HAS_CELL_KV, FLAG2_HAS_CONTAINERS, FLAG3_HAS_BLOCKLIGHT, FLAG3_HAS_SKYLIGHT,
    FLAG_HAS_CELL_STATES, FLAG_HAS_ENTITIES, FLAG_HAS_FLUID, FLAG_HAS_MOBS, SECTION,
};

const SECTION_VOLUME: usize = 4096;
const WIDE_ID_CAP: usize = 4096;

/// v21's furnace frame bit: the frame is gone from v22.
const FLAG_HAS_FURNACES: u8 = 0x04;

/// The cell data key the furnace keeps its counters under from v22 on.
const FURNACE_KEY: &str = "petramond:furnace";

const KNOWN: [u8; 3] = [
    FLAG_HAS_FLUID | FLAG_HAS_ENTITIES | FLAG_HAS_FURNACES | FLAG_HAS_CELL_STATES | FLAG_HAS_MOBS,
    FLAG2_HAS_CELL_KV | FLAG2_HAS_CONTAINERS,
    FLAG3_HAS_SKYLIGHT | FLAG3_HAS_BLOCKLIGHT,
];

/// v21 kept a furnace's counters (cook, burn, burn max: three LE u16 per cell) in a frame of its
/// own. v22 keeps them in the cell's data under [`FURNACE_KEY`], the same six bytes, so this step
/// drops the frame and merges each entry into the cell data frame.
pub(super) fn upgrade(body: &[u8]) -> Result<Vec<u8>, RecordError> {
    let corrupt = |what, at: usize| RecordError::corrupt(SECTION.name, what, at + 1);
    let mut r = Reader::new(body);
    let flags = [0, 1, 2].map(|_| r.u8());
    let [Some(f1), Some(f2), Some(f3)] = flags else {
        return Err(corrupt("flags", r.offset()));
    };
    let unknown = [f1, f2, f3]
        .iter()
        .zip(KNOWN)
        .enumerate()
        .fold(0u32, |acc, (i, (f, known))| {
            acc | (u32::from(f & !known) << (8 * i))
        });
    if unknown != 0 {
        return Err(RecordError::UnknownPayload {
            format: SECTION.name,
            flags: unknown,
        });
    }
    let cube_start = r.offset();
    skip_block_cube(&mut r).ok_or_else(|| corrupt("block cube", r.offset()))?;
    let cube = &body[cube_start..r.offset()];

    let mut frame = |present: bool, what: &'static str| -> Result<Option<&[u8]>, RecordError> {
        if !present {
            return Ok(None);
        }
        let start = r.offset();
        let len = r.u32().ok_or_else(|| corrupt(what, start))? as usize;
        r.bytes(len).map(Some).ok_or_else(|| corrupt(what, start))
    };
    let fluid = frame(f1 & FLAG_HAS_FLUID != 0, "fluid")?;
    let entities = frame(f1 & FLAG_HAS_ENTITIES != 0, "entities")?;
    let furnaces = frame(f1 & FLAG_HAS_FURNACES != 0, "furnaces")?;
    let cell_states = frame(f1 & FLAG_HAS_CELL_STATES != 0, "cell states")?;
    let mobs = frame(f1 & FLAG_HAS_MOBS != 0, "mobs")?;
    let cell_kv = frame(f2 & FLAG2_HAS_CELL_KV != 0, "cell kv")?;
    let containers = frame(f2 & FLAG2_HAS_CONTAINERS != 0, "containers")?;
    let skylight = frame(f3 & FLAG3_HAS_SKYLIGHT != 0, "skylight")?;
    let blocklight = frame(f3 & FLAG3_HAS_BLOCKLIGHT != 0, "block light")?;
    if !r.is_at_end() {
        return Err(corrupt("trailing bytes", r.offset()));
    }

    let mut kv: BTreeMap<u16, BTreeMap<String, Vec<u8>>> = match cell_kv {
        Some(bytes) => {
            let mut r = Reader::new(bytes);
            let kv = get_indexed(&mut r, get_kv_map).ok_or_else(|| corrupt("cell kv", 0))?;
            r.is_at_end()
                .then_some(kv)
                .ok_or_else(|| corrupt("cell kv", 0))?
        }
        None => BTreeMap::new(),
    };
    if let Some(bytes) = furnaces {
        let mut r = Reader::new(bytes);
        let counters = get_indexed(&mut r, |r| r.bytes(6).map(<[u8]>::to_vec))
            .ok_or_else(|| corrupt("furnaces", 0))?;
        if !r.is_at_end() {
            return Err(corrupt("furnaces", 0));
        }
        for (cell, bytes) in counters {
            kv.entry(cell)
                .or_default()
                .insert(FURNACE_KEY.to_owned(), bytes);
        }
    }

    let mut out = Vec::with_capacity(body.len() + 64);
    out.push(f1 & !FLAG_HAS_FURNACES);
    out.push(if kv.is_empty() {
        f2 & !FLAG2_HAS_CELL_KV
    } else {
        f2 | FLAG2_HAS_CELL_KV
    });
    out.push(f3);
    out.extend_from_slice(cube);
    let mut put = |bytes: &[u8]| {
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(bytes);
    };
    for bytes in [fluid, entities, cell_states, mobs].into_iter().flatten() {
        put(bytes);
    }
    if !kv.is_empty() {
        let mut bytes = Vec::new();
        put_indexed(&mut bytes, &kv, 16, put_kv_map);
        put(&bytes);
    }
    for bytes in [containers, skylight, blocklight].into_iter().flatten() {
        put(bytes);
    }
    Ok(out)
}

fn skip_block_cube(r: &mut Reader) -> Option<()> {
    let distinct = r.u16()? as usize;
    if distinct == 0 || distinct > WIDE_ID_CAP {
        return None;
    }
    r.bytes(distinct * 2)?;
    let index_width = if distinct <= 256 { 1 } else { 2 };
    r.bytes(SECTION_VOLUME * index_width).map(|_| ())
}
