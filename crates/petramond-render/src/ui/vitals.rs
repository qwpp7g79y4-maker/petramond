//! The survival vitals over the hotbar: hearts along the bottom-left, breath
//! bubbles above them while a dive is under way, active effects above both.

use petramond::gui::UiSnapshot;
use petramond_world::breath::BreathView;

use super::{push_quad_uv, UiBuild, UiVertex, WHITE};

const HEART_PX: f32 = 9.0;
const HEART_STEP: f32 = 8.0;
const HEART_MARGIN: f32 = 8.0;
const HEART_CELL_U: f32 = 1.0 / 3.0;
const ROW_GAP: f32 = 2.0;

const BUBBLES: u32 = BreathView::BUBBLES;
const BUBBLE_CELL_U: f32 = 0.5;
/// A bubble wears its bursting cell over the last stretch of its share of the breath.
const BURST_BELOW: f32 = 0.35;

pub(super) fn push(ui: &UiSnapshot, build: &mut UiBuild, screen: (u32, u32), scale: f32) {
    if let Some(health) = ui.health {
        push_hearts(&mut build.hearts, screen, health, ui.heart_wiggle, scale);
    }
    let mut top = screen.1 as f32 - (HEART_MARGIN + HEART_PX) * scale;
    if let Some(breath) = ui.breath {
        top -= (ROW_GAP + HEART_PX) * scale;
        push_bubbles(&mut build.bubbles, screen, breath, top, scale);
    }
    push_effects(&mut build.effects, screen, &ui.effects, top, scale);
}

fn push_hearts(
    out: &mut Vec<UiVertex>,
    screen: (u32, u32),
    health: petramond_world::gui_state::HealthView,
    wiggle: Option<(i32, i32, f32)>,
    scale: f32,
) {
    let hearts = (health.max / 2).max(0);
    if hearts == 0 {
        return;
    }
    let size = HEART_PX * scale;
    let step = HEART_STEP * scale;
    let margin = HEART_MARGIN * scale;
    let y = screen.1 as f32 - margin - size;
    let wiggle_offset = |i: i32| -> (f32, f32) {
        let Some((lo, hi, t)) = wiggle else {
            return (0.0, 0.0);
        };
        if 2 * i >= hi || 2 * i + 2 <= lo {
            return (0.0, 0.0);
        }
        ((t * 183.0).sin() * 0.7 * scale, (t * 149.0).cos() * scale)
    };
    let cell_uv = |c: i32| -> ([f32; 2], [f32; 2]) {
        let u0 = c as f32 * HEART_CELL_U;
        ([u0, 0.0], [u0 + HEART_CELL_U, 1.0])
    };
    let current = health.current.clamp(0, health.max);
    for i in 0..hearts {
        let (dx, dy) = wiggle_offset(i);
        let x = margin + i as f32 * step + dx;
        let y = y + dy;
        let (tl, br) = cell_uv(0);
        push_quad_uv(out, screen, x, y, size, size, tl, br, WHITE);
        let cell = match (current - i * 2).clamp(0, 2) {
            2 => 2,
            1 => 1,
            _ => continue,
        };
        let (tl, br) = cell_uv(cell);
        push_quad_uv(out, screen, x, y, size, size, tl, br, WHITE);
    }
}

fn push_effects(
    out: &mut Vec<UiVertex>,
    screen: (u32, u32),
    effects: &[petramond_world::effect::Effect],
    above: f32,
    scale: f32,
) {
    if effects.is_empty() {
        return;
    }
    let cell = crate::effect_icons::CELL_PX as f32 * scale;
    let gap = ROW_GAP * scale;
    let margin = HEART_MARGIN * scale;
    let y = above - gap - cell;
    let strip_cells = petramond_world::effect::defs().len() as f32;
    for (i, effect) in effects.iter().enumerate() {
        let x = margin + i as f32 * (cell + gap);
        let u0 = effect.0 as f32 / strip_cells;
        let u1 = (effect.0 as f32 + 1.0) / strip_cells;
        push_quad_uv(out, screen, x, y, cell, cell, [u0, 0.0], [u1, 1.0], WHITE);
    }
}

fn push_bubbles(
    out: &mut Vec<UiVertex>,
    screen: (u32, u32),
    breath: BreathView,
    y: f32,
    scale: f32,
) {
    if breath.max == 0 {
        return;
    }
    let size = HEART_PX * scale;
    let step = HEART_STEP * scale;
    let margin = HEART_MARGIN * scale;
    let share = breath.max as f32 / BUBBLES as f32;
    let current = breath.current.min(breath.max) as f32;
    for i in 0..BUBBLES {
        let left = ((current - i as f32 * share) / share).clamp(0.0, 1.0);
        let cell = match left {
            l if l <= 0.0 => continue,
            l if l < BURST_BELOW => 1,
            _ => 0,
        };
        let u0 = cell as f32 * BUBBLE_CELL_U;
        let x = margin + i as f32 * step;
        push_quad_uv(
            out,
            screen,
            x,
            y,
            size,
            size,
            [u0, 0.0],
            [u0 + BUBBLE_CELL_U, 1.0],
            WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: (u32, u32) = (1280, 720);
    const QUAD: usize = 6;

    fn hud(breath: Option<BreathView>) -> UiBuild {
        let ui = UiSnapshot {
            breath,
            effects: vec![petramond_world::effect::Effect(0)],
            ..Default::default()
        };
        let mut build = UiBuild::default();
        push(&ui, &mut build, SCREEN, 1.0);
        build
    }

    fn quad_xs(verts: &[UiVertex]) -> Vec<f32> {
        verts.chunks(QUAD).map(|q| q[0].pos[0]).collect()
    }

    #[test]
    fn bubbles_drain_from_the_right_and_burst_before_they_vanish() {
        let view = |current| Some(BreathView { current, max: 300 });
        assert_eq!(hud(view(300)).bubbles.len(), 10 * QUAD);
        let half = hud(view(150));
        assert_eq!(half.bubbles.len(), 5 * QUAD);
        let full_xs = quad_xs(&hud(view(300)).bubbles);
        assert_eq!(
            quad_xs(&half.bubbles),
            full_xs[..5],
            "the left bubbles stay"
        );
        let bursting = hud(view(130));
        assert_eq!(bursting.bubbles.len(), 5 * QUAD);
        let uv = |verts: &[UiVertex]| verts[verts.len() - QUAD].uv[0];
        assert!(
            uv(&bursting.bubbles) > uv(&half.bubbles),
            "the last bubble bursts"
        );
        assert!(hud(view(0)).bubbles.is_empty());
    }
}
