/// Whether a use-click on `block` is the block's own to answer: it opens a GUI, or one of its
/// behaviors answers use (a door swings, a bed sleeps).
pub fn builtin_claims_click(block: crate::block::Block, sneaking: bool) -> bool {
    !sneaking && (block.open_gui().is_some() || block.behaviors().iter().any(|b| b.answers_use()))
}
