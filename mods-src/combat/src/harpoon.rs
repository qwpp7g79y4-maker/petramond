//! The harpoon: charge and throw it on a rope. It lodges in what it strikes and holds there while
//! the rope is in hand. Use reels it back, tearing it free with a second, smaller hit that pulls
//! the body toward the thrower, and the harpoon flies home to be caught. Put the harpoon away, die
//! or let the line run past its reach and the rope is cut: the harpoon stays where it is, lodged
//! in a body until that body dies, and leaves the thrower's pack.
//!
//! The thrown harpoon stays in its slot while it is out, the hand empty around the line, so the
//! line's state is the server's: it tells the thrower's client through [`LINE_EVENT`], and both
//! sides draw the same hand.

mod rows;
#[cfg(test)]
mod tests;

use crate::body::{BodyClocks, TICK_SECONDS};
use crate::claims::{Body, Claims, Rule};
use crate::strike;
use crate::throw::{self, Clock};
use mod_sdk::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use rows::Row;

pub const LINE_EVENT: &str = "combat:harpoon_line";
const CLAIM_PARAM: &str = "combat.harpoon_claim";
/// How close the returning harpoon comes before the hand closes on it.
const CATCH_DISTANCE: f32 = 1.0;
/// How far short of what the crosshair is on a throw's lead stops.
const LEAD_CLEARANCE: f32 = 0.5;
/// Where the line meets the thrower: the main hand, this share of their height up, this far to
/// the right of and ahead of their middle.
const HAND_HEIGHT: f32 = 0.5;
const HAND_SIDE: f32 = 0.35;
const HAND_AHEAD: f32 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Hold {
    Flying,
    Lodged(EntityRef),
    Stuck,
    Loose,
    Reeling,
}

#[derive(Clone, Debug)]
struct Line {
    entity: u64,
    row: usize,
    hold: Hold,
}

pub struct Harpoons {
    rows: Vec<Row>,
    lines: RefCell<BTreeMap<PlayerId, Line>>,
    /// How long each harpoon in flight was drawn, which sets how hard it strikes.
    charges: RefCell<BTreeMap<u64, u32>>,
    /// Reel hits waiting for their `mob_damage_pre`, which turns their knockback toward the thrower.
    pulls: RefCell<Vec<(PlayerId, u64, f32)>>,
}

fn hand_of(state: &PlayerSnapshot) -> [f64; 3] {
    let (sy, cy) = state.yaw.sin_cos();
    [
        state.pos[0] + f64::from(sy * HAND_AHEAD - cy * HAND_SIDE),
        state.pos[1] + f64::from(state.height * HAND_HEIGHT),
        state.pos[2] + f64::from(cy * HAND_AHEAD + sy * HAND_SIDE),
    ]
}

fn advance(clock: &mut Option<f32>, dt: f32, length: f32) {
    if let Some(t) = clock {
        *t += dt;
        if *t >= length {
            *clock = None;
        }
    }
}

fn toward(from: [f64; 3], to: [f64; 3], speed: f32) -> ([f32; 3], f32) {
    let d: [f32; 3] = std::array::from_fn(|i| (to[i] - from[i]) as f32);
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if length < 1e-4 {
        return ([0.0; 3], length);
    }
    (d.map(|v| v / length * speed), length)
}

/// Steers a reeled harpoon home tail first: the rope pulls on the end it is tied to.
fn tow(entity: u64, vel: [f32; 3]) -> bool {
    let steered = steer_item(entity, Some(vel));
    if steered {
        set_item_facing(entity, Some(vel.map(|v| -v)));
    }
    steered
}

fn play(sound: &Option<String>, at: [f64; 3]) {
    if let Some(sound) = sound {
        emit_sound(sound, Some(at));
    }
}

impl Harpoons {
    pub fn load() -> Option<Self> {
        let rows = rows::load();
        (!rows.is_empty()).then(|| Self {
            rows,
            lines: RefCell::default(),
            charges: RefCell::default(),
            pulls: RefCell::default(),
        })
    }

    fn held(&self, state: &PlayerSnapshot) -> Option<&Row> {
        self.rows
            .iter()
            .find(|r| Some(r.id) == state.held)
            .filter(|_| !state.spectator && state.health > 0)
    }

