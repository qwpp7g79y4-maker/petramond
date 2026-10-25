use std::collections::BTreeMap;

use mod_api::PlayerAttribute;
use serde::{Deserialize, Serialize};

use crate::inventory::WearSlot;

/// An item that goes on the body: the slot it fills and what wearing it does.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Wear {
    pub slot: WearSlot,
    /// Scales multiplied into the wearer's body attributes while it is worn.
    pub attributes: &'static [(PlayerAttribute, f32)],
    /// A GUI texture laid over the wearer's own first-person view.
    pub overlay: Option<&'static str>,
    /// What the wearer shows, in the model's `head` display pose; without one a model or block
    /// item shows itself and a sprite item shows nothing.
    pub model: Option<crate::block_model::BlockModelKind>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawWear {
    pub slot: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<crate::block_model::BlockModelKind>,
}

impl RawWear {
    pub(super) fn resolve(&self) -> Result<Wear, String> {
        let slot = WearSlot::from_key(&self.slot).ok_or_else(|| {
            let known: Vec<&str> = WearSlot::ALL.iter().map(|s| s.key()).collect();
            format!(
                "wear slot '{}' is not one of {}",
                self.slot,
                known.join(", ")
            )
        })?;
        let attributes = self
            .attributes
            .iter()
            .map(|(key, &scale)| {
                let attribute = PlayerAttribute::from_key(key)
                    .ok_or_else(|| format!("wear attribute '{key}' is not a body attribute"))?;
                if !scale.is_finite() || scale < 0.0 {
                    return Err(format!(
                        "wear attribute '{key}' scale {scale} must be finite and non-negative"
                    ));
                }
                Ok((attribute, scale))
            })
            .collect::<Result<Vec<_>, String>>()?;
        if self.overlay.as_deref().is_some_and(str::is_empty) {
            return Err("wear overlay path is empty".into());
        }
        Ok(Wear {
            slot,
            attributes: Box::leak(attributes.into_boxed_slice()),
            overlay: self
                .overlay
                .clone()
                .map(|path| &*Box::leak(path.into_boxed_str())),
            model: self.model,
        })
    }
}

impl Wear {
    /// This item's factor on `attribute` (1 when it does not touch it).
    pub fn scale(&self, attribute: PlayerAttribute) -> f32 {
        self.attributes
            .iter()
            .filter(|(a, _)| *a == attribute)
            .map(|(_, s)| s)
            .product()
    }
}
