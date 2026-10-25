#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PackKeyKind {
    Block,
    Item,
    Mob,
    Sound,
    Effect,
    Condition,
    Loot,
    Emitter,
    Model,
    Shape,
    Recipe,
    RecipeClass,
    UndergroundBiome,
    ShaderParam,
    Tag,
    MobTag,
    Data,
    Behavior,
    AiNode,
    GuiKind,
    GuiState,
    Widget(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackKey {
    pub kind: PackKeyKind,
    pub key: &'static str,
}

#[macro_export]
macro_rules! pack_keys {
    ($(
        $(#[$meta:meta])*
        $vis:vis $name:ident : $kind:ident $(($doc:expr))? = $key:literal;
    )*) => {
        $( $(#[$meta])* $vis const $name: &str = $key; )*

        #[cfg(test)]
        pub const PACK_KEYS: &[$crate::PackKey] = &[
            $( $crate::PackKey { kind: $crate::PackKeyKind::$kind $(($doc))?, key: $key }, )*
        ];
    };
}
