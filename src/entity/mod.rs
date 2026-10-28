mod dropped_item;
#[cfg(test)]
pub mod fluid_fixture;
mod item_rules;
pub mod shore;

#[cfg(any(test, feature = "test-support"))]
pub use dropped_item::ATTRACT_RADIUS;
pub use dropped_item::{DroppedItem, Fate, Flight, Heading, Motion, Stuck};

pub const VELOCITY_SLACK: f32 = 1.25;

/// The one id space every entity draws from, mobs and item entities alike, so an id names one
/// body whatever kind it is. Session-scoped: no store persists ids.
#[derive(Clone, Debug, Default)]
pub struct EntityIds(std::sync::Arc<std::sync::atomic::AtomicU64>);

impl EntityIds {
    pub fn next(&self) -> u64 {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }
}

#[inline]
pub fn hash01(seed: u64) -> f32 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    ((z >> 40) as f32) / ((1u32 << 24) as f32)
}

#[inline]
pub fn hash_signed(seed: u64) -> f32 {
    hash01(seed) * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash01_is_in_unit_range_and_deterministic() {
        for i in 0..10_000u64 {
            let h = hash01(i);
            assert!((0.0..1.0).contains(&h), "hash01({i}) = {h} out of range");
            assert_eq!(h, hash01(i), "hash01 must be deterministic");
        }
    }
}
