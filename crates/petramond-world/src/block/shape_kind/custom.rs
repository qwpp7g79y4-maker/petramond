use serde::Deserialize;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomLight {
    Open,
    OpaqueCube,
    CustomAperture,
}

#[derive(Debug, PartialEq)]
pub struct CustomShapeDef {
    pub key: &'static str,
    pub light_shape: CustomLight,
    pub nav_solid: bool,
    pub grass_decay_eligible: bool,
    pub state_key: Option<&'static str>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCustomShapeDef {
    key: String,
    #[serde(default = "default_light")]
    light_shape: CustomLight,
    #[serde(default)]
    nav_profile: Option<String>,
    #[serde(default = "default_true")]
    grass_decay_eligible: bool,
    #[serde(default)]
    state_key: Option<String>,
}

fn default_light() -> CustomLight {
    CustomLight::Open
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct RawCustomShapeFile {
    shapes: Vec<RawCustomShapeDef>,
}

pub(crate) static CUSTOM_SHAPES: crate::content::Slot<&'static [CustomShapeDef]> =
    crate::content::Slot::new(crate::content::stage::SHAPES, &[], load);

fn load(reg: &crate::content::ContentRegistry) -> Result<&'static [CustomShapeDef], String> {
    let layers = reg.packs().read_layers("shapes.json");
    if layers.is_empty() {
        return Ok(&[]);
    }
    let texts: Vec<&str> = layers.iter().map(|(s, _)| s.as_str()).collect();
    crate::registry::load_catalog(
        &texts,
        |t| serde_json::from_str::<RawCustomShapeFile>(t).map(|f| f.shapes),
        |r| &r.key,
        &[],
        "shape",
        |r, id, names| {
            Ok(CustomShapeDef {
                key: names.name(id).expect("id resolved from this table"),
                light_shape: r.light_shape,
                nav_solid: matches!(r.nav_profile.as_deref(), Some("solid")),
                grass_decay_eligible: r.grass_decay_eligible,
                state_key: r.state_key.map(|k| -> &'static str { String::leak(k) }),
            })
        },
    )
    .map(|catalog| catalog.rows())
}

fn defs() -> &'static [CustomShapeDef] {
    CUSTOM_SHAPES.current()
}

pub(super) fn by_key(key: &str) -> Option<&'static CustomShapeDef> {
    defs().iter().find(|d| d.key == key)
}
