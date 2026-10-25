use std::collections::{HashMap, VecDeque};

use super::*;
use petramond::world::{TrailPoint, TrailSpan};
use petramond_math::world_pos::WorldPos;
use petramond_world::block::{ParticleEmitter, ParticleEmitterAnchor};
use petramond_world::particle_emitters::{self as emitters, EmitterBundle};

/// How often a body's trail keeps a sample; particles born between two start on the line
/// between them.
const TRAIL_STEP: f32 = 0.05;
/// How far under the eye the mouth sits, as a share of body height, where no rig places it.
const MOUTH_DROP: f32 = 0.1;
/// How fast an item must move through a fluid to leave its wake.
const WAKE_MIN_SPEED: f32 = 1.0;
/// The body an item wears its wake on: about its own size.
const ITEM_BODY: Vec3 = Vec3::splat(0.25);

pub(super) struct EmitterBody {
    feet: WorldPos,
    size: Vec3,
    yaw: f32,
    /// The level way the head faces.
    facing: Vec3,
    seed: u64,
    skylight: u8,
    blocklight: petramond_world::light::BlockLight6,
    eye_height: f32,
    breath: Option<&'static petramond_world::breath::Respiration>,
    /// The rig's mouth on the posed body, when it was posed this frame.
    posed_mouth: Option<WorldPos>,
}

impl EmitterBody {
    /// The fluid's exhale emitter while this body holds its breath under it.
    fn exhale(&self, world: &petramond_world::world::WorldData) -> Option<u8> {
        let profile = self.breath.filter(|p| p.exhales)?;
        let medium = world.eye_fluid(self.feet + Vec3::Y * self.eye_height)?;
        if profile.breathes(medium) {
            return None;
        }
        medium.fluid_def()?.submerged.exhale
    }

    /// The posed rig's mouth, else just under the eye at the front of the body.
    fn mouth(&self) -> WorldPos {
        self.posed_mouth.unwrap_or_else(|| {
            self.feet
                + Vec3::Y * (self.eye_height - MOUTH_DROP * self.size.y)
                + self.facing * (self.size.z * 0.5)
        })
    }
}

/// Where a body was over the last few seconds, so its particles stay where they were born
/// instead of riding along with it.
#[derive(Default)]
pub(super) struct EmitterTrails {
    bodies: HashMap<u64, BodyTrail>,
    /// This frame's arena, which every placed body row's `TrailSpan` indexes.
    pub(super) points: Vec<TrailPoint>,
}

#[derive(Default)]
struct BodyTrail {
    samples: VecDeque<TrailSample>,
    seen: bool,
}

#[derive(Copy, Clone)]
struct TrailSample {
    time: f32,
    feet: WorldPos,
    mouth: WorldPos,
}

impl EmitterTrails {
    pub(super) fn begin_frame(&mut self) {
        self.points.clear();
        self.bodies.values_mut().for_each(|t| t.seen = false);
    }

    pub(super) fn end_frame(&mut self) {
        self.bodies.retain(|_, t| t.seen);
    }

    /// Samples `body` at `now`, keeping `keep` seconds of history.
    fn record(&mut self, body: &EmitterBody, now: f32, keep: f32) -> &VecDeque<TrailSample> {
        let trail = self.bodies.entry(body.seed).or_default();
        trail.seen = true;
        let samples = &mut trail.samples;
        // The presentation clock wraps; a sample from "the future" belongs to the last lap.
        if samples.back().is_some_and(|s| s.time > now) {
            samples.clear();
        }
        if samples.back().is_none_or(|s| now - s.time >= TRAIL_STEP) {
            samples.push_back(TrailSample {
                time: now,
                feet: body.feet,
                mouth: body.mouth(),
            });
        }
        while samples.len() > 1 && samples[1].time <= now - keep {
            samples.pop_front();
        }
        samples
    }
}

