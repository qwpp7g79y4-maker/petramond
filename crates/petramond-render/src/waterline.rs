//! Which view rays set out under the water. A ray starts where it meets the near plane, so with
//! the eye at the waterline the near plane straddles the surface: rays crossing it below the
//! surface see the water's look, the rest the air's. The frame lane `waterline` carries this to
//! every world shader (`underwater_ray` in petramond::underwater mirrors `under`).

use glam::{Mat3, Mat4, Vec3};

/// A surface height past every near-plane point: all rays on its side.
const PAST_THE_NEAR_PLANE: f32 = 1.0e20;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Waterline {
    /// The view's depth gradient over the near distance: a view vector's depth in near distances.
    depth_per_near: Vec3,
    /// The water's surface above the eye.
    level: f32,
}

impl Waterline {
    /// No ray under any water.
    pub(crate) const NONE: Self = Self {
        depth_per_near: Vec3::ZERO,
        level: 0.0,
    };

    /// The surface `level` above the eye (below it when negative) seen through `view_proj`, whose
    /// near plane lies `near` ahead; `None` = every ray under the water. A surface clear of the
    /// whole near plane puts every ray on one side, points behind the eye included.
    pub(crate) fn new(view_proj: Mat4, near: f32, level: Option<f32>) -> Self {
        let depth_per_near = view_proj.row(3).truncate() / near;
        let all_under = Self {
            depth_per_near,
            level: PAST_THE_NEAR_PLANE,
        };
        let Some(level) = level else {
            return all_under;
        };
        let (low, high) = near_plane_heights(view_proj, near);
        if level > high {
            all_under
        } else if level <= low {
            Self::NONE
        } else {
            Self {
                depth_per_near,
                level,
            }
        }
    }

    pub(crate) fn lane(self) -> [f32; 4] {
        self.depth_per_near.extend(self.level).to_array()
    }

    /// Whether the ray toward `view` (a point less the eye) meets the near plane under the surface.
    /// A point short of the near plane, or behind the eye, stands at its own height.
    pub(crate) fn under(self, view: Vec3) -> bool {
        self.depth_per_near != Vec3::ZERO
            && view.y < self.level * self.depth_per_near.dot(view).max(1.0)
    }
}

/// The lowest and highest points of the near plane, above the eye.
fn near_plane_heights(view_proj: Mat4, near: f32) -> (f32, f32) {
    // A view vector's clip x, y and w are linear in it, and the near plane's corners are where
    // x = ±w and y = ±w at w = near.
    let clip = Mat3::from_cols(
        view_proj.row(0).truncate(),
        view_proj.row(1).truncate(),
        view_proj.row(3).truncate(),
    )
    .transpose();
    let height = clip.inverse().row(1) * near;
    let reach = height.x.abs() + height.y.abs();
    (height.z - reach, height.z + reach)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use petramond_math::world_pos::WorldPos;

    fn camera(pitch: f32) -> Camera {
        let mut cam = Camera::new(WorldPos::new(0.5, 64.0, 0.5), 16.0 / 9.0);
        cam.pitch = pitch;
        cam
    }

    /// An eye a hair over the surface, looking level: the near plane dips under it, so the rays
    /// through the bottom of the screen start in the water and the rest in the air.
    #[test]
    fn rays_through_the_near_plane_under_the_surface_are_under_the_water() {
        let cam = camera(0.0);
        let vp = cam.view_proj();
        let line = Waterline::new(vp, cam.near, Some(-0.02));
        let fwd = cam.forward();
        let down = |angle: f32| fwd * angle.cos() - Vec3::Y * angle.sin();
        assert!(!line.under(fwd * 30.0), "straight ahead clears the surface");
        assert!(
            !line.under(down(0.1) * 30.0),
            "it meets the surface past the near plane"
        );
        assert!(
            line.under(down(0.5) * 30.0),
            "it is under before the near plane"
        );
        assert!(Waterline::new(vp, cam.near, None).under(fwd + Vec3::Y));
        assert!(!Waterline::NONE.under(down(0.5)));
    }

    /// A thing lit at a point behind the eye (a chest underfoot, a flag passed) still shows where
    /// the near plane cuts it, so the point must take that medium, never fall back to air.
    #[test]
    fn points_behind_the_eye_take_the_medium_the_near_plane_gives_them() {
        let cam = camera(-0.6);
        let vp = cam.view_proj();
        let back = -cam.forward().with_y(0.0).normalize();
        let behind_below = back * 6.0 - Vec3::Y * 3.0;
        for level in [None, Some(0.5)] {
            let line = Waterline::new(vp, cam.near, level);
            assert!(line.under(behind_below), "{level:?}: the eye is under");
            assert!(
                line.under(cam.forward() * 0.01),
                "{level:?}: short of the near plane"
            );
        }
        assert!(
            !Waterline::new(vp, cam.near, Some(-0.5)).under(behind_below),
            "the near plane is wholly over the surface"
        );
        let straddling = Waterline::new(vp, cam.near, Some(0.0));
        assert!(straddling.under(behind_below));
        assert!(!straddling.under(back * 6.0 + Vec3::Y * 0.5));
    }
}
