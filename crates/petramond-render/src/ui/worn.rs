//! What a worn item lays over the wearer's own first-person view.

use petramond::gui::UiSnapshot;

use super::{push_quad_uv, UiBuild, WHITE};
use crate::worn_overlays::{row_of, SPAN};

pub(super) fn push(ui: &UiSnapshot, build: &mut UiBuild, screen: (u32, u32)) {
    if !ui.first_person {
        return;
    }
    let (w, h) = (screen.0 as f32, screen.1 as f32);
    let half_u = (w / h / SPAN as f32 * 0.5).min(0.5);
    for stack in ui.worn.iter().flatten() {
        let Some((row, rows)) = row_of(stack.item) else {
            continue;
        };
        // Half a texel in from the row's edges so no neighbouring row bleeds in.
        let inset = 0.5 / (rows as f32 * crate::worn_overlays::CELL_PX as f32);
        let v0 = row as f32 / rows as f32 + inset;
        let v1 = (row + 1) as f32 / rows as f32 - inset;
        push_quad_uv(
            &mut build.worn_overlay,
            screen,
            0.0,
            0.0,
            w,
            h,
            [0.5 - half_u, v0],
            [0.5 + half_u, v1],
            WHITE,
        );
    }
}
