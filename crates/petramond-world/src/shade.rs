use crate::face::Face;

pub const SHADES: [f32; 4] = [1.00, 0.85, 0.75, 0.55];

pub trait FaceShading: Sized {
    fn shade_idx(self) -> u32;

    fn normal_code(self) -> u32;

    fn shade(self) -> f32 {
        SHADES[self.shade_idx() as usize]
    }
}

impl FaceShading for Face {
    #[inline]
    fn shade_idx(self) -> u32 {
        match self {
            Face::PosY => 0,
            Face::PosZ | Face::NegZ => 1,
            Face::PosX | Face::NegX => 2,
            Face::NegY => 3,
        }
    }

    #[inline]
    fn normal_code(self) -> u32 {
        self as u32 + 1
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ContactShadowVertex {
    pub pos: [f32; 3],
    pub darken: f32,
}
