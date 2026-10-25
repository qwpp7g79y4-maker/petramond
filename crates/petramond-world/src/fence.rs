use crate::block::Aabb;

pub const RAIL_INSET: f32 = 0.5 / 16.0;

#[inline]
pub const fn rail_cross(post_lo: f32, post_hi: f32) -> (f32, f32) {
    (post_lo + RAIL_INSET, post_hi - RAIL_INSET)
}

pub const RAIL_TOP_LO: f32 = 11.0 / 16.0;
pub const RAIL_TOP_HI: f32 = 14.0 / 16.0;
pub const RAIL_BOT_LO: f32 = 2.0 / 16.0;
pub const RAIL_BOT_HI: f32 = 5.0 / 16.0;

pub const fn item_posts(post_lo: f32, post_hi: f32) -> [Aabb; 2] {
    let thickness = post_hi - post_lo;
    [
        Aabb {
            min: [0.0, 0.0, post_lo],
            max: [thickness, 1.0, post_hi],
        },
        Aabb {
            min: [1.0 - thickness, 0.0, post_lo],
            max: [1.0, 1.0, post_hi],
        },
    ]
}

pub const fn item_rails(post_lo: f32, post_hi: f32) -> [Aabb; 2] {
    let thickness = post_hi - post_lo;
    let rail_lo = post_lo + RAIL_INSET;
    let rail_hi = post_hi - RAIL_INSET;
    [
        Aabb {
            min: [thickness, RAIL_TOP_LO, rail_lo],
            max: [1.0 - thickness, RAIL_TOP_HI, rail_hi],
        },
        Aabb {
            min: [thickness, RAIL_BOT_LO, rail_lo],
            max: [1.0 - thickness, RAIL_BOT_HI, rail_hi],
        },
    ]
}
