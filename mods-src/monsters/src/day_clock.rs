//! The day/night clock, as the pack reads it.

use mod_sdk::{world_kv_get, ByteReader};

/// The engine's daylight, off the curve the sky draws.
const DAYLIGHT_KEY: &str = "petramond:daylight";

/// How much daylight there is now: 0 at night, 1 by day. `None` without a readable clock.
pub fn daylight_now() -> Option<f32> {
    let bytes = world_kv_get(DAYLIGHT_KEY)?;
    let day = ByteReader::new(&bytes).f32()?;
    (day.is_finite() && (0.0..=1.0).contains(&day)).then_some(day)
}
