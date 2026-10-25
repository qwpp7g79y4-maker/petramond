//! Respiration: what a body breathes, how long it holds out anywhere else,
//! and the clocked pulse it takes once it runs dry. The medium at a body's
//! breath point is the fluid its eye sits in (`WorldData::eye_fluid`, the
//! same rule as the camera murk) or `Block::Air`.

use crate::block::Block;
use crate::condition::{Pulse, RawPulse};

/// Breath is counted in thousandths of a tick so a fractional drain (a
/// potion, a helmet) stays exact over a long dive.
const MILLI: u32 = 1000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Respiration {
    /// Ticks a body holds out away from what it breathes, at drain 1.
    pub capacity: u32,
    /// Media this body breathes: `Block::Air` for open air, or fluid blocks.
    pub breathes: &'static [Block],
    /// Ticks of breath regained per tick back in a breathable medium.
    pub recover: u32,
    /// Dealt every `interval` ticks once the breath is gone.
    pub damage: Pulse,
    /// Trails the fluid's exhale bubbles while holding its breath under it.
    pub exhales: bool,
}

impl Respiration {
    pub const AIR_BREATHER: Self = Self {
        capacity: 300,
        breathes: &[Block::Air],
        recover: 5,
        damage: Pulse {
            amount: 2,
            interval: 20,
        },
        exhales: true,
    };

    #[inline]
    pub fn breathes(&self, medium: Block) -> bool {
        self.breathes.contains(&medium)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BreathView {
    pub current: u32,
    pub max: u32,
}

impl BreathView {
    /// Bubbles the HUD splits a full breath into.
    pub const BUBBLES: u32 = 10;

    /// Bubbles still showing: one per started share of the breath.
    pub fn bubbles_left(self) -> u32 {
        if self.max == 0 {
            return 0;
        }
        (self.current.min(self.max) * Self::BUBBLES).div_ceil(self.max)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BodyBreath {
    profile: Respiration,
    air: u32,
    clock: u32,
}

impl BodyBreath {
    pub fn new(profile: Respiration) -> Self {
        Self {
            air: profile.capacity * MILLI,
            clock: profile.damage.interval,
            profile,
        }
    }

    #[inline]
    pub fn profile(&self) -> &Respiration {
        &self.profile
    }

    pub fn view(&self) -> BreathView {
        BreathView {
            current: self.air.div_ceil(MILLI),
            max: self.profile.capacity,
        }
    }

    pub fn is_full(&self) -> bool {
        self.air >= self.profile.capacity * MILLI
    }

    /// One tick at a breath point in `medium`; `None` = the point is
    /// unreadable, so the body holds where it is. `drain` scales the breath
    /// spent per tick away from a breathable medium (0 = never runs out).
    /// Returns the damage that fell due this tick.
    pub fn tick(&mut self, medium: Option<Block>, drain: f32) -> Option<i32> {
        let medium = medium?;
        let full = self.profile.capacity * MILLI;
        if self.profile.breathes(medium) {
            self.air = (self.air + self.profile.recover * MILLI).min(full);
            self.clock = self.profile.damage.interval;
            return None;
        }
        let spend = if drain.is_finite() {
            (drain.max(0.0) * MILLI as f32).round() as u32
        } else {
            0
        };
        if spend == 0 {
            return None;
        }
        if self.air > 0 {
            self.air = self.air.saturating_sub(spend);
            return None;
        }
        self.clock -= 1;
        if self.clock > 0 {
            return None;
        }
        self.clock = self.profile.damage.interval;
        Some(self.profile.damage.amount)
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRespiration {
    capacity: u32,
    breathes: Vec<Block>,
    #[serde(default = "default_recover")]
    recover: u32,
    #[serde(default)]
    damage: Option<RawPulse>,
    #[serde(default = "default_exhales")]
    exhales: bool,
}

fn default_exhales() -> bool {
    Respiration::AIR_BREATHER.exhales
}

fn default_recover() -> u32 {
    Respiration::AIR_BREATHER.recover
}

impl RawRespiration {
    pub fn resolve(self) -> Result<Respiration, String> {
        if self.capacity == 0 || self.capacity > 72_000 {
            return Err("breath capacity must be 1..=72000 ticks".into());
        }
        if self.recover == 0 || self.recover > self.capacity {
            return Err("breath recover must be 1..=capacity ticks".into());
        }
        let mut breathes = self.breathes;
        breathes.sort_by_key(|b| b.id());
        breathes.dedup();
        if breathes.is_empty() {
            return Err("breath must name what it breathes".into());
        }
        if let Some(b) = breathes.iter().find(|b| **b != Block::Air && !b.is_fluid()) {
            return Err(format!(
                "breath medium {b:?} must be petramond:air or a fluid"
            ));
        }
        Ok(Respiration {
            capacity: self.capacity,
            breathes: Box::leak(breathes.into_boxed_slice()),
            recover: self.recover,
            damage: self
                .damage
                .map(RawPulse::resolve)
                .transpose()?
                .unwrap_or(Respiration::AIR_BREATHER.damage),
            exhales: self.exhales,
        })
    }
}

#[cfg(test)]
mod tests;
