//! Body attribute scales a row applies while it holds (a worn item, an active effect): a
//! `PlayerAttribute` key mapped to the factor it multiplies in.

use std::collections::BTreeMap;

use mod_api::PlayerAttribute;

pub type AttributeScales = &'static [(PlayerAttribute, f32)];

pub(crate) fn resolve(map: &BTreeMap<String, f32>, what: &str) -> Result<AttributeScales, String> {
    let scales = map
        .iter()
        .map(|(key, &scale)| {
            let attribute = PlayerAttribute::from_key(key)
                .ok_or_else(|| format!("{what} attribute '{key}' is not a body attribute"))?;
            if !scale.is_finite() || scale < 0.0 {
                return Err(format!(
                    "{what} attribute '{key}' scale {scale} must be finite and non-negative"
                ));
            }
            Ok((attribute, scale))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Box::leak(scales.into_boxed_slice()))
}

/// The factor `scales` applies to `attribute` (1 when it does not touch it).
pub fn scale(scales: AttributeScales, attribute: PlayerAttribute) -> f32 {
    scales
        .iter()
        .filter(|(a, _)| *a == attribute)
        .map(|(_, s)| s)
        .product()
}
