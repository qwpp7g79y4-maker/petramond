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

pub struct EffectDef {
    pub effect: Effect,
    pub name: &'static str,
    #[allow(dead_code)]
    pub display: &'static str,
    pub icon: &'static str,
    /// Body attribute scales multiplied in while the effect is active.
    pub attributes: crate::attribute::AttributeScales,
    /// Health restored every `interval` ticks while active.
    pub heal: Option<crate::condition::Pulse>,
}

impl EffectDef {
    /// This effect's factor on `attribute` (1 when it does not touch it).
    pub fn scale(&self, attribute: mod_api::PlayerAttribute) -> f32 {
        crate::attribute::scale(self.attributes, attribute)
    }
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
    #[serde(default)]
    attributes: std::collections::BTreeMap<String, f32>,
    #[serde(default)]
    heal: Option<crate::condition::RawPulse>,
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
            let what = format!("effect '{}'", r.effect);
            let attributes = crate::attribute::resolve(&r.attributes, &what)?;
            let heal = r
                .heal
                .map(|pulse| pulse.resolve().map_err(|e| format!("{what}: heal: {e}")))
                .transpose()?;
            if r.icon.is_empty() {
                return Err(format!("effect '{}': icon path is empty", r.effect));
            }
            Ok(EffectDef {
                effect: Effect(id as u8),
                name: names.name(id).expect("id resolved from this table"),
                display: Box::leak(r.display.into_boxed_str()),
                icon: Box::leak(r.icon.into_boxed_str()),
                attributes,
                heal,
            })
        },
    )
}