    pub fn line_out(&self, player: PlayerId) -> bool {
        self.lines.borrow().contains_key(&player)
    }

    fn data(&self, row: &Row, stack: &ItemStackData) -> Vec<(String, Vec<u8>)> {
        stack
            .data
            .iter()
            .filter(|(k, _)| k != "petramond:item_entity")
            .cloned()
            .chain([("petramond:item_entity".to_owned(), row.rules.clone())])
            .collect()
    }

    fn launch(&self, row_index: usize, player: PlayerId, state: &PlayerSnapshot, ticks: u32) {
        let row = &self.rows[row_index];
        let Some(held) = player_held(player).filter(|s| s.item == row.name) else {
            return;
        };
        let reach = crate::projectile_aim::crosshair_reach(player, state);
        let (hand, direction) = crate::projectile_aim::from_hand(state, reach);
        let lead = row
            .lead
            .min(reach.map_or(f32::INFINITY, |r| (r - LEAD_CLEARANCE).max(0.0)));
        let from = crate::strike::offset_by(hand, direction.map(|d| d * lead));
        let speed = row.draw.launch_speed(ticks);
        let data = self.data(row, &held);
        let borrowed: Vec<(&str, &[u8])> = data
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();
        let entity = launch_item(
            &row.name,
            from,
            direction.map(|d| d * speed),
            Some(EntityRef::Player(player)),
            &borrowed,
        );
        if entity == 0 {
            return;
        }
        play(&row.sounds.throw, from);
        self.lines.borrow_mut().insert(
            player,
            Line {
                entity,
                row: row_index,
                hold: Hold::Flying,
            },
        );
        self.charges.borrow_mut().insert(entity, ticks);
        emit_event_to(player, LINE_EVENT, &[1]);
    }

    /// SERVER: a harpoon on a line struck something. Answers whether it was one.
    pub fn on_hit(
        &self,
        entity: u64,
        target: &ProjectileTarget,
        pos: [f64; 3],
        vel: [f32; 3],
        fate: &mut ProjectileFate,
        mut blocks: impl FnMut(PlayerId, [f64; 3]) -> bool,
    ) -> bool {
        let mut lines = self.lines.borrow_mut();
        let Some((&owner, line)) = lines.iter_mut().find(|(_, l)| l.entity == entity) else {
            return false;
        };
        let row = &self.rows[line.row];
        if line.hold == Hold::Reeling {
            *fate = ProjectileFate::Deflect { vel };
            return true;
        }
        let body = match target {
            ProjectileTarget::Block { .. } => {
                *fate = ProjectileFate::Lodge;
                line.hold = Hold::Stuck;
                play(&row.sounds.hit, pos);
                return true;
            }
            ProjectileTarget::Mob(id) => EntityRef::Mob(*id),
            ProjectileTarget::Player(id) => EntityRef::Player(*id),
        };
        let origin = std::array::from_fn(|axis| pos[axis] - f64::from(vel[axis]));
        if let EntityRef::Player(victim) = body {
            if blocks(victim, origin) {
                *fate = ProjectileFate::Drop;
                line.hold = Hold::Loose;
                return true;
            }
        }
        let ticks = self.charges.borrow_mut().remove(&entity).unwrap_or(1);
        let damage = strike::roll(row.damage(ticks), rng_u64("harpoon"));
        let attacker = Some(EntityRef::Player(owner));
        match body {
            EntityRef::Mob(mob) => {
                damage_mob(mob, damage, Some(pos), attacker);
            }
            EntityRef::Player(player) => {
                damage_player(player, damage.round() as i32, Some(pos), attacker);
            }
        }
        play(&row.sounds.hit, pos);
        *fate = ProjectileFate::Lodge;
        line.hold = Hold::Lodged(body);
        true
    }

