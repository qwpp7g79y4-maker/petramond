use crate::world::ServerWorld;
use petramond_math::math::{IVec3, Vec3};
use petramond_world::fluid::{Buoyancy, FluidCurrent, Immersion};
use petramond_world::item::ItemStack;

use super::{hash01, hash_signed};

pub const GRAVITY: f32 = -20.0;

pub const ITEM_HALF_EXTENT: f32 = 0.125;

pub const ABSORB_RADIUS: f32 = 0.6;

pub const ATTRACT_RADIUS: f32 = 1.5;

const MAGNET_BASE_SPEED: f32 = 6.0;
const MAGNET_RAMP_SPEED: f32 = 10.0;

const GROUND_DAMP_PER_SEC: f32 = 6.0;
const AIR_DAMP_PER_SEC: f32 = 0.8;
const SPIN_SPEED: f32 = 2.0;

const THROW_SPEED: f32 = 4.0;
const THROW_UP: f32 = 1.5;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Heading {
    pub yaw: f32,
    pub pitch: f32,
}

impl Heading {
    pub fn of(vel: Vec3) -> Option<Heading> {
        let horizontal = (vel.x * vel.x + vel.z * vel.z).sqrt();
        if horizontal < 1e-6 && vel.y.abs() < 1e-6 {
            return None;
        }
        Some(Heading {
            yaw: vel.x.atan2(vel.z),
            pitch: vel.y.atan2(horizontal),
        })
    }

