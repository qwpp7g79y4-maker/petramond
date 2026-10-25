use serde::Deserialize;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Effect(pub u8);

#[allow(non_upper_case_globals)]
impl Effect {
    pub const Regeneration: Effect = Effect(0);
}

const ENGINE_EFFECT_NAMES: &[&str] = &["petramond:regeneration"];

impl std::fmt::Debug for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match ENGINE_EFFECT_NAMES.get(self.0 as usize) {
            Some(name) => write!(f, "Effect({name})"),
            None => write!(f, "Effect(#{})", self.0),
        }
    }
}

impl Effect {
    #[inline]
    pub fn def(self) -> &'static EffectDef {
        &defs()[self.0 as usize]
    }

    pub fn all() -> impl Iterator<Item = Effect> {
        (0..defs().len()).map(|id| Effect(id as u8))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EffectBehavior {
    None,
    Regen { interval: u32, amount: i32 },
    Speed { scale: f32 },
}

impl EffectBehavior {
    #[inline]
    pub fn speed_scale(self) -> f32 {
        match self {
            Self::Speed { scale } => scale,
            _ => 1.0,
        }
    }
}

pub struct EffectDef {
    pub effect: Effect,
    pub name: &'static str,
    #[allow(dead_code)]
    pub display: &'static str,
    pub icon: &'static str,
    pub behavior: EffectBehavior,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ActiveEffect {
    pub effect: Effect,
    pub remaining: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEffectDef {
    effect: String,
    display: String,
    icon: String,
    behavior: RawBehavior,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawBehavior {
    None,
    Regen { interval: u32, amount: i32 },
    Speed { scale: f32 },
}

const SPEED_SCALE_MAX: f32 = 5.0;

impl RawBehavior {
    fn resolve(&self, effect: &str) -> Result<EffectBehavior, String> {
        match *self {
            RawBehavior::None => Ok(EffectBehavior::None),
            RawBehavior::Regen { interval, amount } => {
                if interval == 0 || amount <= 0 {
                    return Err(format!(
                        "effect '{effect}': regen interval and amount must be positive"
                    ));
                }
                Ok(EffectBehavior::Regen { interval, amount })
            }
            RawBehavior::Speed { scale } => {
                if !scale.is_finite() || scale <= 0.0 || scale > SPEED_SCALE_MAX {
                    return Err(format!(
                        "effect '{effect}': speed scale must be in (0, {SPEED_SCALE_MAX}], got {scale}"
                    ));
                }
                Ok(EffectBehavior::Speed { scale })
            }
        }
    }
}

#[derive(Deserialize)]
struct RawFile {
    effects: Vec<RawEffectDef>,
}

pub fn by_name(name: &str) -> Option<Effect> {
    catalog().id(name).map(|id| Effect(id as u8))
}

pub fn defs() -> &'static [EffectDef] {
    catalog().rows()
}

pub(crate) static CATALOG: crate::content::Slot<crate::registry::Catalog<EffectDef>> =
    crate::content::Slot::new(crate::content::stage::EFFECTS, &[], load);

fn load(
    reg: &crate::content::ContentRegistry,
) -> Result<crate::registry::Catalog<EffectDef>, String> {
    crate::registry::read_catalog(reg.packs(), "effects.json", "effect", parse_layers)
}

fn catalog() -> &'static crate::registry::Catalog<EffectDef> {
    CATALOG.current()
}

fn parse_layers(texts: &[&str]) -> Result<crate::registry::Catalog<EffectDef>, String> {
    crate::registry::load_catalog(
        texts,
        |text| serde_json::from_str::<RawFile>(text).map(|f| f.effects),
        |r| &r.effect,
        ENGINE_EFFECT_NAMES,
        "effect",
        |r, id, names| {
            let behavior = r.behavior.resolve(&r.effect)?;
            if r.icon.is_empty() {
                return Err(format!("effect '{}': icon path is empty", r.effect));
            }
            Ok(EffectDef {
                effect: Effect(id as u8),
                name: names.name(id).expect("id resolved from this table"),
                display: Box::leak(r.display.into_boxed_str()),
                icon: Box::leak(r.icon.into_boxed_str()),
                behavior,
            })
        },
    )
}
