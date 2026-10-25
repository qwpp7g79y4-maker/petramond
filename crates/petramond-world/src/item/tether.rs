//! The line a launched item trails back to whoever launched it (a fishing
//! line): stack data `petramond:item_entity` → `tether`, an object whose
//! `tile`, `tint` and `width` (texels) are each optional.

use serde::Deserialize;

use super::variant::{self, VariantId};
use crate::tile::Tile;

/// The stack data key carrying an item entity's rules.
pub const ITEM_ENTITY_DATA_KEY: &str = "petramond:item_entity";

const DEFAULT_TILE: &str = "wool_block";
const DEFAULT_TINT: [u8; 3] = [222, 214, 200];
const DEFAULT_WIDTH: f32 = 0.5;
const MAX_WIDTH: f32 = 4.0;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TetherStyle {
    pub tile: Tile,
    pub tint: [u8; 3],
    /// Thickness in texels (sixteenths of a block).
    pub width: f32,
}

#[derive(Deserialize)]
struct RawRules {
    #[serde(default)]
    tether: Option<RawTether>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RawTether {
    tile: Option<String>,
    tint: Option<[u8; 3]>,
    width: Option<f32>,
}

impl TetherStyle {
    /// The tether a stack launched with these instance data trails, if any. An unknown tile or
    /// an unusable width falls back to the default rather than dropping the line.
    pub fn of(variant: VariantId) -> Option<TetherStyle> {
        let data = variant::get(variant)?;
        let rules: RawRules = serde_json::from_slice(data.get(ITEM_ENTITY_DATA_KEY)?).ok()?;
        let raw = rules.tether?;
        Some(TetherStyle {
            tile: raw
                .tile
                .as_deref()
                .and_then(Tile::from_name)
                .or_else(|| Tile::from_name(DEFAULT_TILE))?,
            tint: raw.tint.unwrap_or(DEFAULT_TINT),
            width: raw
                .width
                .filter(|w| w.is_finite() && *w > 0.0)
                .unwrap_or(DEFAULT_WIDTH)
                .min(MAX_WIDTH),
        })
    }
}
