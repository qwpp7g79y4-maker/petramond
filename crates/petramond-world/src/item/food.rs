#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FoodDef {
    pub eat_ticks: u32,
    pub effects: &'static [(crate::effect::Effect, u32)],
    /// Conditions eating it clears, like a fluid's `contact.clears`.
    pub clears: &'static [crate::condition::ConditionId],
}