    /// SERVER: a reel's hit on `mob` is about to land; pull it toward the thrower instead.
    pub fn on_mob_damage(&self, attacker: PlayerId, mob: u64, feedback: &mut MobDamageFeedback) {
        let mut pulls = self.pulls.borrow_mut();
        let Some(at) = pulls
            .iter()
            .position(|&(p, m, _)| p == attacker && m == mob)
        else {
            return;
        };
        let (_, _, pull) = pulls.remove(at);
        for component in &mut feedback.components {
            if let MobDamageFeedbackComponent::Knockback { scale, .. } = component {
                *scale = -pull * scale.abs().max(1.0);
            }
        }
    }

    /// SERVER: use while the line is out reels the harpoon in.
    fn reel(&self, owner: PlayerId, state: &PlayerSnapshot) {
        let mut lines = self.lines.borrow_mut();
        let Some(line) = lines.get_mut(&owner) else {
            return;
        };
        if line.hold == Hold::Reeling {
            return;
        }
        let row = &self.rows[line.row];
        let Some(item) = item_entity(line.entity) else {
            return;
        };
        let hand = hand_of(state);
        if let Hold::Lodged(body) = line.hold {
            let damage = strike::roll(row.reel_damage, rng_u64("harpoon"));
            let attacker = Some(EntityRef::Player(owner));
            match body {
                EntityRef::Mob(mob) => {
                    self.pulls.borrow_mut().push((owner, mob, row.pull));
                    damage_mob(mob, damage, Some(hand), attacker);
                }
                EntityRef::Player(victim) => {
                    damage_player(victim, damage.round() as i32, Some(hand), attacker);
                    let (impulse, _) = toward(item.pos, hand, row.pull * 4.0);
                    apply_knockback_to(victim, impulse);
                }
            }
        }
        play(&row.sounds.reel, item.pos);
        let Some(stack) = take_item_entity(line.entity) else {
            return;
        };
        let data = self.data(row, &stack);
        let borrowed: Vec<(&str, &[u8])> = data
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();
        let (vel, _) = toward(item.pos, hand, row.reel_speed);
        let entity = launch_item(
            &row.name,
            item.pos,
            vel,
            Some(EntityRef::Player(owner)),
            &borrowed,
        );
        if entity == 0 {
            lines.remove(&owner);
            emit_event_to(owner, LINE_EVENT, &[0]);
            return;
        }
        tow(entity, vel);
        line.entity = entity;
        line.hold = Hold::Reeling;
    }

    /// Cuts the rope: the harpoon stays where it is, free to be picked up, and leaves the pack.
    fn abandon(&self, owner: PlayerId, line: &Line) {
        set_item_rules(line.entity, None);
        take_item(owner, &self.rows[line.row].name, 1, None);
        emit_event_to(owner, LINE_EVENT, &[0]);
    }

    /// SERVER, every tick: keeps each line in hand, steers the reeled harpoons home and catches
    /// them, and cuts the lines their throwers let go of.
    pub fn step(&self, roster: &[PlayerListEntry]) {
        let mut lines = self.lines.borrow_mut();
        let mut ended = Vec::new();
        for (&owner, line) in lines.iter_mut() {
            let row = &self.rows[line.row];
            let state = roster
                .iter()
                .find(|p| p.id == owner)
                .map(|p| &p.state)
                .filter(|s| !s.spectator && s.health > 0 && s.held == Some(row.id));
            let Some(item) = item_entity(line.entity) else {
                ended.push((owner, false));
                continue;
            };
            let Some(state) = state else {
                ended.push((owner, true));
                continue;
            };
            let hand = hand_of(state);
            let (vel, distance) = toward(item.pos, hand, row.reel_speed);
            if distance > row.reach {
                ended.push((owner, true));
                continue;
            }
            match line.hold {
                Hold::Reeling if distance <= CATCH_DISTANCE => {
                    take_item_entity(line.entity);
                    play(&row.sounds.catch, hand);
                    ended.push((owner, false));
                }
                Hold::Reeling => {
                    if !tow(line.entity, vel) {
                        ended.push((owner, false));
                    }
                }
                Hold::Lodged(_) | Hold::Flying if item.motion == ItemMotion::Loose => {
                    line.hold = Hold::Loose;
                }
                _ => {}
            }
        }
        for (owner, cut) in ended {
            if let Some(line) = lines.remove(&owner) {
                if cut {
                    self.abandon(owner, &line);
                } else {
                    emit_event_to(owner, LINE_EVENT, &[0]);
                }
            }
        }
        drop(lines);
        self.charges
            .borrow_mut()
            .retain(|entity, _| self.lines.borrow().values().any(|l| l.entity == *entity));
    }
}

