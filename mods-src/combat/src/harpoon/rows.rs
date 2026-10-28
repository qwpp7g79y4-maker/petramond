use crate::charge::Draw;
use crate::claims::Claims;
use crate::keys::HARPOON_KEY;
use crate::throw::{self, Clips, Motion, Thrown};
use mod_sdk::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Spec {
    draw_ticks: u32,
    strain_ticks: u32,
    draw_speed_scale: f32,
    launch_speed: [f32; 2],
    damage_weak: [f32; 2],
    damage_full: [f32; 2],
    reel_damage: [f32; 2],
    /// How hard the reel's hit pulls the struck body toward the thrower.
    pull: f32,
    /// How far the line reaches before it snaps.
    reach: f32,
    reel_speed: f32,
    /// How far past the hand along the throw the harpoon leaves from, so its tail (where the
    /// rope ties) starts in front of the fist.
    #[serde(default)]
    lead: f32,
    /// How the line is drawn: tile, tint, width (texels), each over the plain rope's.
    #[serde(default)]
    rope: Rope,
    #[serde(default)]
    sounds: Sounds,
    animations: Clips,
    /// While the line is out: the hold, the reel's tug, and the catch once it is in.
    line_animations: LineClips,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LineClips {
    hold: RigClips,
    reel: RigClips,
    catch: RigClips,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RigClips {
    first_person: Option<String>,
    body: Option<String>,
}

/// A clip per rig (first person, body), each with its length.
#[derive(Clone, Debug, Default)]
pub(super) struct Gesture {
    pub clips: [Option<(String, f32)>; 2],
}

impl Gesture {
    fn resolve(clips: RigClips) -> Result<Self, &'static str> {
        let one = |rig: &str, name: Option<String>| {
            name.map(|name| {
                animation_clip(rig, &name)
                    .map(|clip| (name, clip.length))
                    .ok_or("missing line clip")
            })
            .transpose()
        };
        Ok(Self {
            clips: [
                one(rig::PLAYER_FIRST_PERSON, clips.first_person)?,
                one(rig::PLAYER_BODY, clips.body)?,
            ],
        })
    }

    /// How long the gesture runs: its longest clip.
    pub fn length(&self) -> f32 {
        self.clips
            .iter()
            .flatten()
            .map(|(_, l)| *l)
            .fold(0.0, f32::max)
    }

    /// Plays every clip `elapsed` seconds in.
    pub fn claim(&self, elapsed: f32, claim_param: &str, claims: &mut Claims) {
        for (rig, clip) in [rig::PLAYER_FIRST_PERSON, rig::PLAYER_BODY]
            .into_iter()
            .zip(&self.clips)
        {
            let Some((name, length)) = clip else {
                continue;
            };
            let progress = (elapsed / length.max(1e-3)).clamp(0.0, 1.0);
            claims.plays.push(throw::claim_play(
                rig,
                name,
                progress,
                claim_param,
                &mut claims.params,
            ));
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct LineMotion {
    pub hold: Gesture,
    pub reel: Gesture,
    pub catch: Gesture,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Rope {
    tile: Option<String>,
    tint: Option<[u8; 3]>,
    width: Option<f32>,
}

impl Rope {
    /// The plain rope with this row's own look laid over it.
    fn look(&self) -> DrawPrim {
        let DrawPrim::Line {
            tile, tint, width, ..
        } = rope(LineEnd::Local([0.0; 3]), LineEnd::Local([0.0; 3]))
        else {
            unreachable!("a rope is a line");
        };
        DrawPrim::Line {
            from: LineEnd::Local([0.0; 3]),
            to: LineEnd::Local([0.0; 3]),
            tile: self.tile.clone().unwrap_or(tile),
            tint: self.tint.unwrap_or(tint),
            width: self
                .width
                .filter(|w| w.is_finite() && *w > 0.0)
                .unwrap_or(width),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Sounds {
    pub throw: Option<String>,
    pub hit: Option<String>,
    pub reel: Option<String>,
    pub catch: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct Row {
    pub id: ItemId,
    pub name: String,
    pub draw: Draw,
    pub damage_weak: [f32; 2],
    pub damage_full: [f32; 2],
    pub reel_damage: [f32; 2],
    pub pull: f32,
    pub reach: f32,
    pub reel_speed: f32,
    pub lead: f32,
    /// The rules a thrown harpoon flies with.
    pub rules: ItemEntityRules,
    /// The rope's look; [`Row::rope_between`] gives it its ends.
    pub rope: DrawPrim,
    pub sounds: Sounds,
    pub motion: Motion,
    pub line: LineMotion,
}

impl Row {
    /// The rope from the harpoon `entity` back to `owner`'s hand.
    pub fn rope_between(&self, entity: u64, owner: PlayerId) -> DrawPrim {
        let mut rope = self.rope.clone();
        if let DrawPrim::Line { from, to, .. } = &mut rope {
            *from = LineEnd::Entity(EntityRef::Entity(entity));
            *to = LineEnd::Entity(EntityRef::Player(owner));
        }
        rope
    }

    pub fn from_spec(id: ItemId, name: String, spec: Spec) -> Result<Self, &'static str> {
        let positive = |v: f32| v.is_finite() && v > 0.0;
        let band = |[lo, hi]: [f32; 2]| lo.is_finite() && hi.is_finite() && lo >= 0.0 && hi >= lo;
        if spec.draw_ticks == 0
            || spec.draw_ticks.checked_add(spec.strain_ticks).is_none()
            || !positive(spec.draw_speed_scale)
            || !spec.launch_speed.into_iter().all(positive)
            || spec.launch_speed[1] < spec.launch_speed[0]
            || spec.launch_speed[1] > 100.0
            || !band(spec.damage_weak)
            || !band(spec.damage_full)
            || !band(spec.reel_damage)
            || !(0.0..=8.0).contains(&spec.pull)
            || !positive(spec.reach)
            || !positive(spec.reel_speed)
            || spec.reel_speed > 100.0
            || !(0.0..=4.0).contains(&spec.lead)
        {
            return Err("invalid charge, line or damage band");
        }
        if spec
            .rope
            .width
            .is_some_and(|w| !(w > 0.0 && w <= mod_sdk::LINE_WIDTH_MAX))
        {
            return Err("rope width is outside a line's");
        }
        let rules = ItemEntityRules {
            pickup: false,
            lifetime_ticks: None,
        };
        Ok(Self {
            id,
            name,
            draw: Draw {
                full_ticks: spec.draw_ticks,
                strain_ticks: spec.strain_ticks,
                speed_scale: spec.draw_speed_scale,
                launch_speed: spec.launch_speed,
            },
            damage_weak: spec.damage_weak,
            damage_full: spec.damage_full,
            reel_damage: spec.reel_damage,
            pull: spec.pull,
            reach: spec.reach,
            reel_speed: spec.reel_speed,
            lead: spec.lead,
            rules,
            rope: spec.rope.look(),
            sounds: spec.sounds,
            motion: Motion::resolve(spec.animations)?,
            line: LineMotion {
                hold: Gesture::resolve(spec.line_animations.hold)?,
                reel: Gesture::resolve(spec.line_animations.reel)?,
                catch: Gesture::resolve(spec.line_animations.catch)?,
            },
        })
    }

    /// The damage band of a throw drawn for `ticks`, between the weak and the full one.
    pub fn damage(&self, ticks: u32) -> [f32; 2] {
        let t = (ticks as f32 / self.draw.full_ticks.max(1) as f32).clamp(0.0, 1.0);
        std::array::from_fn(|i| {
            self.damage_weak[i] + (self.damage_full[i] - self.damage_weak[i]) * t
        })
    }
}

impl Thrown for Row {
    fn id(&self) -> ItemId {
        self.id
    }

    fn draw(&self) -> &Draw {
        &self.draw
    }

    fn motion(&self) -> &Motion {
        &self.motion
    }
}

pub(super) fn load() -> Vec<Row> {
    let specs = items_with_data_as::<Spec>(HARPOON_KEY);
    let names = item_names(specs.iter().map(|(id, _)| *id).collect());
    specs
        .into_iter()
        .zip(names)
        .filter_map(|((id, spec), name)| {
            let name = name?;
            Row::from_spec(id, name.clone(), spec)
                .map_err(|e| log(&row_error(HARPOON_KEY, &name, e)))
                .ok()
        })
        .collect()
}