    pub fn dir(self) -> Vec3 {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        Vec3::new(sy * cp, sp, cy * cp)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Flight {
    pub owner: Option<crate::mob::EntityRef>,
    pub left_owner: bool,
    pub heading: Heading,
    pub contact: Option<crate::mob::EntityRef>,
    pub control_ticks: Option<u8>,
    /// The heading a steering controller holds, in place of the motion's.
    pub facing: Option<Heading>,
}

impl Flight {
    fn turn_with(&mut self, vel: Vec3) {
        let held = self.facing.filter(|_| self.control_ticks.is_some());
        if let Some(heading) = held.or_else(|| Heading::of(vel)) {
            self.heading = heading;
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Stuck {
    pub flight: Flight,
    pub anchor: IVec3,
    pub verified: bool,
}

/// Lodged in a living body: rides it at `offset` and `yaw` in the body's turning frame.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Embedded {
    pub flight: Flight,
    pub body: crate::mob::EntityRef,
    pub offset: Vec3,
    pub yaw: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Motion {
    Loose,
    Flight(Flight),
    Stuck(Stuck),
    Embedded(Embedded),
}

/// Turns `v` about +Y so a heading of yaw `y` comes out at `y + by`.
pub fn turn_y(v: Vec3, by: f32) -> Vec3 {
    let (s, c) = by.sin_cos();
    Vec3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}

impl Motion {
    pub fn flying(vel: Vec3, owner: Option<crate::mob::EntityRef>) -> Motion {
        Motion::Flight(Flight {
            owner,
            left_owner: owner.is_none(),
            heading: Heading::of(vel).unwrap_or(Heading {
                yaw: 0.0,
                pitch: 0.0,
            }),
            contact: None,
            control_ticks: None,
            facing: None,
        })
    }

    pub fn projectile(self) -> Option<Flight> {
        match self {
            Motion::Flight(f) => Some(f),
            Motion::Stuck(s) => Some(s.flight),
            Motion::Embedded(e) => Some(e.flight),
            Motion::Loose => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Fate {
    Consume,
    Lodge,
    Drop,
    Deflect { vel: Vec3 },
}

impl Fate {
    pub fn of_impact(sticks: bool, struck_block: bool) -> Fate {
        if sticks && struck_block {
            Fate::Lodge
        } else {
            Fate::Drop
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DroppedItem {
    pub id: u64,
    pub pos: petramond_math::world_pos::WorldPos,
    pub vel: Vec3,
    pub stack: ItemStack,
    pub skylight: u8,
    pub blocklight: petramond_world::light::BlockLight6,
    pub ticks_lived: u32,
    pub pickup_requested: Option<crate::player::PlayerId>,
    pub spin: f32,
    pub motion: Motion,
    pub prev_pos: petramond_math::world_pos::WorldPos,
    pub prev_spin: f32,
    /// Who the item's line runs back to, for an item launched on a tether.
    pub tether: Option<crate::mob::EntityRef>,
    /// Prims a mod draws in place of the item's own art.
    pub draw: crate::world::draw::BodyDraw,
    rules: super::item_rules::ItemRules,
    escape: petramond_world::collision::EscapeRoute,
}

impl DroppedItem {
    pub(crate) fn with_motion(
        pos: petramond_math::world_pos::WorldPos,
        stack: ItemStack,
        vel: Vec3,
        motion: Motion,
    ) -> Self {
        DroppedItem {
            id: 0,
            pos,
            vel,
            stack,
            skylight: 63,
            blocklight: petramond_world::light::BlockLight6::DARK,
            ticks_lived: 0,
            pickup_requested: None,
            spin: 0.0,
            motion,
            prev_pos: pos,
            prev_spin: 0.0,
            tether: None,
            draw: Default::default(),
            rules: super::item_rules::ItemRules::of(stack),
            escape: Default::default(),
        }
    }

    pub fn new(pos: petramond_math::world_pos::WorldPos, stack: ItemStack, seed: u32) -> Self {
        let s = seed as u64;
        let ang = hash01(s) * std::f32::consts::TAU;
        let speed = 1.0 + hash01(s ^ 0x5151) * 1.5;
        let up = 2.5 + hash01(s ^ 0xA2A2) * 1.5;
        let vel = Vec3::new(ang.cos() * speed, up, ang.sin() * speed);
        let mut item = Self::with_motion(pos, stack, vel, Motion::Loose);
        item.spin = hash_signed(s ^ 0x3C3C) * std::f32::consts::PI;
        item.prev_spin = item.spin;
        item
    }

    pub fn thrown(pos: petramond_math::world_pos::WorldPos, stack: ItemStack, dir: Vec3) -> Self {
        let d = dir.normalize_or_zero();
        let vel = Vec3::new(
            d.x * THROW_SPEED,
            d.y * THROW_SPEED + THROW_UP,
            d.z * THROW_SPEED,
        );
        Self::with_motion(pos, stack, vel, Motion::Loose)
    }

    pub fn launched(
        pos: petramond_math::world_pos::WorldPos,
        stack: ItemStack,
        vel: Vec3,
        owner: Option<crate::mob::EntityRef>,
    ) -> Self {
        let mut item = Self::with_motion(pos, stack, vel, Motion::flying(vel, owner));
        item.tether = owner.filter(|_| item.rules.tether.is_some());
        item
    }

    /// Replaces the item's `petramond:item_entity` rules, keeping its other data; `None` drops
    /// them, so it behaves like a plain stack. A line, once let go, is not picked back up.
    pub fn set_rules(&mut self, rules: Option<&[u8]>) -> Result<(), String> {
        use petramond_world::item::{variant, ITEM_ENTITY_DATA_KEY};
        let parsed = match rules {
            Some(raw) => serde_json::from_slice::<super::item_rules::ItemRules>(raw)
                .map_err(|e| format!("item rules: {e}"))?,
            None => Default::default(),
        };
        let mut data = variant::get(self.stack.variant)
            .map(|m| (*m).clone())
            .unwrap_or_default();
        match rules {
            Some(raw) => data.insert(ITEM_ENTITY_DATA_KEY.to_owned(), raw.to_vec()),
            None => data.remove(ITEM_ENTITY_DATA_KEY),
        };
        self.stack.variant = if data.is_empty() {
            variant::VariantId::NONE
        } else {
            variant::intern(&data).map_err(|e| format!("item rules: {e:?}"))?
        };
        self.rules = parsed;
        if parsed.tether.is_none() {
            self.tether = None;
        }
        Ok(())
    }

    pub fn heading(&self) -> Option<Heading> {
        self.motion.projectile().map(|f| f.heading)
    }

    pub fn collectable(&self) -> bool {
        self.rules.pickup && matches!(self.motion, Motion::Loose | Motion::Stuck(_))
    }

    pub fn lifetime_ticks(&self) -> Option<u32> {
        self.rules.lifetime_ticks
    }

    pub fn lodge(&mut self, anchor: IVec3) {
        if let Some(flight) = self.motion.projectile() {
            self.motion = Motion::Stuck(Stuck {
                flight,
                anchor,
                verified: true,
            });
        } else {
            self.motion = Motion::Loose;
        }
        self.vel = Vec3::ZERO;
    }

    /// Lodges in `body`, whose frame is at `at` turned by `yaw`, keeping the item's pose there.
    pub fn embed(
        &mut self,
        body: crate::mob::EntityRef,
        at: petramond_math::world_pos::WorldPos,
        yaw: f32,
    ) {
        let Some(flight) = self.motion.projectile() else {
            self.motion = Motion::Loose;
            return;
        };
        self.motion = Motion::Embedded(Embedded {
            flight,
            body,
            offset: turn_y(self.pos - at, -yaw),
            yaw: flight.heading.yaw - yaw,
        });
        self.vel = Vec3::ZERO;
    }

    /// Follows the body an embedded item is lodged in; falls loose when the body is gone.
    pub fn ride(&mut self, frame: Option<(petramond_math::world_pos::WorldPos, f32)>) {
        self.prev_pos = self.pos;
        self.prev_spin = self.spin;
        let Motion::Embedded(e) = &mut self.motion else {
            return;
        };
        match frame {
            Some((at, yaw)) => {
                self.pos = at + turn_y(e.offset, yaw);
                e.flight.heading.yaw = e.yaw + yaw;
            }
            None => {
                self.vel = Vec3::ZERO;
                self.release();
            }
        }
    }

    pub fn release(&mut self) {
        self.motion = Motion::Loose;
    }

    pub fn deflect(&mut self, vel: Vec3) {
        if let Motion::Flight(flight) = &mut self.motion {
            self.vel = vel;
            flight.turn_with(vel);
        }
    }

    pub fn steer(&mut self, vel: Option<Vec3>) -> bool {
        let Some(vel) = vel else {
            self.release();
            self.vel = Vec3::ZERO;
            return true;
        };
        if let Motion::Flight(flight) = &mut self.motion {
            flight.control_ticks = Some(2);
            self.deflect(vel);
            true
        } else {
            false
        }
    }

    /// Holds a steered flight's heading along `facing` (`None`: along its motion again).
    pub fn face(&mut self, facing: Option<Vec3>) -> bool {
        let Motion::Flight(flight) = &mut self.motion else {
            return false;
        };
        if flight.control_ticks.is_none() {
            return false;
        }
        flight.facing = facing.and_then(Heading::of);
        flight.turn_with(self.vel);
        true
    }

    pub fn request_pickup(&mut self, by: crate::player::PlayerId) {
        self.pickup_requested = Some(by);
    }

    pub fn clear_pickup_request(&mut self) {
        self.pickup_requested = None;
    }

    pub fn step_loose(
        &mut self,
        dt: f32,
        world: &ServerWorld,
        magnet_target: Option<petramond_math::world_pos::WorldPos>,
    ) {
        debug_assert!(matches!(self.motion, Motion::Loose));
        let cells = world.cursor();
        let boxes = |x: i32, y: i32, z: i32| cells.collision_boxes_xyz(x, y, z);
        let feet = self.pos - Vec3::Y * ITEM_HALF_EXTENT;
        let immersion = world
            .data()
            .body_fluid(feet, 2.0 * ITEM_HALF_EXTENT, Buoyancy::Swim);
        let current = world.data().fluid_current_at(self.pos);
        self.integrate_with_flow(dt, magnet_target, &boxes, immersion, current);
    }

    pub fn advance_flight(&mut self, dt: f32) -> Option<Vec3> {
        let Motion::Flight(flight) = &mut self.motion else {
            return None;
        };
        self.prev_pos = self.pos;
        self.prev_spin = self.spin;
        let params = self.stack.item.projectile();
        self.spin = (self.spin + params.sprite_spin * dt).rem_euclid(std::f32::consts::TAU);
        // A steered flight moves at the velocity its controller set: the controller is its physics.
        if flight.control_ticks.is_none() {
            self.vel.y -= params.gravity * dt;
            self.vel *= params.drag_factor(dt);
        }
        flight.turn_with(self.vel);
        Some(self.vel * dt)
    }

    #[cfg(test)]
    fn integrate(
        &mut self,
        dt: f32,
        magnet_target: Option<petramond_math::world_pos::WorldPos>,
        solid_at: &impl Fn(IVec3) -> bool,
    ) {
        let boxes = |x: i32, y: i32, z: i32| {
            if solid_at(IVec3::new(x, y, z)) {
                petramond_world::block::Block::Stone.collision_boxes()
            } else {
                &[][..]
            }
        };
        self.integrate_with_flow(dt, magnet_target, &boxes, None, FluidCurrent::NONE);
    }

    fn integrate_with_flow(
        &mut self,
        dt: f32,
        magnet_target: Option<petramond_math::world_pos::WorldPos>,
        boxes: &impl Fn(i32, i32, i32) -> &'static [petramond_world::block::Aabb],
        immersion: Option<Immersion>,
        current: FluidCurrent,
    ) {
        self.prev_pos = self.pos;
        self.prev_spin = self.spin;
        self.spin = (self.spin + SPIN_SPEED * dt) % std::f32::consts::TAU;

        if let Some(target) = magnet_target {
            let to = target - self.pos;
            let dist = to.length();
            if dist <= ATTRACT_RADIUS {
                if dist > 1e-4 {
                    let closeness = 1.0 - (dist / ATTRACT_RADIUS);
                    let speed = MAGNET_BASE_SPEED + MAGNET_RAMP_SPEED * closeness;
                    self.vel = Vec3::ZERO;
                    let step = (speed * dt).min(dist);
                    self.pos += (to / dist) * step;
                }
                return;
            }
        }

        if let Some(sample) = immersion {
            self.vel = sample
                .fluid
                .motion
                .horizontal_velocity(self.vel, Vec3::ZERO, dt);
            self.vel.y = sample.vertical_velocity(
                self.vel.y,
                (self.pos.y - f64::from(ITEM_HALF_EXTENT)) as f32,
                Buoyancy::Swim,
                false,
                dt,
            );
        } else {
            self.vel.y += GRAVITY * dt;
        }
        self.vel = current.apply(self.vel, dt);

        let h = f64::from(ITEM_HALF_EXTENT);
        let min = [self.pos.x - h, self.pos.y - h, self.pos.z - h];
        let max = [self.pos.x + h, self.pos.y + h, self.pos.z + h];
        let (moved, grounded, hit) = petramond_world::collision::resolve_body(
            min,
            max,
            self.vel.to_array(),
            dt,
            0.0,
            &mut self.escape,
            boxes,
        );
        self.pos += Vec3::from(moved);
        if hit[0] {
            self.vel.x = 0.0;
        }
        if hit[1] {
            self.vel.y = 0.0;
        }
        if hit[2] {
            self.vel.z = 0.0;
        }

        let damp = if grounded {
            GROUND_DAMP_PER_SEC
        } else {
            AIR_DAMP_PER_SEC
        };
        let k = (1.0 - damp * dt).clamp(0.0, 1.0);
        self.vel.x *= k;
        self.vel.z *= k;
        if grounded && self.vel.y < 0.0 {
            self.vel.y = 0.0;
        }
    }

    #[inline]
    pub fn within_pickup(&self, player_pos: petramond_math::world_pos::WorldPos) -> bool {
        self.pos.distance_squared(player_pos) <= f64::from(ABSORB_RADIUS * ABSORB_RADIUS)
    }

    #[inline]
    pub fn within_attract(&self, player_pos: petramond_math::world_pos::WorldPos) -> bool {
        self.pos.distance_squared(player_pos) <= f64::from(ATTRACT_RADIUS * ATTRACT_RADIUS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petramond_math::world_pos::WorldPos;
    use petramond_world::item::ItemType;

    fn stack() -> ItemStack {
        ItemStack::new(ItemType::Dirt, 1)
    }

    fn empty(_p: IVec3) -> bool {
        false
    }

    fn boxes_of(
        solid: impl Fn(IVec3) -> bool,
    ) -> impl Fn(i32, i32, i32) -> &'static [petramond_world::block::Aabb] {
        move |x, y, z| {
            if solid(IVec3::new(x, y, z)) {
                petramond_world::block::Block::Stone.collision_boxes()
            } else {
                &[]
            }
        }
    }

    #[test]
    fn a_current_pushes_item_entities_along() {
        let mut d = DroppedItem::new(WorldPos::new(0.5, 1.5, 0.5), stack(), 15);
        d.vel = Vec3::ZERO;
        let flow = |p: WorldPos| {
            if p.y.floor() as i32 == 1 {
                Vec3::Z
            } else {
                Vec3::ZERO
            }
        };

        d.integrate_with_flow(
            0.1,
            None,
            &boxes_of(empty),
            None,
            FluidCurrent {
                velocity: flow(d.pos),
                accel: 9.0,
            },
        );

        assert!(d.vel.z > 0.0, "current should add +Z velocity: {}", d.vel.z);
        assert!(d.pos.z > 0.5, "current should move the item: {}", d.pos.z);
    }

    #[test]
    fn item_rests_on_an_inset_block_top_not_the_cell_top() {
        let chest = petramond_world::block::Block::Chest.collision_boxes();
        let chest_top = chest.iter().map(|b| b.max[1]).fold(0.0, f32::max);
        assert!(
            chest_top < 1.0,
            "the chest box must actually be inset (top {chest_top})"
        );
        let boxes = |_x: i32, y: i32, _z: i32| if y == 0 { chest } else { &[][..] };
        let mut d = DroppedItem::new(WorldPos::new(0.5, 3.0, 0.5), stack(), 1);
        d.vel = Vec3::ZERO;
        for _ in 0..300 {
            d.integrate_with_flow(1.0 / 60.0, None, &boxes, None, FluidCurrent::NONE);
        }
        assert!(
            (d.pos.y - f64::from(ITEM_HALF_EXTENT) - f64::from(chest_top)).abs() < 0.02,
            "item should rest on the chest top {chest_top}, got bottom {}",
            d.pos.y - f64::from(ITEM_HALF_EXTENT)
        );
    }

    #[test]
    fn magnet_does_not_overshoot_target() {
        let target = WorldPos::new(0.0, 0.0, 0.0);
        let start = WorldPos::new(0.0, 1.0, 0.0);
        let mut d = DroppedItem::new(start, stack(), 13);
        d.vel = Vec3::ZERO;
        d.integrate(10.0, Some(target), &empty);
        let dist = (d.pos - target).length();
        assert!(
            dist <= 1.0 + 1e-4,
            "item should not overshoot the target: {dist}"
        );
    }

    #[test]
    fn magnet_leaves_no_velocity_to_fling_a_released_drop() {
        let mut d = DroppedItem::new(WorldPos::new(0.0, 1.0, 0.0), stack(), 1);
        d.vel = Vec3::new(5.0, 5.0, 5.0);
        let target = WorldPos::ZERO;
        d.integrate(1.0 / 60.0, Some(target), &empty);
        assert_eq!(
            d.vel,
            Vec3::ZERO,
            "a magnetised drop carries no velocity to fling when released"
        );
    }

    #[test]
    fn far_target_leaves_physics_untouched() {
        let far = WorldPos::new(100.0, 0.0, 0.0);
        let mut d = DroppedItem::new(WorldPos::new(0.0, 50.0, 0.0), stack(), 14);
        d.vel = Vec3::ZERO;
        let before = d.pos.y;
        d.integrate(0.1, Some(far), &empty);
        assert!(
            d.pos.y < before,
            "free fall should still apply outside attract range"
        );
        assert!(d.vel.y < 0.0);
    }

    #[test]
    fn an_embedded_item_rides_its_body_and_falls_loose_when_it_is_gone() {
        let body = WorldPos::new(10.0, 64.0, 10.0);
        let mut it = DroppedItem::launched(
            body + Vec3::new(0.0, 1.0, -0.5),
            stack(),
            Vec3::new(0.0, 0.0, 8.0),
            None,
        );
        it.embed(crate::mob::EntityRef::Mob(7), body, 0.0);
        assert!(matches!(it.motion, Motion::Embedded(_)));
        assert!(
            !it.collectable(),
            "a lodged item is not picked off the body"
        );

        let moved = body + Vec3::new(3.0, 0.0, 0.0);
        it.ride(Some((moved, std::f32::consts::FRAC_PI_2)));
        let rel = it.pos - moved;
        assert!(
            (rel - Vec3::new(-0.5, 1.0, 0.0)).length() < 1e-4,
            "the quarter turn carries the offset round: {rel}"
        );
        let heading = it.heading().expect("a lodged item keeps its heading");
        assert!(
            (heading.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-4,
            "and turns its heading with the body: {}",
            heading.yaw
        );

        it.ride(None);
        assert_eq!(it.motion, Motion::Loose, "no body, no hold");
    }

    #[test]
    fn a_steered_flight_holds_its_facing_until_the_steering_lets_go() {
        let mut it = DroppedItem::launched(
            WorldPos::new(0.0, 64.0, 0.0),
            stack(),
            Vec3::new(0.0, 0.0, 8.0),
            None,
        );
        let yaw = |it: &DroppedItem| it.heading().expect("in flight").yaw;
        assert!(
            !it.face(Some(-Vec3::Z)),
            "only a steered flight takes a facing"
        );

        let home = Vec3::new(0.0, 0.0, -8.0);
        assert!(it.steer(Some(home)) && it.face(Some(-home)));
        it.advance_flight(0.05);
        assert!(yaw(&it).abs() < 1e-4, "faces away from where it goes");
        it.steer(Some(Vec3::new(8.0, 0.0, 0.0)));
        it.advance_flight(0.05);
        assert!(yaw(&it).abs() < 1e-4, "whichever way it is steered");

        it.face(None);
        assert!(
            (yaw(&it) - std::f32::consts::FRAC_PI_2).abs() < 1e-4,
            "and turns with its motion again"
        );
    }

    #[test]
    fn rules_set_after_launch_cut_the_line_and_clear_back_to_a_plain_stack() {
        let rules = br#"{"pickup":false,"tether":{}}"#;
        let data = [(
            petramond_world::item::ITEM_ENTITY_DATA_KEY.to_owned(),
            rules.to_vec(),
        )]
        .into_iter()
        .collect();
        let variant = petramond_world::item::variant::intern(&data).expect("interns");
        let launched = ItemStack::with_variant(ItemType::Dirt, 1, variant);
        let mut it = DroppedItem::launched(
            WorldPos::new(0.0, 64.0, 0.0),
            launched,
            Vec3::X,
            Some(crate::mob::EntityRef::Player(crate::player::PlayerId(1))),
        );
        assert!(it.tether.is_some());
        it.release();
        assert!(!it.collectable());

        it.set_rules(None).expect("clears");
        assert_eq!(it.tether, None, "the line is cut");
        assert!(it.collectable());
        assert_eq!(it.stack, stack(), "and the stack is plain again");
        assert!(it.set_rules(Some(b"not json")).is_err());
    }
}
