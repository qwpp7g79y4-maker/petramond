//! Hold to charge, let go to throw: the clock, clips and claims every thrown weapon shares.

use crate::body::TICK_SECONDS;
use crate::charge::{self, Draw, Press, State};
use crate::claims::Claims;
use mod_sdk::*;
use serde::Deserialize;

/// A row that charges and throws.
pub(crate) trait Thrown {
    fn id(&self) -> ItemId;
    fn draw(&self) -> &Draw;
    fn motion(&self) -> &Motion;
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Clips {
    pub first_person: [String; 2],
    pub body: [String; 2],
}

/// A row's draw and throw clips for both views, and when the throw lets go.
#[derive(Clone, Debug)]
pub(crate) struct Motion {
    pub first_person: [String; 2],
    pub body: [String; 2],
    pub length: f32,
    pub fp_length: f32,
    pub fp_release: f32,
}

impl Motion {
    pub fn resolve(clips: Clips) -> Result<Self, &'static str> {
        let release = |rig: &str, names: &[String; 2]| {
            animation_clip(rig, &names[0]).ok_or("missing draw clip")?;
            let clip = animation_clip(rig, &names[1]).ok_or("missing throw clip")?;
            let at = clip
                .markers
                .iter()
                .find(|(name, _)| name == "release")
                .map(|(_, at)| *at)
                .ok_or("throw clip needs a release marker")?;
            if !clip.length.is_finite() || !at.is_finite() || at <= 0.0 || at >= clip.length {
                return Err("release marker must be inside the throw");
            }
            Ok((clip.length, at))
        };
        let (length, _) = release(rig::PLAYER_BODY, &clips.body)?;
        let (fp_length, fp_release) = release(rig::PLAYER_FIRST_PERSON, &clips.first_person)?;
        Ok(Self {
            first_person: clips.first_person,
            body: clips.body,
            length,
            fp_length,
            fp_release,
        })
    }

    pub fn release_time(&self) -> f32 {
        self.fp_release
    }

    pub fn throw_length(&self) -> f32 {
        self.length.max(self.fp_length)
    }

    pub fn throw_times(&self, elapsed: f32) -> [Option<f32>; 2] {
        [self.fp_length, self.length]
            .map(|length| (elapsed < length).then_some((elapsed / length).clamp(0.0, 1.0)))
    }
}

#[derive(Default)]
pub(crate) struct Clock {
    draw: charge::Clock,
    item: Option<ItemId>,
    throwing: Option<Throw>,
}

struct Throw {
    item: ItemId,
    ticks: u32,
    elapsed: f32,
    released: bool,
}

pub(crate) enum Pose {
    Draw(f32),
    Throw(f32),
}

impl Clock {
    pub fn item(&self) -> Option<ItemId> {
        self.throwing.as_ref().map(|t| t.item).or(self.item)
    }

    pub fn pose(&self) -> Option<Pose> {
        if let Some(t) = &self.throwing {
            return Some(Pose::Throw(t.elapsed));
        }
        match self.draw.state() {
            State::Drawing(ticks) => Some(Pose::Draw(ticks)),
            _ => None,
        }
    }

    /// Advances the draw or the throw; answers the drawn ticks on the frame the throw lets go.
    pub fn step(
        &mut self,
        row: Option<&impl Thrown>,
        held: Option<ItemId>,
        alive: bool,
        press: bool,
        dt_ticks: f32,
    ) -> Option<u32> {
        if !alive {
            *self = Self::default();
            return None;
        }
        if let Some(t) = &mut self.throwing {
            if (!t.released && held != Some(t.item)) || held.is_some_and(|id| id != t.item) {
                *self = Self::default();
                return None;
            }
            let row = row?;
            t.elapsed += dt_ticks.max(0.0) * TICK_SECONDS;
            let release =
                (!t.released && t.elapsed >= row.motion().release_time()).then_some(t.ticks);
            t.released |= release.is_some();
            if t.elapsed >= row.motion().throw_length() {
                self.throwing = None;
            }
            return release;
        }
        let Some(row) = row.filter(|r| held == Some(r.id())) else {
            *self = Self::default();
            return None;
        };
        if self.item != Some(row.id()) {
            self.draw = charge::Clock::default();
            self.item = Some(row.id());
        }
        if let Some(ticks) = self.draw.step(
            if press {
                Press::Held(row.draw())
            } else {
                Press::Released
            },
            dt_ticks,
        ) {
            self.throwing = Some(Throw {
                item: row.id(),
                ticks,
                elapsed: 0.0,
                released: false,
            });
        }
        None
    }
}

/// `clip` scrubbed to `progress` in `rig`'s main claim slot, standing down the vanilla swing and
/// jab, and in first person the item's equip gesture (`claim_param`).
pub(crate) fn claim_play(
    rig: &str,
    clip: &str,
    progress: f32,
    claim_param: &str,
    params: &mut Vec<AnimatorParam>,
) -> AnimatorPlay {
    for param in ["main.swing_claim", "main.jab_claim"] {
        params.push(AnimatorParam {
            rig: rig.into(),
            param: param.into(),
            value: AnimatorValue::Number(1.0),
        });
    }
    if rig == rig::PLAYER_FIRST_PERSON {
        params.push(AnimatorParam {
            rig: rig.into(),
            param: claim_param.into(),
            value: AnimatorValue::Number(1.0),
        });
    }
    AnimatorPlay::scrubbed(rig, "main_claim", clip, progress)
}

/// Hides the main hand's item on both rigs.
pub(crate) fn empty_hand(params: &mut Vec<AnimatorParam>) {
    params.extend(
        [rig::PLAYER_FIRST_PERSON, rig::PLAYER_BODY]
            .into_iter()
            .map(|rig| AnimatorParam {
                rig: rig.into(),
                param: "main.item_visible".into(),
                value: AnimatorValue::Number(0.0),
            }),
    );
}

/// The draw and throw clips on both rigs, standing down the vanilla swing and jab and the
/// first-person equip (`claim_param`). `hide_after_release` empties the hand once it lets go.
pub(crate) fn claims(
    row: Option<&impl Thrown>,
    clock: &Clock,
    claim_param: &str,
    hide_after_release: bool,
) -> Claims {
    let (Some(row), Some(pose)) = (row, clock.pose()) else {
        return Claims::default();
    };
    let (draw, motion) = (row.draw(), row.motion());
    let mut params = Vec::new();
    let times = match pose {
        Pose::Draw(ticks) => [Some((ticks / draw.full_ticks as f32).clamp(0.0, 1.0)); 2],
        Pose::Throw(elapsed) => motion.throw_times(elapsed),
    };
    if hide_after_release
        && matches!(pose, Pose::Throw(elapsed) if elapsed >= motion.release_time())
    {
        empty_hand(&mut params);
    }
    let plays = [rig::PLAYER_FIRST_PERSON, rig::PLAYER_BODY]
        .into_iter()
        .zip([&motion.first_person, &motion.body])
        .zip(times)
        .filter_map(|((rig, clips), progress)| {
            let clip = &clips[usize::from(matches!(pose, Pose::Throw(_)))];
            Some(claim_play(rig, clip, progress?, claim_param, &mut params))
        })
        .collect();
    Claims {
        holds_press: true,
        speed: if matches!(pose, Pose::Draw(_)) {
            draw.speed_scale
        } else {
            1.0
        },
        denied: vec![BodyAction::Attack, BodyAction::Mine],
        params,
        plays,
        ..Default::default()
    }
}
