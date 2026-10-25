//! Undercuts: the sea wears a notch into the foot of its cliffs, so a lip of rock hangs over the
//! sand. A column standing [`MIN_DROP`] or more above a side neighbour loses the cells just over
//! that neighbour's floor and keeps a lip at least [`LIP`] thick; where the rock behind it stands
//! as tall, the notch runs a cell deeper. A slow noise picks which stretches of cliff are worn.
//!
//! The notch is a pure function of the seed and the raw floors, so every section cuts its own
//! share of the same notch.

use mod_sdk::build::{Heights, Noise2};

/// How far a column must stand over its neighbour to be worn.
pub const MIN_DROP: i32 = 4;
/// Rock kept over a notch.
const LIP: i32 = 2;
/// The tallest a notch is worn.
const HEIGHT: i32 = 3;
/// Floors read beyond a section's own columns: a notch's back sits two columns from its foot.
pub const REACH: i32 = 2;
const WEAR_SCALE: f32 = 1.0 / 13.0;
/// Wear noise at or over this wears the cliff.
const WEAR_OPEN: f32 = -0.1;
/// Wear noise at or over this wears it a cell deeper.
const DEEP_OPEN: f32 = 0.2;
const SALT_WEAR: u32 = 0x0DE7_C0A1;

const SIDES: [[i32; 2]; 4] = [[1, 0], [-1, 0], [0, 1], [0, -1]];

/// A worn lip: the side it faces and the cells it loses, `bottom..=top`.
struct Lip {
    toward: [i32; 2],
    bottom: i32,
    top: i32,
    wear: f32,
}

/// The notch column `(x, z)` wears toward its lowest side, if it is a submerged cliff lip.
fn lip(seed: u32, sea: i32, floors: &Heights, x: i32, z: i32) -> Option<Lip> {
    let floor = floors.get(x, z)?;
    if floor >= sea - 1 {
        return None;
    }
    let (toward, low) = SIDES
        .iter()
        .filter_map(|&d| Some((d, floors.get(x + d[0], z + d[1])?)))
        .min_by_key(|&(_, h)| h)?;
    if floor - low < MIN_DROP {
        return None;
    }
    let wear = Noise2(seed ^ SALT_WEAR).at(x as f32 * WEAR_SCALE, z as f32 * WEAR_SCALE);
    let (bottom, top) = (low + 1, (low + HEIGHT).min(floor - LIP));
    (wear >= WEAR_OPEN && top >= bottom).then_some(Lip {
        toward,
        bottom,
        top,
        wear,
    })
}

/// Every cell of the 16×16 columns at `origin` the sea wears away. `floors` must reach
/// [`REACH`] columns beyond them.
pub fn cells(seed: u32, sea: i32, floors: &Heights, origin: [i32; 2]) -> Vec<[i32; 3]> {
    let mut out = Vec::new();
    for lz in 0..16 {
        for lx in 0..16 {
            let (x, z) = (origin[0] + lx, origin[1] + lz);
            if let Some(lip) = lip(seed, sea, floors, x, z) {
                out.extend((lip.bottom..=lip.top).map(|y| [x, y, z]));
            }
            for d in SIDES {
                let Some(front) = lip(seed, sea, floors, x + d[0], z + d[1]) else {
                    continue;
                };
                let behind = front.toward == d && front.wear >= DEEP_OPEN;
                if behind && floors.get(x, z).is_some_and(|f| f >= front.top + LIP) {
                    out.extend((front.bottom..=front.top).map(|y| [x, y, z]));
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests;
