use serde::{Deserialize, Serialize};

pub const WALK: &str = "walk";
pub const SNEAK: &str = "sneak";
pub const SWIM: &str = "swim";
pub const TREAD: &str = "tread";
pub const HURT: &str = "hurt";
pub const AMBIENT: &str = "ambient";
pub const IDLE_PREFIX: &str = "idle_";

pub fn is_idle(name: &str) -> bool {
    name.starts_with(IDLE_PREFIX)
}

/// The locomotion loop a body plays under its named layers.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gait {
    Walk,
    /// Stroking through a fluid, off the floor.
    Swim,
    /// Holding still in a fluid, off the floor.
    Tread,
    Idle(u8),
}
