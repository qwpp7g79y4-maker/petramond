pub const PROJECTILE_DATA_KEY: &str = mod_api::PROJECTILE_KEY;

pub const DEFAULT_GRAVITY: f32 = mod_api::DEFAULT_ITEM_GRAVITY;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Projectile {
    pub gravity: f32,
    pub drag: f32,
    pub sticks: bool,
    pub sprite_spin: f32,
    pub sprite_tilt: f32,
}

impl Default for Projectile {
    fn default() -> Self {
        Projectile {
            gravity: DEFAULT_GRAVITY,
            drag: 0.0,
            sticks: false,
            sprite_spin: 0.0,
            sprite_tilt: 0.0,
        }
    }
}

impl Projectile {
    #[inline]
    pub fn flight(self) -> mod_api::ItemFlight {
        mod_api::ItemFlight {
            gravity: self.gravity,
            drag: self.drag,
        }
    }
}