fn anchor_of(e: &ParticleEmitter, feet: WorldPos, mouth: WorldPos) -> WorldPos {
    match e.anchor {
        ParticleEmitterAnchor::Mouth => mouth,
        _ => feet,
    }
}

#[derive(Clone, Debug)]
pub struct BodyEmitters {
    attached: Vec<u8>,
    conditions: Vec<(u8, u8)>,
    ids: Vec<u8>,
    tint: [f32; 3],
    self_lit: f32,
}

impl Default for BodyEmitters {
    fn default() -> Self {
        Self {
            attached: Vec::new(),
            conditions: Vec::new(),
            ids: Vec::new(),
            tint: [1.0; 3],
            self_lit: 0.0,
        }
    }
}

impl BodyEmitters {
    pub fn refresh(&mut self, attached: &[u8], conditions: &[(u8, u8)]) {
        if self.attached == attached && self.conditions == conditions {
            return;
        }
        self.attached.clear();
        self.attached.extend_from_slice(attached);
        self.conditions.clear();
        self.conditions.extend_from_slice(conditions);
        let stages = conditions.iter().filter_map(|&(condition, stage)| {
            let def = petramond_world::condition::defs().get(condition as usize)?;
            Some(emitters::by_key(def.stages.get(stage as usize)?.emitter?)?.id)
        });
        self.ids.clear();
        self.ids.extend(attached.iter().copied().chain(stages));
        self.ids.sort_unstable();
        self.ids.dedup();
        self.tint = emitter_tint(self.ids.iter().copied());
        self.self_lit = emitter_self_lit(self.ids.iter().copied());
    }

    pub fn ids(&self) -> &[u8] {
        &self.ids
    }

    pub fn tint(&self) -> [f32; 3] {
        self.tint
    }

    pub fn self_lit(&self) -> f32 {
        self.self_lit
    }
}

fn bundles(ids: impl IntoIterator<Item = u8>) -> impl Iterator<Item = &'static EmitterBundle> {
    ids.into_iter().filter_map(emitters::def)
}

fn emitter_tint(ids: impl IntoIterator<Item = u8>) -> [f32; 3] {
    bundles(ids)
        .filter_map(|bundle| bundle.tint)
        .fold([1.0; 3], |t, b| [t[0] * b[0], t[1] * b[1], t[2] * b[2]])
}

fn emitter_self_lit(ids: impl IntoIterator<Item = u8>) -> f32 {
    bundles(ids)
        .map(|bundle| bundle.body_self_lit)
        .fold(0.0, f32::max)
}

/// Every row of the bundles `ids` on `body`, its particles trailing where the body has been.
fn append_emitters(
    out: &mut Vec<PlacedEmitter>,
    trails: &mut EmitterTrails,
    world: &petramond_world::world::WorldData,
    ids: impl IntoIterator<Item = u8> + Clone,
    body: &EmitterBody,
    view: &ViewVolume,
    now: f32,
) {
    let keep = bundles(ids.clone())
        .flat_map(|b| b.rows)
        .map(|e| e.lifetime[0].max(e.lifetime[1]))
        .fold(0.0, f32::max);
    if keep <= 0.0 {
        return;
    }
    trails.record(body, now, keep);
    for bundle in bundles(ids) {
        append(out, trails, world, bundle, body, view, now);
    }
}

