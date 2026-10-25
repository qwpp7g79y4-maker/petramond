//! Stinging: a mob whose row carries `exploration:sting` data puts its
//! condition on any player who brushes against it. The condition row owns
//! the damage pulses and the look; applying it again only tops it up, so a
//! player tangled in a jellyfish is not stung once per tick.

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StingRow {
    pub condition: String,
    pub stage: String,
    pub ticks: u32,
    /// Extra gap between bodies that still counts as a touch.
    #[serde(default)]
    pub reach: f32,
    /// Played where a touch begins.
    #[serde(default)]
    pub sound: Option<String>,
}

/// A resolved sting.
#[derive(Clone, Debug, PartialEq)]
pub struct Sting {
    pub condition: mod_sdk::ConditionId,
    pub stage: u8,
    pub ticks: u32,
    pub reach: f32,
    pub sound: Option<String>,
}

impl StingRow {
    pub fn resolve(&self) -> Option<Sting> {
        let info = mod_sdk::resolve_condition_logged(&self.condition)?;
        Some(Sting {
            condition: info.id,
            stage: info.stage(&self.stage)?,
            ticks: self.ticks,
            reach: self.reach,
            sound: self.sound.clone(),
        })
    }
}

/// Whether two axis-aligned bodies (feet, half width, height) come within `reach` of each other.
pub fn touching(a: ([f64; 3], f32, f32), b: ([f64; 3], f32, f32), reach: f32) -> bool {
    let r = f64::from(reach);
    let apart = |axis: usize| (a.0[axis] - b.0[axis]).abs() > f64::from(a.1 + b.1) + r;
    if apart(0) || apart(2) {
        return false;
    }
    let (alo, ahi) = (a.0[1], a.0[1] + f64::from(a.2));
    let (blo, bhi) = (b.0[1], b.0[1] + f64::from(b.2));
    alo <= bhi + r && blo <= ahi + r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_touch_needs_both_bodies_to_meet_on_every_axis() {
        let jelly = ([0.0, 50.0, 0.0], 0.4, 0.8);
        let at = |x: f64, y: f64| ([x, y, 0.0], 0.3, 1.8);
        assert!(touching(jelly, at(0.6, 49.0), 0.0));
        assert!(!touching(jelly, at(0.8, 49.0), 0.0));
        assert!(
            touching(jelly, at(0.8, 49.0), 0.2),
            "reach widens the touch"
        );
        assert!(
            !touching(jelly, at(0.0, 51.2), 0.0),
            "above the bell is no touch"
        );
    }
}
