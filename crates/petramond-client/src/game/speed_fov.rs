const GAIN: f32 = 0.15;
const MIN_MULT: f32 = 0.92;
const MAX_MULT: f32 = 1.30;
const SETTLE_SPEED: f32 = 8.0;

pub struct SpeedFov {
    base_fov_y: f32,
    mult: f32,
}

impl SpeedFov {
    pub fn new(base_fov_y: f32) -> Self {
        Self {
            base_fov_y,
            mult: 1.0,
        }
    }

    pub fn advance(&mut self, dt: f32, speed_ratio: f32) {
        let target = (1.0 + GAIN * (speed_ratio - 1.0)).clamp(MIN_MULT, MAX_MULT);
        let settle = 1.0 - (-SETTLE_SPEED * dt.max(0.0)).exp();
        self.mult += (target - self.mult) * settle;
        if (self.mult - target).abs() < 1e-4 {
            self.mult = target;
        }
    }

    pub fn fov_y(&self) -> f32 {
        self.base_fov_y * self.mult
    }
}
