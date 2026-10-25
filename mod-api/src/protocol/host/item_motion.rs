use crate::legality::prelude::*;

host_domain! {
    ItemMotionCall {
        ItemEntitiesInRadius {
            pos: [f64; 3],
            radius: f32,
            limit: u32,
        } => legal(SERVER, Sim, Read),
        ItemImpulses {
            impulses: Vec<(u64, [f32; 3])>,
        } => legal(SERVER, Sim, Write),
        /// Steers a flight through ordinary swept physics. Renew every tick; an expired
        /// controller releases the item. `None` immediately releases it as a loose drop.
        SteerItem {
            entity: u64,
            vel: Option<[f32; 3]>,
        } => legal(SERVER, Sim, Write),
        /// Removes one live item entity, returning its complete stack exactly once.
        TakeItemEntity {
            entity: u64,
        } => legal(SERVER, Sim, Write),
        /// Draws a live item entity as `prims` in place of its own art, about its position
        /// (`World` stays upright; `Body` turns with its heading). An empty list restores the art.
        SetItemDraw {
            entity: u64,
            frame: crate::DrawFrame,
            prims: Vec<crate::DrawPrim>,
        } => legal(SERVER, Sim, Write),
        /// Replaces a live item entity's `petramond:item_entity` rules (the JSON it was launched
        /// with); `None` removes them, leaving a plain stack. Dropping `tether` cuts its line.
        SetItemRules {
            entity: u64,
            rules: Option<Vec<u8>>,
        } => legal(SERVER, Sim, Write),
        /// Holds a steered flight's heading along `facing` instead of along its motion, while its
        /// controller keeps steering it; `None` turns it with its motion again.
        SetItemFacing {
            entity: u64,
            facing: Option<[f32; 3]>,
        } => legal(SERVER, Sim, Write),
    }
}
