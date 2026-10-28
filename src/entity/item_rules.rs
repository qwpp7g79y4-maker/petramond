use petramond_world::item::{variant, ItemStack};

pub(super) use mod_api::ItemEntityRules as ItemRules;

/// The rules `stack` carries, or a plain stack's.
pub(super) fn of(stack: ItemStack) -> ItemRules {
    variant::get(stack.variant)
        .and_then(|data| {
            data.get(petramond_world::item::ITEM_ENTITY_DATA_KEY)
                .and_then(|raw| serde_json::from_slice(raw).ok())
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::ItemRules;

    #[test]
    fn rules_a_mod_writes_read_back_the_same() {
        for rules in [
            ItemRules::default(),
            ItemRules {
                pickup: false,
                lifetime_ticks: Some(2400),
            },
        ] {
            let (_, json) = rules.data();
            assert_eq!(serde_json::from_slice::<ItemRules>(&json).unwrap(), rules);
        }
    }
}
