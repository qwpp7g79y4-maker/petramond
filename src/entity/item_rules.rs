use petramond_world::item::{variant, ItemStack};
use serde::Deserialize;

/// Rules carried with an item through flight, lodging, dropping and save restoration.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub(super) struct ItemRules {
    pub pickup: bool,
    pub lifetime_ticks: Option<u32>,
    /// Present when the item trails a line back to its launcher; the style is the client's.
    pub tether: Option<serde::de::IgnoredAny>,
}

impl Default for ItemRules {
    fn default() -> Self {
        Self {
            pickup: true,
            lifetime_ticks: None,
            tether: None,
        }
    }
}

impl ItemRules {
    pub fn of(stack: ItemStack) -> Self {
        variant::get(stack.variant)
            .and_then(|data| {
                data.get(petramond_world::item::ITEM_ENTITY_DATA_KEY)
                    .and_then(|raw| serde_json::from_slice(raw).ok())
            })
            .unwrap_or_default()
    }
}
