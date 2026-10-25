use crate::game::GameEvents;
use petramond_world::breath::BreathView;
use petramond_world::gui_state::HealthView;

const HURT_SHAKE_SECS: f32 = 0.25;
const SLEEP_INTERACT_HAND_SECS: f32 = 0.30;
const HEART_WIGGLE_SECS: f64 = 0.2;

#[derive(Copy, Clone)]
struct HeartWiggle {
    lo: i32,
    hi: i32,
    started: f64,
}

#[derive(Clone, Copy)]
pub(super) struct HurtShake {
    pub(super) yaw: f32,
    pub(super) pitch: f32,
    pub(super) hand: [f32; 2],
    pub(super) flash: f32,
}

#[derive(Default)]
pub(super) struct HudFx {
    hurt_shake_t: f32,
    sleep_hand_t: f32,
    prev_heart_health: Option<i32>,
    prev_bubbles: Option<u32>,
    heart_wiggle: Option<HeartWiggle>,
    hand_events: Vec<(petramond::player::RigId, u16)>,
}

impl HudFx {
    pub(super) fn latch_hurt(&mut self) {
        self.hurt_shake_t = HURT_SHAKE_SECS;
    }

    pub(super) fn latch_hand_triggers(&mut self, events: &GameEvents) {
        if events.bed_interacted {
            self.sleep_hand_t = SLEEP_INTERACT_HAND_SECS;
        }
        for (hand, kind) in events.one_shots() {
            self.hand_events
                .extend(petramond::player::one_shot::fired(hand, kind));
        }
        self.hand_events.extend_from_slice(&events.animator_events);
    }

    pub(super) fn advance(&mut self, dt: f32) {
        self.sleep_hand_t = (self.sleep_hand_t - dt).max(0.0);
        self.hurt_shake_t = (self.hurt_shake_t - dt).max(0.0);
    }

    pub(super) fn sleep_hand_visible(&self) -> bool {
        self.sleep_hand_t > 0.0
    }

    pub(super) fn hurt_remaining(&self) -> f32 {
        self.hurt_shake_t
    }

    pub(super) fn shake(&self, now: f64) -> HurtShake {
        if self.hurt_shake_t <= 0.0 {
            return HurtShake {
                yaw: 0.0,
                pitch: 0.0,
                hand: [0.0, 0.0],
                flash: 0.0,
            };
        }
        let envelope = (self.hurt_shake_t / HURT_SHAKE_SECS).clamp(0.0, 1.0);
        let amp = envelope * envelope;
        let t = now as f32;
        let (a, b) = ((t * 71.0).sin(), (t * 53.0).cos());
        HurtShake {
            yaw: 0.011 * amp * a,
            pitch: 0.008 * amp * b,
            hand: [0.032 * amp * b, 0.026 * amp * a],
            flash: envelope,
        }
    }

    pub(super) fn hand_events(&self) -> &[(petramond::player::RigId, u16)] {
        &self.hand_events
    }

    pub(super) fn clear_hand_events(&mut self) {
        self.hand_events.clear();
    }

    /// True when a breath bubble vanished from the HUD since the last frame.
    pub(super) fn bubble_popped(&mut self, breath: Option<BreathView>) -> bool {
        let now = breath.map(BreathView::bubbles_left);
        let popped = matches!((self.prev_bubbles, now), (Some(prev), Some(cur)) if cur < prev);
        self.prev_bubbles = now;
        popped
    }

    pub(super) fn heart_wiggle(
        &mut self,
        health: Option<HealthView>,
        now: f64,
    ) -> Option<(i32, i32, f32)> {
        let current = health.map(|h| h.current);
        if let (Some(prev), Some(cur)) = (self.prev_heart_health, current) {
            if cur != prev {
                self.heart_wiggle = Some(HeartWiggle {
                    lo: cur.min(prev),
                    hi: cur.max(prev),
                    started: now,
                });
            }
        }
        self.prev_heart_health = current;
        let w = self.heart_wiggle?;
        let t = now - w.started;
        if t >= HEART_WIGGLE_SECS {
            self.heart_wiggle = None;
            return None;
        }
        Some((w.lo, w.hi, t as f32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health(current: i32) -> Option<HealthView> {
        Some(HealthView { current, max: 20 })
    }

    #[test]
    fn a_health_change_wiggles_the_changed_hearts_for_a_moment() {
        let mut fx = HudFx::default();
        assert_eq!(
            fx.heart_wiggle(health(20), 0.0),
            None,
            "first sight is not a change"
        );
        assert_eq!(fx.heart_wiggle(health(17), 1.0), Some((17, 20, 0.0)));
        assert_eq!(
            fx.heart_wiggle(health(17), 1.1).map(|w| (w.0, w.1)),
            Some((17, 20))
        );
        assert_eq!(fx.heart_wiggle(health(17), 1.3), None, "the burst ends");
    }

    #[test]
    fn a_hidden_bar_breaks_the_comparison() {
        let mut fx = HudFx::default();
        fx.heart_wiggle(health(20), 0.0);
        fx.heart_wiggle(None, 0.5);
        assert_eq!(fx.heart_wiggle(health(10), 1.0), None);
    }

    #[test]
    fn a_bubble_pops_only_when_one_vanishes_from_a_shown_row() {
        let breath = |current| Some(BreathView { current, max: 300 });
        let mut fx = HudFx::default();
        assert!(!fx.bubble_popped(breath(299)), "first sight is not a pop");
        assert!(
            !fx.bubble_popped(breath(271)),
            "the bubble is still showing"
        );
        assert!(fx.bubble_popped(breath(270)));
        assert!(!fx.bubble_popped(breath(300)), "refilling never pops");
        assert!(!fx.bubble_popped(None));
        assert!(
            !fx.bubble_popped(breath(30)),
            "a row reappearing is not a pop"
        );
    }
}
