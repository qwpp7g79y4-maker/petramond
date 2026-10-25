pub const USE_REPEAT_TICKS: u32 = 5;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConsumerKind {
    Registered,
    Shear,
    BuiltinBlock,
    ContextualPlace,
    Eat,
    ItemUse,
    Place,
}

impl ConsumerKind {
    pub const CLAIM_ORDER: [ConsumerKind; 7] = [
        ConsumerKind::Registered,
        ConsumerKind::Shear,
        ConsumerKind::BuiltinBlock,
        ConsumerKind::ContextualPlace,
        ConsumerKind::Eat,
        ConsumerKind::ItemUse,
        ConsumerKind::Place,
    ];

    pub const fn presents_itself(self) -> bool {
        matches!(self, ConsumerKind::Eat)
    }
}
