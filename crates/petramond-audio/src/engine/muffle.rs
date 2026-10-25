//! The world lane heard through a medium (the listener's head under water): a
//! one-pole low-pass and a gentle duck, applied where the world mix leaves for
//! the device or the offline mixdown. Its depth eases toward the amount the
//! game sets, so ducking under or surfacing never clicks, and a dry listener
//! passes samples through untouched.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Cutoff with nothing muffled and with the full muffle.
const OPEN_HZ: f32 = 20_000.0;
const MUFFLED_HZ: f32 = 520.0;
/// Level lost at the full muffle.
const DUCK: f32 = 0.35;
/// Seconds the depth takes to settle most of the way to a new amount.
const EASE_SECONDS: f32 = 0.12;
/// Depth below which an easing-out muffle snaps back to the untouched dry path.
const DRY_BELOW: f32 = 2e-3;

/// The amount (0 = dry, 1 = fully muffled) the game writes and the mix reads.
#[derive(Clone, Default)]
pub(super) struct MuffleControl(Arc<AtomicU32>);

impl MuffleControl {
    pub(super) fn set(&self, amount: f32) {
        let amount = if amount.is_finite() {
            amount.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.0.store(amount.to_bits(), Ordering::Relaxed);
    }

    fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
}

/// One interleaved stream's filter state.
pub(super) struct Muffle {
    control: MuffleControl,
    amount: f32,
    ease: f32,
    rate: f32,
    coeff: f32,
    gain: f32,
    held: Vec<f32>,
    channel: usize,
}

impl Muffle {
    pub(super) fn new(control: MuffleControl, channels: u16, sample_rate: u32) -> Self {
        let rate = sample_rate.max(1) as f32;
        Self {
            control,
            amount: 0.0,
            ease: 1.0 - (-1.0 / (EASE_SECONDS * rate)).exp(),
            rate,
            coeff: 1.0,
            gain: 1.0,
            held: vec![0.0; channels.max(1) as usize],
            channel: 0,
        }
    }

    #[inline]
    pub(super) fn apply(&mut self, x: f32) -> f32 {
        let ch = self.channel;
        self.channel = (ch + 1) % self.held.len();
        if ch == 0 {
            self.step();
        }
        if self.amount == 0.0 {
            self.held[ch] = x;
            return x;
        }
        let y = self.held[ch] + self.coeff * (x - self.held[ch]);
        self.held[ch] = y;
        y * self.gain
    }

    /// Once per frame: ease the depth and recompute the filter for it.
    fn step(&mut self) {
        let target = self.control.get();
        if target == 0.0 && self.amount < DRY_BELOW {
            self.amount = 0.0;
            return;
        }
        self.amount += (target - self.amount) * self.ease;
        let cutoff = OPEN_HZ * (MUFFLED_HZ / OPEN_HZ).powf(self.amount);
        self.coeff = 1.0 - (-std::f32::consts::TAU * cutoff / self.rate).exp();
        self.gain = 1.0 - DUCK * self.amount;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f32, rate: u32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|i| (std::f32::consts::TAU * hz * i as f32 / rate as f32).sin())
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn under_water_the_highs_fade_far_more_than_the_lows_and_surfacing_restores_them() {
        let rate = 48_000;
        let control = MuffleControl::default();
        control.set(1.0);
        let heard = |hz: f32| {
            let mut muffle = Muffle::new(control.clone(), 1, rate);
            let out: Vec<f32> = tone(hz, rate, 48_000)
                .iter()
                .map(|&s| muffle.apply(s))
                .collect();
            rms(&out[24_000..])
        };
        let (low, high) = (heard(120.0), heard(6_000.0));
        assert!(low > 0.4, "the lows stay audible: {low}");
        assert!(high < low * 0.25, "the highs fade: {high} vs {low}");

        let mut muffle = Muffle::new(control.clone(), 1, rate);
        for s in tone(6_000.0, rate, 24_000) {
            muffle.apply(s);
        }
        control.set(0.0);
        let input = tone(6_000.0, rate, 72_000);
        let out: Vec<f32> = input.iter().map(|&s| muffle.apply(s)).collect();
        assert_eq!(
            &out[60_000..],
            &input[60_000..],
            "back to dry once it has eased out"
        );
    }
}
