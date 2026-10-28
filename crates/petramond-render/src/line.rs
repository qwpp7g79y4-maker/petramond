//! A `DrawPrim::Line`: a hanging curve of thin cuboids on the block atlas, baked with the held
//! items every frame so an end on a player meets their posed held-item tip (or empty fist).

use glam::{Mat4, Vec3};
use petramond_math::face::Face;

use crate::item_cube::push_cell_local_face;
use crate::lighting::DynLight;

const SEGMENTS: usize = 20;

/// What a line wears: an atlas tile (once a block along it), a tint and a thickness in texels.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LineStyle {
    pub tile: petramond_world::tile::Tile,
    pub tint: [u8; 3],
    pub width: f32,
}
/// How far the middle hangs, per block of horizontal span, and at most.
const SAG_PER_BLOCK: f32 = 0.06;
const SAG_MAX: f32 = 1.2;

/// The point on the line `t` of the way from `from` to `to` (both render-relative): a parabola
/// hanging below the chord. It never dips below its lower end: the slack rests there (on the
/// water a bobber floats on) instead of hanging through it.
pub(crate) fn point(from: Vec3, to: Vec3, t: f32) -> Vec3 {
    let span = Vec3::new(to.x - from.x, 0.0, to.z - from.z).length();
    let sag = (span * SAG_PER_BLOCK)
        .min(SAG_MAX)
        .min((from.y - to.y).abs() * 0.25);
    from.lerp(to, t) - Vec3::Y * (sag * 4.0 * t * (1.0 - t))
}

/// A line leaving the first-person viewmodel: it starts nearer the eye than the item's tip
/// shows, so near the eye it thins to the width it would have `full_at` blocks out.
#[derive(Copy, Clone)]
pub(crate) struct NearEye {
    pub eye: Vec3,
    pub full_at: f32,
}

pub(crate) fn push(
    verts: &mut Vec<petramond_mesh::Vertex>,
    indices: &mut Vec<u32>,
    from: Vec3,
    to: Vec3,
    style: LineStyle,
    light: DynLight,
    near: Option<NearEye>,
) {
    let full = style.width / 16.0;
    let mut a = from;
    let mut laid = 0.0f32;
    for i in 1..=SEGMENTS {
        let t = i as f32 / SEGMENTS as f32;
        // Near the eye the width changes fast, so a thinned line spends its segments there.
        let b = point(from, to, if near.is_some() { t * t } else { t });
        let width = near.map_or(full, |n| {
            (full * ((a + b) * 0.5 - n.eye).length() / n.full_at.max(1e-3)).min(full)
        });
        let length = (b - a).length();
        // The tile repeats once a block, texel true. Where the line thins near the eye its texels
        // shrink with it, so it keeps the look it has at its full width.
        let across = width / full;
        let laying = length / across.max(1e-3);
        if length > 1e-4 {
            let bar = Bar::along((b - a) / length, across);
            for (done, step, u) in repeats(laid, laying) {
                let caps = [
                    i == 1 && done == 0.0,
                    i == SEGMENTS && laying - done - step <= 1e-4,
                ];
                let start = verts.len();
                bar.push_piece(verts, indices, style, light, u, caps);
                let m = bar.frame(a + bar.dir * (done * across), step * across, u);
                for v in &mut verts[start..] {
                    v.pos = m.transform_point3(Vec3::from(v.pos)).to_array();
                }
                crate::block_draw::multiply_tint(&mut verts[start..], style.tint);
            }
        }
        laid += laying;
        a = b;
    }
}

/// The pieces a run `length` tile repeats long splits into, `laid` repeats along the line: each keeps
/// within one repeat, as (repeats into the run, repeats long, the tile's `u` range it covers).
fn repeats(laid: f32, length: f32) -> impl Iterator<Item = (f32, f32, [f32; 2])> {
    let mut done = 0.0f32;
    std::iter::from_fn(move || {
        if length - done <= 1e-4 {
            return None;
        }
        let at = laid + done;
        let repeat = (at + 1e-4).floor();
        let step = (repeat + 1.0 - at).min(length - done);
        let piece = (
            done,
            step,
            [(at - repeat).max(0.0), (at - repeat + step).min(1.0)],
        );
        done += step;
        Some(piece)
    })
}

