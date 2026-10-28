use super::ItemStack;

pub const TOOL_DATA_KEY: &str = "petramond:tool";

pub use crate::material::ToolKind;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Tool {
    pub kind: ToolKind,
    pub tier: u8,
    pub speed: f32,
    pub damage: (f32, f32),
    pub knockback: f32,
}

pub const DEFAULT_KNOCKBACK: f32 = 1.0;

pub fn default_speed(tier: u8) -> f32 {
    match tier {
        0 => 1.0,
        1 => 2.0,
        2 => 4.0,
        3 => 6.0,
        _ => 8.0,
    }
}

pub fn default_damage(kind: ToolKind, tier: u8) -> (f32, f32) {
    match tier {
        0 => FIST_DAMAGE,
        t => kind.def().damage[usize::from(t.min(4)) - 1],
    }
}

#[derive(serde::Deserialize)]
struct RawToolOverride {
    #[serde(default)]
    tier: Option<u8>,
    #[serde(default)]
    speed: Option<f32>,
    #[serde(default)]
    damage: Option<[f32; 2]>,
    #[serde(default)]
    knockback: Option<f32>,
}

impl Tool {
    pub fn with_override(self, bytes: &[u8]) -> Tool {
        let Ok(raw) = serde_json::from_slice::<RawToolOverride>(bytes) else {
            return self;
        };
        let mut t = self;
        if let Some(tier) = raw.tier {
            t.tier = tier;
        }
        if let Some(speed) = raw.speed {
            if speed.is_finite() && speed > 0.0 {
                t.speed = speed;
            }
        }
        if let Some([lo, hi]) = raw.damage {
            if lo.is_finite() && hi.is_finite() && 0.0 <= lo && lo <= hi {
                t.damage = (lo, hi);
            }
        }
        if let Some(k) = raw.knockback {
            if k.is_finite() && k >= 0.0 {
                t.knockback = k;
            }
        }
        t
    }

    pub fn new(kind: ToolKind, tier: u8) -> Tool {
        Tool {
            kind,
            tier,
            speed: default_speed(tier),
            damage: default_damage(kind, tier),
            knockback: DEFAULT_KNOCKBACK,
        }
    }

    pub fn attack_damage(self) -> (f32, f32) {
        self.damage
    }
}

pub const FIST_DAMAGE: (f32, f32) = (1.0, 1.0);

pub fn attack_damage(stack: Option<&ItemStack>) -> (f32, f32) {
    stack
        .and_then(ItemStack::tool)
        .map(Tool::attack_damage)
        .unwrap_or(FIST_DAMAGE)
}
