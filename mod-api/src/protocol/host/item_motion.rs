use crate::legality::prelude::*;

host_domain! {
    ItemMotionCall {
        /// Steers a flight through ordinary swept physics. Renew every tick; an expired
        /// controller releases the item. `None` immediately releases it as a loose drop.
        SteerItem {
            entity: u64,
            vel: Option<[f32; 3]>,
        } => legal(SERVER, Sim, Write),
        /// Replaces a live item entity's rules (what it was launched with); `None` removes them,
        /// leaving a plain stack.
        SetItemRules {
            entity: u64,
            rules: Option<crate::ItemEntityRules>,
        } => legal(SERVER, Sim, Write),
        /// Holds a steered flight's heading along `facing` instead of along its motion, while its
        /// controller keeps steering it; `None` turns it with its motion again.
        SetItemFacing {
            entity: u64,
            facing: Option<[f32; 3]>,
        } => legal(SERVER, Sim, Write),
    }
}