pub struct HarpoonRule {
    harpoons: Rc<Harpoons>,
}

impl HarpoonRule {
    pub fn new(harpoons: Rc<Harpoons>) -> Self {
        Self { harpoons }
    }

    fn row_of(&self, clocks: &BodyClocks, state: &PlayerSnapshot) -> Option<(usize, &Row)> {
        let id = clocks.harpoon.item().or(state.held)?;
        self.harpoons
            .rows
            .iter()
            .enumerate()
            .find(|(_, r)| r.id == id)
    }
}

impl Rule for HarpoonRule {
    fn takes_press(&self, state: &PlayerSnapshot) -> bool {
        self.harpoons.held(state).is_some()
    }

    fn step(
        &self,
        clocks: &mut BodyClocks,
        player: PlayerId,
        state: &PlayerSnapshot,
        press: bool,
        dt_ticks: f32,
        authority: bool,
    ) {
        if authority {
            clocks.harpoon_line = self.harpoons.line_out(player);
        }
        let edge = press && !clocks.harpoon_pressed;
        clocks.harpoon_pressed = press;
        let alive = !state.spectator && state.health > 0;
        let held = self.row_of(clocks, state).filter(|_| alive);
        match held {
            Some((_, row)) => {
                let dt = dt_ticks.max(0.0) * TICK_SECONDS;
                advance(&mut clocks.harpoon_reel, dt, row.line.reel.length());
                advance(&mut clocks.harpoon_catch, dt, row.line.catch.length());
            }
            None => (clocks.harpoon_reel, clocks.harpoon_catch) = (None, None),
        }
        if clocks.harpoon_line {
            if edge && !clocks.harpoon_reeled {
                clocks.harpoon = Clock::default();
                clocks.harpoon_reeled = true;
                clocks.harpoon_reel = Some(0.0);
                if authority {
                    self.harpoons.reel(player, state);
                }
            } else if matches!(clocks.harpoon.pose(), Some(throw::Pose::Throw(_))) {
                // The throw plays out, whenever the line is confirmed: it ends in the line's hold.
                let row = held.map(|(_, row)| row);
                clocks.harpoon.step(row, state.held, alive, false, dt_ticks);
            } else {
                clocks.harpoon = Clock::default();
            }
            return;
        }
        if std::mem::take(&mut clocks.harpoon_reeled) && held.is_some() {
            clocks.harpoon_catch = Some(0.0);
        }
        let Some((index, row)) = self.row_of(clocks, state) else {
            clocks.harpoon = Clock::default();
            return;
        };
        let released = clocks
            .harpoon
            .step(Some(row), state.held, alive, press, dt_ticks);
        if let (Some(ticks), true) = (released, authority) {
            self.harpoons.launch(index, player, state, ticks);
        }
    }

    fn claims(&self, body: &Body) -> Claims {
        let row = self.row_of(body.clocks, body.state).map(|(_, row)| row);
        let mut claims = throw::claims(row, &body.clocks.harpoon, CLAIM_PARAM, true);
        let (Some(row), clocks) = (row, body.clocks) else {
            return claims;
        };
        if clocks.harpoon_line {
            throw::empty_hand(&mut claims.params);
            claims.holds_press = true;
            claims.denied = vec![BodyAction::Attack, BodyAction::Mine];
        }
        if claims.plays.is_empty() {
            let gesture = match (
                clocks.harpoon_line,
                clocks.harpoon_reel,
                clocks.harpoon_catch,
            ) {
                (true, Some(t), _) => Some((&row.line.reel, t)),
                (true, None, _) => Some((&row.line.hold, 0.0)),
                (false, _, Some(t)) => Some((&row.line.catch, t)),
                (false, _, None) => None,
            };
            if let Some((gesture, t)) = gesture {
                gesture.claim(t, CLAIM_PARAM, &mut claims);
            }
        }
        claims
    }
}
