use super::views::EntityShadow;

pub const VERTS_PER_SHADOW: u32 = 4;

pub const QUAD_INDEX_PATTERN: [u32; 6] = [0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowVertex {
    pub pos: [f32; 3],
    pub corner: [f32; 2],
    pub strength: f32,
}

pub fn build_entity_shadows(
    shadows: &[EntityShadow],
    render_origin: glam::IVec3,
    verts: &mut Vec<ShadowVertex>,
) -> u32 {
    verts.clear();
    for shadow in shadows {
        let c = shadow.center.relative_to(render_origin);
        let r = shadow.radius;
        let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        for &(cx, cz) in &corners {
            verts.push(ShadowVertex {
                pos: [c.x + cx * r, c.y, c.z + cz * r],
                corner: [cx, cz],
                strength: shadow.strength,
            });
        }
    }
    verts.len() as u32
}

pub fn quad_index_count(quad_count: usize) -> usize {
    quad_count * 6
}
