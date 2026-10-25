pub use glam::{IVec3, Mat4, Quat, Vec3, Vec4};

#[derive(Copy, Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tilt {
    pub pitch: f32,
    pub roll: f32,
}

impl Tilt {
    pub const LEVEL: Tilt = Tilt {
        pitch: 0.0,
        roll: 0.0,
    };

    pub const fn new(pitch: f32, roll: f32) -> Tilt {
        Tilt { pitch, roll }
    }

    pub fn is_level(self) -> bool {
        self == Tilt::LEVEL
    }

    pub fn is_finite(self) -> bool {
        self.pitch.is_finite() && self.roll.is_finite()
    }

    pub fn lerp(self, to: Tilt, t: f32) -> Tilt {
        Tilt {
            pitch: self.pitch + (to.pitch - self.pitch) * t,
            roll: self.roll + (to.roll - self.roll) * t,
        }
    }

    pub fn toward_level(self, max_step: f32) -> Tilt {
        Tilt {
            pitch: self.pitch - self.pitch.clamp(-max_step, max_step),
            roll: self.roll - self.roll.clamp(-max_step, max_step),
        }
    }

    pub fn rotation(self) -> Mat4 {
        crate::detmath::mat4_rotation_x(self.pitch) * crate::detmath::mat4_rotation_z(self.roll)
    }

    pub fn body_frame(self, yaw: f32) -> Mat4 {
        crate::detmath::mat4_rotation_y(yaw) * self.rotation()
    }
}

pub const FACE_NEIGHBORS: [IVec3; 6] = [
    IVec3::new(1, 0, 0),
    IVec3::new(-1, 0, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(0, 0, -1),
];

pub const MAX_SELECTION_BOXES: usize = 3;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SelectionBoxes {
    pub boxes: [(Vec3, Vec3); MAX_SELECTION_BOXES],
    pub len: u8,
}

impl SelectionBoxes {
    #[inline]
    pub fn iter(self) -> impl Iterator<Item = (Vec3, Vec3)> {
        self.boxes.into_iter().take(self.len as usize)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SelectionShape {
    Box {
        origin: IVec3,
        min: Vec3,
        max: Vec3,
    },
    Torch {
        origin: IVec3,
        transform: Mat4,
    },
    Boxes {
        origin: IVec3,
        boxes: SelectionBoxes,
    },
}

impl SelectionShape {
    pub fn full_block(block: IVec3) -> Self {
        Self::Box {
            origin: block,
            min: Vec3::ZERO,
            max: Vec3::ONE,
        }
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn voxel_at(pos: Vec3) -> IVec3 {
    IVec3::new(
        pos.x.floor() as i32,
        pos.y.floor() as i32,
        pos.z.floor() as i32,
    )
}

pub fn wrap_angle(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut d = a % TAU;
    if d > PI {
        d -= TAU;
    } else if d < -PI {
        d += TAU;
    }
    d
}

pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    a + wrap_angle(b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn wrap_angle_takes_the_shorter_way_round() {
        for (a, want) in [
            (0.0, 0.0),
            (FRAC_PI_2, FRAC_PI_2),
            (-FRAC_PI_2, -FRAC_PI_2),
            (PI + FRAC_PI_2, -FRAC_PI_2),
            (-PI - FRAC_PI_2, FRAC_PI_2),
            (TAU, 0.0),
            (5.0 * TAU + 0.25, 0.25),
            (-7.0 * TAU - 0.25, -0.25),
        ] {
            let got = wrap_angle(a);
            assert!(close(got, want), "wrap_angle({a}) = {got}, want {want}");
            assert!((-PI..=PI).contains(&got));
        }
    }

    #[test]
    fn voxel_at_floors_negative_coordinates() {
        assert_eq!(voxel_at(Vec3::new(0.5, 1.999, 2.0)), IVec3::new(0, 1, 2));
        assert_eq!(
            voxel_at(Vec3::new(-0.5, -1.0, -1.001)),
            IVec3::new(-1, -1, -2)
        );
        assert_eq!(voxel_at(Vec3::splat(-0.0)), IVec3::ZERO);
    }
}
