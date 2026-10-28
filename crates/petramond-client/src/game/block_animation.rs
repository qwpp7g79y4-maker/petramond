use std::collections::HashMap;

use petramond_math::math::IVec3;
use rustc_hash::FxHashSet;

#[derive(Default)]
pub(super) struct BlockAnimations {
    swings: HashMap<IVec3, f32>,
    held_open: FxHashSet<IVec3>,
}

impl BlockAnimations {
    pub(super) fn set_held_open(&mut self, open: FxHashSet<IVec3>) {
        let entered: Vec<IVec3> = open.difference(&self.held_open).copied().collect();
        let left: Vec<IVec3> = self.held_open.difference(&open).copied().collect();
        for pos in entered {
            self.begin(pos, false);
        }
        for pos in left {
            self.begin(pos, true);
        }
        self.held_open = open;
    }

    pub(super) fn held_open(&self) -> &FxHashSet<IVec3> {
        &self.held_open
    }

    pub(super) fn open_progress(&self, anchor: IVec3, pose_open: bool) -> f32 {
        self.progress(anchor, pose_open || self.held_open.contains(&anchor))
    }

    pub(super) fn advance_poses(&mut self, dt: f32, pose: impl Fn(IVec3) -> Option<(bool, f32)>) {
        let held_open = std::mem::take(&mut self.held_open);
        self.advance(dt, |anchor| {
            pose(anchor).map(|(open, speed)| (open || held_open.contains(&anchor), speed))
        });
        self.held_open = held_open;
    }

    pub(super) fn begin(&mut self, anchor: IVec3, was_open: bool) {
        self.swings
            .entry(anchor)
            .or_insert(if was_open { 1.0 } else { 0.0 });
    }

    pub(super) fn progress(&self, anchor: IVec3, open: bool) -> f32 {
        self.swings
            .get(&anchor)
            .copied()
            .unwrap_or(if open { 1.0 } else { 0.0 })
    }

    pub(super) fn advance(&mut self, dt: f32, target: impl Fn(IVec3) -> Option<(bool, f32)>) {
        self.swings.retain(|&anchor, progress| {
            let Some((open, speed)) = target(anchor) else {
                return false;
            };
            let goal = if open { 1.0 } else { 0.0 };
            let step = (dt * speed).clamp(0.0, 1.0);
            if *progress < goal {
                *progress = (*progress + step).min(goal);
            } else {
                *progress = (*progress - step).max(goal);
            }
            (*progress - goal).abs() > f32::EPSILON
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: IVec3 = IVec3::new(1, 2, 3);

    #[test]
    fn a_reversal_mid_swing_turns_around_and_a_gone_block_is_dropped() {
        let mut anims = BlockAnimations::default();
        anims.begin(AT, false);
        anims.advance(0.1, |_| Some((true, 5.0)));
        anims.begin(AT, true);
        assert!(
            (anims.progress(AT, false) - 0.5).abs() < 1e-5,
            "keeps its progress"
        );
        anims.advance(0.05, |_| Some((false, 5.0)));
        assert!(
            (anims.progress(AT, false) - 0.25).abs() < 1e-5,
            "and eases back"
        );
        anims.advance(0.1, |_| None);
        assert!(anims.swings.is_empty(), "a broken block stops animating");
    }
}
