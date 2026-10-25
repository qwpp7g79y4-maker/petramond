use std::f32::consts::TAU;

const CYCLES_PER_BLOCK: f32 = 0.35;

const ENVELOPE_RATE: f32 = 9.0;

const FULL_ENVELOPE_SPEED: f32 = 4.3;

const MAX_ENVELOPE: f32 = 1.3;

const MIN_SPEED: f32 = 0.35;

#[derive(Default)]
pub struct ViewBob {
    phase: f32,
    weight: f32,
}

impl ViewBob {
    pub fn advance(&mut self, dt: f32, hspeed: f32, striding: bool) {
        let dt = dt.max(0.0);
        let target = if striding && hspeed >= MIN_SPEED {
            (hspeed / FULL_ENVELOPE_SPEED).min(MAX_ENVELOPE)
        } else {
            0.0
        };
        let settle = 1.0 - (-ENVELOPE_RATE * dt).exp();
        self.weight += (target - self.weight) * settle;
        if self.weight < 1e-4 && target == 0.0 {
            self.weight = 0.0;
            self.phase = 0.0;
            return;
        }
        self.phase = (self.phase + dt * hspeed * CYCLES_PER_BLOCK).fract();
    }

    pub fn offset(&self) -> [f32; 2] {
        if self.weight == 0.0 {
            return [0.0, 0.0];
        }
        let theta = self.phase * TAU;
        [theta.sin() * self.weight, (2.0 * theta).cos() * self.weight]
    }

    pub fn stride(&self) -> (f32, f32) {
        (self.phase, self.weight)
    }
}
