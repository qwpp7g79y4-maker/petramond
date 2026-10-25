use crate::block::Block;
use crate::mathh::Vec3;

pub mod contact;
mod eye;
pub mod held;
pub(crate) mod load;
pub mod medium;
mod motion;
mod query;
pub use contact::{ConditionGrant, FluidContact};
pub use medium::{AlbedoMix, FluidMedium, FluidOptics};
pub use query::sample_body_current;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Buoyancy {
    #[default]
    Swim,
    Surface,
    Neutral,
    /// A heavy body: sinks to the floor and walks it. A drive's vertical speed swims it up or
    /// down for that tick instead.
    Sink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FluidClimb {
    Jump,
    Step,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FluidMotion {
    pub speed_scale: f32,
    pub accel: f32,
    pub friction: f32,
    pub rise: f32,
    pub sink: f32,
    pub vertical_accel: f32,
    pub entry_friction: f32,
    pub probe_fraction: f32,
    pub probe_offset: f32,
    pub climb: FluidClimb,
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentProperties {
    pub speed: f32,
    pub accel: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Quench {
    pub by: Block,
    pub result: Block,
}

/// How it presents to a body whose breath point is under this fluid.
#[derive(Clone, Copy, Debug, Default)]
pub struct FluidSubmerged {
    /// Looping body emitter worn while the body holds its breath here.
    pub exhale: Option<u8>,
    /// Loop the listener hears while their head is under it.
    pub ambience: Option<crate::sound_registry::Sound>,
    /// How muffled the world sounds to a head under it (0 = clear, 1 = fully).
    pub muffle: f32,
    /// Burst a container under it lets out as it opens.
    pub release: Option<u8>,
    /// Looping emitter worn by an item moving through it.
    pub wake: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct FluidSplash {
    pub burst: u8,
    /// Burst under the surface where the body went in, at the splash's intensity.
    pub below: Option<u8>,
    pub sound_small: crate::sound_registry::Sound,
    pub sound_big: crate::sound_registry::Sound,
}

#[derive(Debug)]
pub struct FluidDef {
    pub block: Block,
    pub name: &'static str,
    pub delay: u64,
    pub drop_off: u8,
    pub renewable: bool,
    /// A block that can hold a fluid takes this one in: placed into a source of it, or poured
    /// into from a bucket.
    pub floods: bool,
    pub quench: Option<Quench>,
    pub motion: FluidMotion,
    pub current: CurrentProperties,
    pub splash: Option<FluidSplash>,
    pub submerged: FluidSubmerged,
    pub contact: FluidContact,
    pub medium: FluidMedium,
}

#[derive(Clone, Copy, Debug)]
pub struct Immersion {
    pub fluid: &'static FluidDef,
    pub surface_y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FluidCurrent {
    pub velocity: Vec3,
    pub accel: f32,
}

impl FluidCurrent {
    pub const NONE: Self = Self {
        velocity: Vec3::ZERO,
        accel: 0.0,
    };
}
