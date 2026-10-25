use petramond_math::world_pos::WorldPos;
use petramond_world::light::BlockLight6;

use crate::waterline::Waterline;

pub(super) const FULL_SKYLIGHT: u8 = 63;

pub(crate) const SKY_MIN: f32 = 0.02;
pub(crate) const FINAL_MIN: f32 = 0.006;

#[inline]
pub(super) fn skylight_bits(skylight: u8) -> u32 {
    (skylight.min(FULL_SKYLIGHT) as u32) << petramond_mesh::SKY_SHIFT
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct DynLight {
    pub sky: u8,
    pub block: BlockLight6,
    /// Where the light was sampled, when the lit thing is in the world (for the underwater
    /// correction); `None` for icons and other worldless lighting.
    pub at: Option<WorldPos>,
}

impl DynLight {
    pub(super) const FULL: Self = Self {
        sky: FULL_SKYLIGHT,
        block: BlockLight6::DARK,
        at: None,
    };

    #[cfg(test)]
    pub(super) fn new(sky: u8, block: BlockLight6) -> Self {
        Self {
            sky,
            block,
            at: None,
        }
    }

    /// Light sampled for something at `at`.
    #[inline]
    pub(super) fn at(sky: u8, block: BlockLight6, at: WorldPos) -> Self {
        Self {
            sky,
            block,
            at: Some(at),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LightEnv {
    pub sky_scale: f32,
    pub sky_color: [f32; 3],
    /// With the eye in or at water that has optics: the light below its surface.
    pub water: Option<WaterLight>,
}

/// The eye's water, for lighting what lies under its surface as seen from under it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WaterLight {
    /// Absolute height of the open surface the eye's depth is measured from.
    pub surface_y: f64,
    /// The water's absorption per block, by channel.
    pub absorb: [f32; 3],
    /// The share of `absorb` that tints light per block of depth.
    pub shift: f32,
    /// The share of daylight lost per block of depth.
    pub depth_dim: f32,
    pub eye: WorldPos,
    /// Which rays from the eye set out under the water; `None` = all (what the eye holds).
    pub rays: Option<Waterline>,
}

impl WaterLight {
    /// The tint of skylight that crossed `depth` blocks of this water, its strongest channel kept
    /// whole. Mirrors `underwater_hue` in petramond::underwater.
    pub(crate) fn hue_at(&self, depth: f32) -> [f32; 3] {
        let filtered = self
            .absorb
            .map(|a| (-a * depth.max(0.0) * self.shift).exp());
        let top = filtered.iter().copied().fold(f32::MIN, f32::max).max(1e-6);
        filtered.map(|f| f / top)
    }
}

impl LightEnv {
    pub(super) const IDENTITY: Self = Self {
        sky_scale: 1.0,
        sky_color: [1.0, 1.0, 1.0],
        water: None,
    };

    /// The light on what the eye holds: it sits at the eye, so it takes the eye's medium rather
    /// than the per-ray waterline test.
    pub(super) fn held(self) -> Self {
        Self {
            water: self.water.map(|water| WaterLight {
                rays: None,
                ..water
            }),
            ..self
        }
    }
}

/// The skylight flood's loss per block of water, as a share of full skylight.
const UNDERWATER_SKY_STEP: f32 = 2.0 / 30.0;
/// The 6-bit skylight a point in deep open water keeps past the flood's reach.
const UNDERWATER_DEEP_SKY6: u8 =
    petramond_world::light::level6(petramond_world::light::DEEP_WATER_SKY);

/// The skylight curve (before the sky scale) of `light` and its tint: the cube of its level, or
/// below the eye's water surface and seen along a ray that sets out under it, what open water
/// passes at that depth (losing `depth_dim` per block) times the share of open water's flood the
/// point received to the 1.5th power (never measured below the deep-water step), tinted by the
/// water above. Mirrors
/// `underwater_sky_curve` and `underwater_hue` in petramond::underwater.
#[inline]
fn sky_curve(light: DynLight, env: &LightEnv) -> (f32, [f32; 3]) {
    let x = light.sky.min(FULL_SKYLIGHT) as f32 / FULL_SKYLIGHT as f32;
    let air = (x * x * x, [1.0; 3]);
    let (Some(water), Some(at)) = (env.water, light.at) else {
        return air;
    };
    let depth = (water.surface_y - at.y) as f32;
    if depth <= 0.0 || water.rays.is_some_and(|rays| !rays.under(at - water.eye)) {
        return air;
    }
    let open = (1.0 - depth * UNDERWATER_SKY_STEP)
        .max(f32::from(UNDERWATER_DEEP_SKY6) / FULL_SKYLIGHT as f32);
    let reached = (x / open).clamp(0.0, 1.0);
    (
        reached * reached.sqrt() * (-depth * water.depth_dim).exp(),
        water.hue_at(depth),
    )
}

#[inline]
fn curve(x: f32, scale: f32) -> f32 {
    SKY_MIN + (1.0 - SKY_MIN) * (x * x * x * scale)
}

#[cfg(test)]
const SKY_GAMMA: i32 = 3;

#[cfg(test)]
const WGSL_BLOCK_TERM: &str = "mix(vec3<f32>(SKY_MIN), vec3<f32>(1.0), blk * blk * blk)";

#[inline]
pub(super) fn light_rgb(light: DynLight, env: LightEnv) -> [f32; 3] {
    let (sky, hue) = sky_curve(light, &env);
    let sky_term = SKY_MIN + (1.0 - SKY_MIN) * sky * env.sky_scale.clamp(0.0, 1.0);
    let block = light.block.fractions();
    let mut out = [0.0f32; 3];
    for (i, (o, c)) in out.iter_mut().zip(env.sky_color).enumerate() {
        *o = (sky_term * c * hue[i])
            .max(curve(block[i], 1.0))
            .max(FINAL_MIN);
    }
    out
}

#[inline]
pub(super) fn mul3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

#[inline]
pub(super) fn fold_self_lit(light: [f32; 3], self_lit: f32) -> [f32; 3] {
    let s = self_lit.clamp(0.0, 1.0);
    light.map(|c| c + (1.0 - c) * s)
}

#[inline]
pub(super) fn fold_tint(base: [f32; 3], light: DynLight, env: LightEnv) -> [f32; 3] {
    mul3(base, light_rgb(light, env))
}

#[inline]
pub(super) fn fold_tint_self_lit(
    base: [f32; 3],
    light: DynLight,
    env: LightEnv,
    self_lit: f32,
) -> [f32; 3] {
    mul3(base, fold_self_lit(light_rgb(light, env), self_lit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_channel_is_night_invariant() {
        let torchlit = DynLight {
            sky: 0,
            block: BlockLight6::grey(60),
            at: None,
        };
        let night = LightEnv {
            sky_scale: 0.0,
            sky_color: [1.0, 1.0, 1.0],
            water: None,
        };
        assert_eq!(
            light_rgb(torchlit, night),
            light_rgb(torchlit, LightEnv::IDENTITY)
        );

        let skylit = DynLight {
            sky: 63,
            block: BlockLight6::DARK,
            at: None,
        };
        let day = light_rgb(skylit, LightEnv::IDENTITY);
        let dark = light_rgb(skylit, night);
        assert!(dark[0] < day[0], "sky-only light must dim with the scale");
    }

    #[test]
    fn sky_color_tints_only_the_sky_term() {
        let env = LightEnv {
            sky_scale: 1.0,
            sky_color: [0.75, 0.82, 1.0],
            water: None,
        };
        let sky = light_rgb(DynLight::new(63, BlockLight6::DARK), env);
        assert!(sky[0] < sky[2], "red must dim below blue under a blue sky");
        let torch = light_rgb(DynLight::new(0, BlockLight6::grey(63)), env);
        assert_eq!(torch[0], torch[2], "white block light is colour-neutral");
    }

    fn wgsl_const(src: &str, name: &str) -> f32 {
        let needle = format!("const {name}: f32 = ");
        let at = src
            .find(&needle)
            .unwrap_or_else(|| panic!("no `{name}` declaration"))
            + needle.len();
        let end = src[at..]
            .find(';')
            .unwrap_or_else(|| panic!("unterminated `{name}`"));
        src[at..at + end]
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("`{name}` is not a float literal: {e}"))
    }

    /// Entities and hands under the water are lit like the terrain around them: the same
    /// correction as the shader's `underwater_sky_curve`.
    #[test]
    fn under_water_a_lit_point_takes_open_waters_light_and_shelter_dims_it() {
        assert_eq!(
            wgsl_const(
                include_str!("../shaders/underwater.wgsl"),
                "UNDERWATER_DEEP_SKY6"
            ),
            f32::from(UNDERWATER_DEEP_SKY6),
            "underwater.wgsl mirrors the deep-water step"
        );
        let env = LightEnv {
            water: Some(WaterLight {
                surface_y: 63.0,
                absorb: [0.13, 0.06, 0.05],
                shift: 1.0,
                depth_dim: 0.045,
                eye: WorldPos::new(0.0, 62.0, 0.0),
                rays: None,
            }),
            ..LightEnv::IDENTITY
        };
        let depth = 10.0;
        // The flood under open water at this depth, as the 6-bit level the world samples.
        let open = ((1.0 - depth as f32 * UNDERWATER_SKY_STEP) * 63.0).round() as u8;
        let at = |sky| {
            DynLight::at(
                sky,
                BlockLight6::DARK,
                WorldPos::new(0.0, 63.0 - depth, 0.0),
            )
        };
        let lit = light_rgb(at(open), env)[1];
        let raw = light_rgb(at(open), LightEnv::IDENTITY)[1];
        assert!(
            lit > raw * 3.0,
            "open water {depth} down: {lit} vs the raw flood's {raw}"
        );
        let shade = [open, open / 2, open / 4, 0].map(|sky| light_rgb(at(sky), env)[1]);
        assert!(
            shade.windows(2).all(|w| w[0] > w[1]),
            "less of the flood under shelter, less light: {shade:?}"
        );
        // Past the flood's reach both read dark but for the deep-water step.
        let past_reach = |sky| {
            light_rgb(
                DynLight::at(sky, BlockLight6::DARK, WorldPos::new(0.0, 63.0 - 24.0, 0.0)),
                env,
            )[1]
        };
        let (deep, cave) = (past_reach(UNDERWATER_DEEP_SKY6), past_reach(0));
        assert!(
            deep > lit * 0.3 && cave < deep * 0.1,
            "deep open water {deep} vs a cave {cave} at the same depth"
        );
        let above = DynLight::at(open, BlockLight6::DARK, WorldPos::new(0.0, 70.0, 0.0));
        assert_eq!(light_rgb(above, env), light_rgb(above, LightEnv::IDENTITY));
        let worldless = DynLight::new(open, BlockLight6::DARK);
        assert_eq!(
            light_rgb(worldless, env),
            light_rgb(worldless, LightEnv::IDENTITY)
        );
    }

    /// Under the water only skylight crossed the water above: it loses red with depth, while
    /// block light starts beside what it lights and keeps its colour.
    #[test]
    fn under_water_skylight_takes_the_waters_hue_and_block_light_keeps_its_own() {
        let env = LightEnv {
            water: Some(WaterLight {
                surface_y: 63.0,
                absorb: [0.13, 0.06, 0.05],
                shift: 1.0,
                depth_dim: 0.045,
                eye: WorldPos::new(0.0, 62.0, 0.0),
                rays: None,
            }),
            ..LightEnv::IDENTITY
        };
        let at = |sky, block, y| DynLight::at(sky, block, WorldPos::new(0.0, y, 0.0));
        let red = |c: [f32; 3]| c[0] / c[2];
        let shallow = light_rgb(at(63, BlockLight6::DARK, 61.0), env);
        let deep = light_rgb(at(63, BlockLight6::DARK, 43.0), env);
        assert!(
            red(deep) < red(shallow) && red(shallow) < 1.0,
            "{shallow:?} then {deep:?}"
        );
        let above = at(63, BlockLight6::DARK, 70.0);
        assert_eq!(light_rgb(above, env), light_rgb(above, LightEnv::IDENTITY));
        let amber = at(0, BlockLight6::new(63, 45, 26), 43.0);
        assert_eq!(
            light_rgb(amber, env),
            light_rgb(amber, LightEnv::IDENTITY),
            "block light under water keeps its colour"
        );
    }

    #[test]
    fn every_light_shader_spells_the_same_curve() {
        for (name, src) in [
            ("block.wgsl", include_str!("../shaders/block.wgsl")),
            ("model3d.wgsl", include_str!("../shaders/model3d.wgsl")),
            ("mob.wgsl", include_str!("../shaders/mob.wgsl")),
        ] {
            assert_eq!(wgsl_const(src, "SKY_MIN"), SKY_MIN, "{name} SKY_MIN");
            assert_eq!(wgsl_const(src, "FINAL_MIN"), FINAL_MIN, "{name} FINAL_MIN");
            assert_eq!(
                wgsl_const(src, "SKY_GAMMA"),
                SKY_GAMMA as f32,
                "{name} SKY_GAMMA"
            );
            assert!(
                src.contains(WGSL_BLOCK_TERM),
                "{name} no longer floors the block term PER CHANNEL as \
                 `{WGSL_BLOCK_TERM}` — a saturated colour's weak channels can \
                 now render darker than an unlit cave"
            );
        }
        for level in 0..=63u8 {
            let x = level as f32 / FULL_SKYLIGHT as f32;
            assert_eq!(
                curve(x, 1.0),
                SKY_MIN + (1.0 - SKY_MIN) * x.powi(SKY_GAMMA),
                "the Rust mirror is no longer the shaders' curve at level {level}"
            );
        }
    }

    #[test]
    fn coloured_block_light_rides_the_cave_floor_per_channel() {
        let purple = light_rgb(
            DynLight::new(0, BlockLight6::new(40, 0, 63)),
            LightEnv::IDENTITY,
        );
        let unlit = light_rgb(DynLight::new(0, BlockLight6::DARK), LightEnv::IDENTITY);
        assert!(purple[2] > purple[0], "blue must outshine red");
        assert!(purple[0] > purple[1], "red must outshine the dead green");
        for c in 0..3 {
            assert!(
                purple[c] >= unlit[c],
                "channel {c} fell below the unlit floor: {purple:?} vs {unlit:?}"
            );
        }
    }
}