fn append(
    out: &mut Vec<PlacedEmitter>,
    trails: &mut EmitterTrails,
    world: &petramond_world::world::WorldData,
    bundle: &EmitterBundle,
    body: &EmitterBody,
    view: &ViewVolume,
    now: f32,
) {
    let scale = bundle
        .body_size
        .map(|size| body.size / Vec3::from_array(size));
    let mouth = body.mouth();
    for (row, authored) in bundle.rows.iter().enumerate() {
        let mut emitter = *authored;
        if let Some(scale) = scale {
            emitter.offset = (Vec3::from_array(emitter.offset) * scale).to_array();
            let spread = Vec3::from_array(emitter.spawn_box) * scale;
            let (sin, cos) = body.yaw.sin_cos();
            emitter.spawn_box = [
                cos.abs() * spread.x + sin.abs() * spread.z,
                spread.y,
                sin.abs() * spread.x + cos.abs() * spread.z,
            ];
            emitter.velocity = (Vec3::from_array(emitter.velocity) * scale).to_array();
            emitter.velocity_jitter =
                (Vec3::from_array(emitter.velocity_jitter) * scale).to_array();
            let size_scale = (scale.x * scale.y * scale.z).cbrt();
            emitter.size = emitter.size.map(|v| v * size_scale);
            emitter.spiral[0] *= (scale.x * scale.z).sqrt();
        }
        let offset = Vec3::from_array(emitter.offset);
        let origin = anchor_of(&emitter, body.feet, mouth) + offset;
        let start = trails.points.len();
        if let Some(trail) = trails.bodies.get(&body.seed) {
            trails
                .points
                .extend(trail.samples.iter().map(|s| TrailPoint {
                    time: s.time,
                    origin: anchor_of(&emitter, s.feet, s.mouth) + offset,
                }));
        }
        trails.points.push(TrailPoint { time: now, origin });
        let span = &trails.points[start..];
        let envelope = petramond::world::emitter_envelope(&emitter);
        let (mut lo, mut hi) = (origin - envelope, origin + envelope);
        for p in span {
            lo = WorldPos::from_dvec3(lo.as_dvec3().min((p.origin - envelope).as_dvec3()));
            hi = WorldPos::from_dvec3(hi.as_dvec3().max((p.origin + envelope).as_dvec3()));
        }
        if !view.covers_a_pixel(lo, hi, emitters::particle_size(&emitter))
            || !view.aabb_visible(lo, hi)
        {
            trails.points.truncate(start);
            continue;
        }
        out.push(PlacedEmitter {
            origin,
            emitter,
            seed: body.seed
                ^ ((bundle.id as u64) << 32)
                ^ (row as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            skylight: body.skylight,
            blocklight: body.blocklight,
            floor_y: f32::NEG_INFINITY,
            ceiling_y: petramond::world::emitter_ceiling_y(world, origin, &emitter),
            trail: TrailSpan {
                start: start as u32,
                len: (trails.points.len() - start) as u32,
            },
        });
    }
}

impl GamePresentationScratch {
    pub(super) fn collect_mob_emitters(
        &mut self,
        game: &Game,
        tick_alpha: f32,
        view: &ViewVolume,
        now: f32,
    ) {
        let world = game.replica.world.data();
        for (entry, m) in game.replica.entities.mobs().iter().zip(&self.mobs) {
            let def = petramond::mob::def(m.kind);
            let size = def.size;
            let look = m.yaw + m.head_yaw;
            let body = EmitterBody {
                feet: m
                    .ragdoll_span
                    .map_or(m.prev_pos.lerp(m.pos, tick_alpha), |s| s.ground()),
                size: Vec3::new(
                    size.half_width * 2.0,
                    size.height,
                    size.half_length.unwrap_or(size.half_width) * 2.0,
                ),
                yaw: m.yaw,
                facing: Vec3::new(-look.sin(), 0.0, -look.cos()),
                seed: m.id,
                skylight: m.skylight,
                blocklight: m.blocklight,
                eye_height: def.eye_height,
                breath: def.breath.as_ref(),
                posed_mouth: None,
            };
            let exhale = body.exhale(world);
            append_emitters(
                &mut self.particle_emitters,
                &mut self.emitter_trails,
                world,
                entry.emitters().ids().iter().copied().chain(exhale),
                &body,
                view,
                now,
            );
        }
    }

    /// The fluid's wake behind each item moving fast enough through it.
    pub(super) fn collect_item_emitters(
        &mut self,
        game: &Game,
        tick_alpha: f32,
        view: &ViewVolume,
        now: f32,
    ) {
        let world = game.replica.world.data();
        for entry in game.replica.entities.items().iter() {
            let Some([yaw, _, speed]) = entry.curr.flight else {
                continue;
            };
            if speed < WAKE_MIN_SPEED {
                continue;
            }
            let pos = entry.prev.pos.lerp(entry.curr.pos, tick_alpha);
            let cell = pos.block();
            let Some(wake) = world
                .cell_fluid(cell.x, cell.y, cell.z)
                .and_then(|fluid| fluid.fluid_def())
                .and_then(|def| def.submerged.wake)
            else {
                continue;
            };
            let (skylight, blocklight) = world.dynamic_light_at_world(cell.x, cell.y, cell.z);
            let body = EmitterBody {
                feet: pos,
                size: ITEM_BODY,
                yaw,
                facing: Vec3::new(yaw.sin(), 0.0, yaw.cos()),
                seed: entry.curr.id ^ 0x9E37_79B9_7F4A_7C15,
                skylight,
                blocklight,
                eye_height: 0.0,
                breath: None,
                posed_mouth: None,
            };
            append_emitters(
                &mut self.particle_emitters,
                &mut self.emitter_trails,
                world,
                std::iter::once(wake),
                &body,
                view,
                now,
            );
        }
    }

    /// The local player's and every visible remote player's emitters. Runs after the bodies are
    /// posed, so mouth rows sit on the posed head.
    pub(super) fn collect_player_emitters(
        &mut self,
        game: &Game,
        local: Option<&BodyInput>,
        view: &ViewVolume,
        now: f32,
    ) {
        let world = game.replica.world.data();
        if !game.local.player.is_spectator() && game.replica.self_view.health > 0 {
            let mut body = match local {
                Some(body) => EmitterBody::player(
                    body.pos,
                    body.state.body_yaw,
                    body.state.body_yaw + body.state.head_yaw,
                    body.skylight,
                    body.blocklight,
                    0,
                ),
                None => {
                    let (skylight, blocklight) = game.held_item_light();
                    let feet = game.local.player.pos
                        + Vec3::new(0.0, game.local.camera_rig.step_y_offset(), 0.0);
                    EmitterBody::player(
                        feet,
                        local_body_yaw(game),
                        game.local.player.yaw,
                        skylight,
                        blocklight,
                        0,
                    )
                }
            };
            body.posed_mouth = self.animation.mouth(petramond_render::LOCAL_BODY_KEY);
            let exhale = body.exhale(world);
            append_emitters(
                &mut self.particle_emitters,
                &mut self.emitter_trails,
                world,
                self.local_emitters.ids().iter().copied().chain(exhale),
                &body,
                view,
                now,
            );
        }
        let players = game.replica.entities.players();
        let mut pending = std::mem::take(&mut self.remote_emitter_bodies);
        for (mut body, id, key) in pending.drain(..) {
            let Some(p) = players.get(id) else {
                continue;
            };
            body.posed_mouth = self.animation.mouth(key);
            append_emitters(
                &mut self.particle_emitters,
                &mut self.emitter_trails,
                world,
                p.emitters().ids().iter().copied().chain(body.exhale(world)),
                &body,
                view,
                now,
            );
        }
        self.remote_emitter_bodies = pending;
    }
}

impl EmitterBody {
    pub(super) fn player(
        feet: WorldPos,
        yaw: f32,
        look_yaw: f32,
        skylight: u8,
        blocklight: petramond_world::light::BlockLight6,
        id: u64,
    ) -> Self {
        Self {
            feet,
            yaw,
            facing: Vec3::new(look_yaw.sin(), 0.0, look_yaw.cos()),
            skylight,
            blocklight,
            size: Vec3::new(
                petramond::player::HALF_W * 2.0,
                petramond::player::HEIGHT,
                petramond::player::HALF_W * 2.0,
            ),
            seed: id ^ 0xD1B5_4A32_D192_ED03,
            eye_height: petramond::player::EYE,
            breath: Some(&petramond::player::RESPIRATION),
            posed_mouth: None,
        }
    }
}
