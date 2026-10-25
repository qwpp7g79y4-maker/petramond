//! Whether a point sits inside a fluid the way an eye experiences it. The
//! camera murk and a body's breath share this one rule, so the screen never
//! shows clear air while the bubbles drain, or the reverse.

use crate::block::Block;
use crate::fluid_math::{fills_cell, fluid_height};
use crate::mathh::{lerp, IVec3, Vec3};
use crate::world::{SectionCursor, WorldData};
use petramond_math::world_pos::WorldPos;

impl WorldData {
    /// The fluid enclosing `eye`, or `None` in air. A medium with an
    /// `eye_margin` counts the eye only once it sits that far below the
    /// rendered surface (the same corner-averaged heights the mesher draws),
    /// so bobbing at the waterline reads as above water.
    pub fn eye_fluid(&self, eye: WorldPos) -> Option<Block> {
        let cur = self.cursor();
        let cell = eye.block();
        let fluid = cur.cell_fluid(cell)?;
        let margin = fluid.fluid_def()?.medium.eye_margin;
        eye_inside(&cur, eye, fluid, margin).then_some(fluid)
    }

    /// The open surface of the fluid the eye is in or just over, at the eye's column, as the
    /// mesher draws it: the fluid and the surface's absolute height. `None` with no fluid there,
    /// with more than a cell of it over the eye's own, or under a solid lid.
    pub fn eye_surface(&self, eye: WorldPos) -> Option<(Block, f32)> {
        let cur = self.cursor();
        let cell = eye.block();
        let (top, fluid) = match cur.cell_fluid(cell) {
            Some(fluid) if cur.cell_fluid(cell + IVec3::Y) == Some(fluid) => {
                (cell + IVec3::Y, fluid)
            }
            Some(fluid) => (cell, fluid),
            None => (cell - IVec3::Y, cur.cell_fluid(cell - IVec3::Y)?),
        };
        let above = top + IVec3::Y;
        if cur.cell_fluid(above) == Some(fluid)
            || cur.block_if_loaded(above).is_none_or(Block::is_solid)
        {
            return None;
        }
        Some((fluid, surface_y_at(&cur, top, eye.relative_to(top), fluid)))
    }
}

impl WorldData {
    /// The height a body rising from `point` leaves the fluid it starts in, as the mesher draws
    /// that surface. `point.y` outside fluid; infinity when no surface lies within `reach`.
    pub fn fluid_ceiling(&self, point: WorldPos, reach: f32) -> f32 {
        let cur = self.cursor();
        let mut top = point.block();
        let Some(fluid) = cur.cell_fluid(top) else {
            return point.y as f32;
        };
        let last = top.y + reach.max(0.0).ceil() as i32;
        while cur.cell_fluid(top + IVec3::Y) == Some(fluid) {
            if top.y >= last {
                return f32::INFINITY;
            }
            top += IVec3::Y;
        }
        surface_y_at(&cur, top, point.relative_to(top), fluid)
    }
}

fn eye_inside(cur: &SectionCursor<'_>, eye: WorldPos, fluid: Block, margin: Option<f32>) -> bool {
    let Some(margin) = margin else {
        return true;
    };
    let cell = eye.block();
    if cur.cell_fluid(cell + IVec3::Y) == Some(fluid) {
        return true;
    }
    eye.y < f64::from(surface_y_at(cur, cell, eye.relative_to(cell), fluid) - margin)
}

fn surface_y_at(cur: &SectionCursor<'_>, cell: IVec3, eye_in_cell: Vec3, fluid: Block) -> f32 {
    if fills_cell_at(cur, cell, fluid) {
        return cell.y as f32 + 1.0;
    }
    let mut h = [[1.0f32; 2]; 2];
    for cx in 0..2i32 {
        for cz in 0..2i32 {
            let mut sum = 0.0;
            let mut cnt = 0;
            for ox in (cx - 1)..=cx {
                for oz in (cz - 1)..=cz {
                    if let Some(height) = height_at(cur, cell + IVec3::new(ox, 0, oz), fluid) {
                        sum += height;
                        cnt += 1;
                    }
                }
            }
            h[cx as usize][cz as usize] = if cnt == 0 { 1.0 } else { sum / cnt as f32 };
        }
    }
    let fx = eye_in_cell.x.clamp(0.0, 1.0);
    let fz = eye_in_cell.z.clamp(0.0, 1.0);
    let z0 = lerp(h[0][0], h[1][0], fx);
    let z1 = lerp(h[0][1], h[1][1], fx);
    cell.y as f32 + lerp(z0, z1, fz)
}

fn height_at(cur: &SectionCursor<'_>, c: IVec3, fluid: Block) -> Option<f32> {
    if cur.cell_fluid(c) != Some(fluid) {
        return None;
    }
    Some(fluid_height(
        cur.fluid_meta(c),
        cur.cell_fluid(c + IVec3::Y),
        fluid,
    ))
}

fn fills_cell_at(cur: &SectionCursor<'_>, c: IVec3, fluid: Block) -> bool {
    cur.cell_fluid(c) == Some(fluid)
        && fills_cell(cur.fluid_meta(c), cur.cell_fluid(c + IVec3::Y), fluid)
}

#[cfg(test)]
mod tests;
