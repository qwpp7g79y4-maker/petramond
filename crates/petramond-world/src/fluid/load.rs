use super::{
    contact::RawContact, CurrentProperties, FluidDef, FluidMedium, FluidMotion, FluidSplash,
    FluidSubmerged, Quench,
};
use crate::{block::Block, registry::ContentNames};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawFluid {
    delay: u64,
    drop_off: u8,
    #[serde(default)]
    renewable: bool,
    #[serde(default)]
    floods: bool,
    #[serde(default)]
    quench: Option<RawQuench>,
    motion: FluidMotion,
    #[serde(default)]
    current: CurrentProperties,
    #[serde(default)]
    splash: Option<RawSplash>,
    #[serde(default)]
    submerged: Option<RawSubmerged>,
    #[serde(default)]
    contact: Option<RawContact>,
    medium: FluidMedium,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct RawQuench {
    by: String,
    result: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct RawSplash {
    burst: String,
    #[serde(default)]
    below: Option<String>,
    sound_small: String,
    sound_big: String,
}

fn burst_id(key: &str, what: &str) -> Result<u8, String> {
    crate::particle_emitters::by_key(key)
        .filter(|bundle| bundle.burst.is_some())
        .map(|bundle| bundle.id)
        .ok_or_else(|| format!("fluid {what} '{key}' is not a burst bundle"))
}

impl RawSplash {
    fn resolve(self) -> Result<FluidSplash, String> {
        let burst = burst_id(&self.burst, "splash burst")?;
        let below = self
            .below
            .map(|key| burst_id(&key, "splash below"))
            .transpose()?;
        let sound = |key: &str| {
            crate::sound_registry::by_name(key)
                .ok_or_else(|| format!("fluid splash names unknown sound '{key}'"))
        };
        Ok(FluidSplash {
            burst,
            below,
            sound_small: sound(&self.sound_small)?,
            sound_big: sound(&self.sound_big)?,
        })
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct RawSubmerged {
    #[serde(default)]
    exhale: Option<String>,
    #[serde(default)]
    ambience: Option<String>,
    #[serde(default)]
    muffle: f32,
    #[serde(default)]
    release: Option<String>,
    #[serde(default)]
    wake: Option<String>,
}

impl RawSubmerged {
    fn resolve(self) -> Result<FluidSubmerged, String> {
        let exhale = self
            .exhale
            .map(|key| {
                crate::particle_emitters::by_key(&key)
                    .filter(|bundle| !bundle.rows.is_empty())
                    .map(|bundle| bundle.id)
                    .ok_or_else(|| format!("fluid exhale '{key}' is not a looping bundle"))
            })
            .transpose()?;
        let ambience = self
            .ambience
            .map(|key| {
                crate::sound_registry::by_name(&key)
                    .ok_or_else(|| format!("fluid ambience names unknown sound '{key}'"))
            })
            .transpose()?;
        if !self.muffle.is_finite() || !(0.0..=1.0).contains(&self.muffle) {
            return Err("fluid submerged.muffle must be in 0..=1".into());
        }
        let release = self
            .release
            .map(|key| burst_id(&key, "submerged.release"))
            .transpose()?;
        let wake = self
            .wake
            .map(|key| {
                crate::particle_emitters::by_key(&key)
                    .filter(|bundle| !bundle.rows.is_empty())
                    .map(|bundle| bundle.id)
                    .ok_or_else(|| format!("fluid wake '{key}' is not a looping bundle"))
            })
            .transpose()?;
        Ok(FluidSubmerged {
            exhale,
            ambience,
            muffle: self.muffle,
            release,
            wake,
        })
    }
}

impl RawFluid {
    pub(crate) fn resolve(self, block: Block, names: &ContentNames) -> Result<FluidDef, String> {
        if self.delay == 0 || self.delay > 1200 || !(1..=7).contains(&self.drop_off) {
            return Err("fluid delay must be 1..=1200 ticks and drop_off 1..=7".into());
        }
        let m = self.motion;
        for (name, value, max) in [
            ("speed_scale", m.speed_scale, 4.0),
            ("accel", m.accel, 1000.0),
            ("friction", m.friction, 1.0),
            ("rise", m.rise, 30.0),
            ("sink", m.sink, 30.0),
            ("vertical_accel", m.vertical_accel, 1000.0),
            ("entry_friction", m.entry_friction, 1.0),
            ("probe_fraction", m.probe_fraction, 1.0),
            ("probe_offset", m.probe_offset, 1.0),
            ("current.speed", self.current.speed, 30.0),
            ("current.accel", self.current.accel, 1000.0),
        ] {
            if !value.is_finite() || !(0.0..=max).contains(&value) {
                return Err(format!("fluid {name} must be finite and in 0..={max}"));
            }
        }
        let resolve = |key: &str| {
            names
                .blocks
                .id(key)
                .map(Block)
                .ok_or_else(|| format!("unknown fluid reaction block '{key}'"))
        };
        let quench = self
            .quench
            .map(|q| -> Result<_, String> {
                Ok(Quench {
                    by: resolve(&q.by)?,
                    result: resolve(&q.result)?,
                })
            })
            .transpose()?;
        self.medium.validate()?;
        if self.floods && self.medium.is_opaque() {
            return Err("floods requires a see-through medium".into());
        }
        let contact = self
            .contact
            .map(RawContact::resolve)
            .transpose()?
            .unwrap_or_default();
        let name = names
            .blocks
            .name(block.id())
            .ok_or_else(|| "fluid block has no registered name".to_owned())?;
        Ok(FluidDef {
            block,
            name,
            delay: self.delay,
            drop_off: self.drop_off,
            renewable: self.renewable,
            floods: self.floods,
            quench,
            motion: m,
            current: self.current,
            splash: self.splash.map(RawSplash::resolve).transpose()?,
            submerged: self
                .submerged
                .map(RawSubmerged::resolve)
                .transpose()?
                .unwrap_or_default(),
            contact,
            medium: self.medium,
        })
    }
}
