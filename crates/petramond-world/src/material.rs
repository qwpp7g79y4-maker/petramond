//! Block materials (`materials.json`) and tool kinds (`tool_kinds.json`): what a block sounds
//! like, which tool works it, and how each kind of tool mines and hits. Engine rows use bare
//! names (`stone`, `pickaxe`); a pack adds its own under its namespace.

use serde::Deserialize;

use crate::block::sounds::{BlockSoundSet, RawSoundSet};
use crate::registry::Catalog;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToolKind(pub u8);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialId(pub u8);

/// Damage per tool tier: tiers 1..=3 by row, tier 4 and above share the last entry, tier 0 is a
/// fist.
#[derive(Clone, Debug)]
pub struct ToolKindDef {
    pub name: &'static str,
    pub efficiency: f32,
    pub damage: [(f32, f32); 4],
}

#[derive(Clone, Debug)]
pub struct MaterialDef {
    pub name: &'static str,
    pub sounds: BlockSoundSet,
    pub tool: Option<ToolKind>,
    /// The preferred tool cuts this material outright, whatever its hardness (foliage).
    pub cut: bool,
    /// Tags every block of this material carries.
    pub tags: &'static [crate::block::BlockTag],
}

const ENGINE_TOOL_KINDS: &[&str] = &["pickaxe", "axe", "shovel", "shears", "sword"];

const ENGINE_MATERIALS: &[&str] = &[
    "none", "dirt", "sand", "snow", "stone", "ore", "wood", "wool", "foliage", "plant", "glass",
    "ice", "other",
];

impl ToolKind {
    #[inline]
    pub fn def(self) -> &'static ToolKindDef {
        &tool_kinds().rows()[self.0 as usize]
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    #[inline]
    pub fn mining_efficiency(self) -> f32 {
        self.def().efficiency
    }

    pub fn by_name(name: &str) -> Option<ToolKind> {
        tool_kinds().id(name).map(|id| ToolKind(id as u8))
    }
}

impl<'de> Deserialize<'de> for ToolKind {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        ToolKind::by_name(&name)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown tool kind '{name}'")))
    }
}

impl MaterialId {
    #[inline]
    pub fn def(self) -> &'static MaterialDef {
        &materials().rows()[self.0 as usize]
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    pub fn by_name(name: &str) -> Option<MaterialId> {
        materials().id(name).map(|id| MaterialId(id as u8))
    }
}

#[derive(Deserialize)]
struct RawToolKinds {
    tool_kinds: Vec<RawToolKind>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawToolKind {
    tool_kind: String,
    efficiency: f32,
    damage: [[f32; 2]; 4],
}

#[derive(Deserialize)]
struct RawMaterials {
    materials: Vec<RawMaterial>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMaterial {
    material: String,
    #[serde(default)]
    sounds: RawSoundSet,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    cut: bool,
    #[serde(default)]
    tags: Vec<String>,
}

pub(crate) static TOOL_KINDS: crate::content::Slot<Catalog<ToolKindDef>> =
    crate::content::Slot::new(crate::content::stage::TOOL_KINDS, &[], |reg| {
        crate::registry::read_catalog(reg.packs(), "tool_kinds.json", "tool kind", |texts| {
            parse_tool_kinds(texts)
        })
    });

pub(crate) static MATERIALS: crate::content::Slot<Catalog<MaterialDef>> = crate::content::Slot::new(
    crate::content::stage::MATERIALS,
    &[
        crate::content::stage::SOUNDS,
        crate::content::stage::TOOL_KINDS,
    ],
    |reg| {
        crate::registry::read_catalog(reg.packs(), "materials.json", "material", |texts| {
            parse_materials(texts)
        })
    },
);

fn tool_kinds() -> &'static Catalog<ToolKindDef> {
    TOOL_KINDS.current()
}

fn materials() -> &'static Catalog<MaterialDef> {
    MATERIALS.current()
}

fn parse_tool_kinds(texts: &[&str]) -> Result<Catalog<ToolKindDef>, String> {
    crate::registry::load_catalog(
        texts,
        |text| serde_json::from_str::<RawToolKinds>(text).map(|f| f.tool_kinds),
        |r| &r.tool_kind,
        ENGINE_TOOL_KINDS,
        "tool kind",
        |r, id, names| {
            let name = names.name(id).expect("id resolved from this table");
            if !(r.efficiency.is_finite() && r.efficiency > 0.0) {
                return Err(format!("tool kind '{name}': efficiency must be positive"));
            }
            let mut damage = [(0.0, 0.0); 4];
            for (slot, [lo, hi]) in damage.iter_mut().zip(r.damage) {
                if !(lo.is_finite() && hi.is_finite() && 0.0 <= lo && lo <= hi) {
                    return Err(format!("tool kind '{name}': damage needs 0 <= low <= high"));
                }
                *slot = (lo, hi);
            }
            Ok(ToolKindDef {
                name,
                efficiency: r.efficiency,
                damage,
            })
        },
    )
}

fn parse_materials(texts: &[&str]) -> Result<Catalog<MaterialDef>, String> {
    crate::registry::load_catalog(
        texts,
        |text| serde_json::from_str::<RawMaterials>(text).map(|f| f.materials),
        |r| &r.material,
        ENGINE_MATERIALS,
        "material",
        |r, id, names| {
            let name = names.name(id).expect("id resolved from this table");
            let tool = r
                .tool
                .as_deref()
                .map(|t| {
                    ToolKind::by_name(t)
                        .ok_or_else(|| format!("material '{name}': unknown tool kind '{t}'"))
                })
                .transpose()?;
            if r.cut && tool.is_none() {
                return Err(format!("material '{name}': 'cut' needs a 'tool'"));
            }
            Ok(MaterialDef {
                name,
                sounds: r
                    .sounds
                    .resolve()
                    .map_err(|e| format!("material '{name}': {e}"))?,
                tool,
                cut: r.cut,
                tags: Box::leak(
                    r.tags
                        .iter()
                        .map(|t| crate::block::BlockTag::resolve(t))
                        .collect::<Result<Vec<_>, String>>()?
                        .into_boxed_slice(),
                ),
            })
        },
    )
}