/// One straight run of the line. Its pieces are boxes in tile space (`u` along the line, the
/// tile's middle `width` texels across) carried onto the run, so every face samples the strip the
/// line's own length and thickness cover.
struct Bar {
    dir: Vec3,
    side: Vec3,
    up: Vec3,
    /// World thickness per tile unit across: 1 at full width, less where thinned near the eye.
    across: f32,
}

impl Bar {
    fn along(dir: Vec3, across: f32) -> Self {
        // Squared to the world's up, so neighbouring runs show their texels the same way round.
        let side = Vec3::Y.cross(dir).try_normalize().unwrap_or(Vec3::X);
        Self {
            dir,
            side,
            up: dir.cross(side),
            across,
        }
    }

    fn push_piece(
        &self,
        verts: &mut Vec<petramond_mesh::Vertex>,
        indices: &mut Vec<u32>,
        style: LineStyle,
        light: DynLight,
        [u0, u1]: [f32; 2],
        [first, last]: [bool; 2],
    ) {
        let band = (style.width / 16.0).min(1.0);
        let (lo, hi) = (0.5 - band * 0.5, 0.5 + band * 0.5);
        for face in Face::ALL {
            let cap = match face {
                Face::NegX => Some(first),
                Face::PosX => Some(last),
                _ => None,
            };
            if cap == Some(false) {
                continue;
            }
            push_cell_local_face(
                verts,
                indices,
                style.tile,
                Vec3::ZERO,
                1.0,
                [u0, lo, lo],
                [u1, hi, hi],
                face,
                light,
            );
        }
    }

    /// Carries the tile-space piece `u0..u1` onto `length` blocks of this run from `start`.
    fn frame(&self, start: Vec3, length: f32, [u0, u1]: [f32; 2]) -> Mat4 {
        let x = self.dir * (length / (u1 - u0).max(1e-4));
        let (y, z) = (self.side * self.across, self.up * self.across);
        Mat4::from_cols(
            x.extend(0.0),
            y.extend(0.0),
            z.extend(0.0),
            (start - x * u0 - (y + z) * 0.5).extend(1.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_runs_end_to_end_and_hangs_below_its_chord_but_never_its_lower_end() {
        let (from, to) = (Vec3::new(0.0, 2.0, 0.0), Vec3::new(10.0, 0.0, 0.0));
        assert!(point(from, to, 0.0).distance(from) < 1e-5);
        assert!(point(from, to, 1.0).distance(to) < 1e-5);
        let mid = point(from, to, 0.5);
        assert!(mid.y < 1.0, "the middle sags below the chord: {mid:?}");
        for (from, to) in [(from, to), (to, from), (from, Vec3::new(20.0, 1.5, 0.0))] {
            for i in 0..=100 {
                let p = point(from, to, i as f32 / 100.0);
                assert!(
                    p.y >= from.y.min(to.y) - 1e-5,
                    "the line hangs below its lower end: {p:?}"
                );
            }
        }
        let short = point(from, Vec3::new(0.0, -3.0, 0.0), 0.5);
        assert!(
            short.distance(Vec3::new(0.0, -0.5, 0.0)) < 1e-5,
            "a plumb line hangs straight"
        );
    }

    #[test]
    fn the_tile_runs_on_along_the_line_one_repeat_per_piece() {
        let (mut laid, mut last) = (0.0f32, 0.0f32);
        for length in [0.3, 2.45, 0.7, 1.0, 0.05, 3.2] {
            let mut covered = 0.0f32;
            for (done, step, [u0, u1]) in repeats(laid, length) {
                assert!(
                    (done - covered).abs() < 1e-4,
                    "pieces follow on: {done} {covered}"
                );
                assert!(
                    0.0 <= u0 && u0 < u1 && u1 <= 1.0,
                    "inside one repeat: {u0}..{u1}"
                );
                let carried = if last >= 1.0 - 1e-4 { 0.0 } else { last };
                assert!(
                    (u0 - carried).abs() < 1e-4,
                    "the tile continues: {u0} after {last}"
                );
                assert!((u1 - u0 - step).abs() < 1e-4, "texel true along the run");
                (covered, last) = (covered + step, u1);
            }
            assert!(
                (covered - length).abs() < 1e-4,
                "the run is covered: {covered} of {length}"
            );
            laid += length;
        }
    }
}
