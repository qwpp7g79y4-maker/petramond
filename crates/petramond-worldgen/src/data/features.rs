use petramond_world::content::{ContentRegistry, Slot};
use petramond_world::registry::Catalog;

use serde::Deserialize;

use crate::feature::placers::foliage::{
    ConiferFoliage, DroopyFoliage, FlatSparseFoliage, FoliagePlacer, LayeredFoliage,
};
use crate::feature::placers::trunk::{LeaningTrunk, StraightTrunk, TrunkPlacer, WhorledTrunk};
use crate::feature::tree::{BlockyOakFeature, CanopyTreeFeature, RedwoodFeature, TreeFeature};
use crate::feature::{ConfiguredFeature, Feature};
use petramond_world::block::Block;

macro_rules! engine_features {
    ($($accessor:ident => $name:literal),+ $(,)?) => {
        const ENGINE_FEATURE_NAMES: &[&str] = &[$($name),+];

        $(
            #[doc = concat!("The `", $name, "` feature row.")]
            pub fn $accessor() -> &'static ConfiguredFeature {
                engine($name)
            }
        )+
    };
}

engine_features! {
    oak_young => "petramond:oak_young",
    oak_small => "petramond:oak_small",
    oak_swamp => "petramond:oak_swamp",
    oak_big => "petramond:oak_big",
    redwood => "petramond:redwood",
    spruce => "petramond:spruce",
    birch => "petramond:birch",
    jungle => "petramond:jungle",
    acacia => "petramond:acacia",
}

static STRAIGHT: StraightTrunk = StraightTrunk;
static LEANING: LeaningTrunk = LeaningTrunk;

pub struct FeatureDef {
    pub configured: ConfiguredFeature,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFeatureDef {
    feature: String,
    shape: RawShape,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum RawShape {
    BlockyOak(BlockyOakFeature),
    Canopy(CanopyTreeFeature),
    Redwood(RedwoodFeature),
    Tree(RawTree),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTree {
    trunk: RawTrunk,
    foliage: RawFoliage,
    log: Block,
    leaf: Block,
    height: (i32, i32),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum RawTrunk {
    Straight,
    Leaning,
    Whorled(WhorledTrunk),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum RawFoliage {
    Droopy(DroopyFoliage),
    Conifer(ConiferFoliage),
    FlatSparse(FlatSparseFoliage),
    Layered(LayeredFoliage),
}

impl RawShape {
    fn resolve(self) -> Result<&'static dyn Feature, String> {
        Ok(match self {
            RawShape::BlockyOak(f) => {
                f.validate()?;
                Box::leak(Box::new(f))
            }
            RawShape::Canopy(f) => {
                f.validate()?;
                Box::leak(Box::new(f))
            }
            RawShape::Redwood(f) => {
                f.validate()?;
                Box::leak(Box::new(f))
            }
            RawShape::Tree(t) => {
                crate::data::bounds::ascending("height", t.height, 5..=56)?;
                let trunk: &'static dyn TrunkPlacer = match t.trunk {
                    RawTrunk::Straight => &STRAIGHT,
                    RawTrunk::Leaning => &LEANING,
                    RawTrunk::Whorled(trunk) => {
                        trunk.validate(t.height)?;
                        Box::leak(Box::new(trunk))
                    }
                };
                let foliage: &'static dyn FoliagePlacer = match t.foliage {
                    RawFoliage::Droopy(f) => Box::leak(Box::new(f)),
                    RawFoliage::Conifer(f) => Box::leak(Box::new(f)),
                    RawFoliage::FlatSparse(f) => Box::leak(Box::new(f)),
                    RawFoliage::Layered(f) => {
                        f.validate()?;
                        Box::leak(Box::new(f))
                    }
                };
                let reach = foliage.horizontal_reach() + trunk.max_lean();
                if reach > crate::biome::trees::MAX_TREE_SPACING_RADIUS {
                    return Err(format!(
                        "foliage reach + trunk lean: {reach} blocks exceed the {} block candidate window",
                        crate::biome::trees::MAX_TREE_SPACING_RADIUS
                    ));
                }
                Box::leak(Box::new(TreeFeature {
                    trunk,
                    foliage,
                    log: t.log,
                    leaf: t.leaf,
                    height: t.height,
                }))
            }
        })
    }
}

#[derive(Deserialize)]
struct RawFile {
    features: Vec<RawFeatureDef>,
}

pub(crate) static CATALOG: Slot<Catalog<FeatureDef>> = Slot::new(
    "features.json",
    &[petramond_world::content::stage::BLOCKS],
    load,
);

fn load(reg: &ContentRegistry) -> Result<Catalog<FeatureDef>, String> {
    let table = petramond_world::registry::read_catalog(
        reg.packs(),
        "features.json",
        "worldgen feature",
        parse_layers,
    )?;
    let mut unknown = Vec::new();
    for block in petramond_world::block::Block::all() {
        for (key, _) in block.grows_into() {
            if table.id(key).is_none() {
                unknown.push(format!(
                    "blocks.json: '{}' grows_into names unknown worldgen feature '{key}'",
                    reg.names().blocks.name(block.id()).unwrap_or("?")
                ));
            }
        }
    }
    if !unknown.is_empty() {
        return Err(unknown.join("\n"));
    }
    Ok(table)
}

fn catalog() -> &'static Catalog<FeatureDef> {
    CATALOG.current()
}

fn parse_layers(texts: &[&str]) -> Result<petramond_world::registry::Catalog<FeatureDef>, String> {
    petramond_world::registry::load_catalog(
        texts,
        |text| serde_json::from_str::<RawFile>(text).map(|f| f.features),
        |r| &r.feature,
        ENGINE_FEATURE_NAMES,
        "worldgen feature",
        |r, id, names| {
            let name = names.name(id).expect("id resolved from this table");
            Ok(FeatureDef {
                configured: ConfiguredFeature {
                    feature: r
                        .shape
                        .resolve()
                        .map_err(|e| format!("worldgen feature '{name}': {e}"))?,
                },
            })
        },
    )
}

pub fn by_name(name: &str) -> Option<&'static ConfiguredFeature> {
    let c = catalog();
    c.id(name).map(|id| &c.rows()[id as usize].configured)
}

fn engine(name: &str) -> &'static ConfiguredFeature {
    by_name(name).unwrap_or_else(|| panic!("engine feature {name} is not loaded"))
}

#[cfg(test)]
mod growth_target_tests {
    #[test]
    fn every_block_growth_target_resolves() {
        for block in petramond_world::block::Block::all() {
            for (key, _) in block.grows_into() {
                assert!(
                    super::by_name(key).is_some(),
                    "block row grows_into '{key}' missing from features.json"
                );
            }
        }
    }
}
