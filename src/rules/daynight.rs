pub const DEFAULT_CYCLE_TICKS: u64 =
    cycle_ticks_for_day_minutes(crate::save::settings::DEFAULT_DAY_MINUTES);

pub const MOON_PHASES: u64 = 8;

pub const CLOCK_KEY: &str = "petramond:clock";
pub const TIME_KEY: &str = "petramond:time";
/// How much daylight there is (0 night, 1 day; f32 LE), off the one curve the sky draws.
pub const DAYLIGHT_KEY: &str = "petramond:daylight";
pub const NIGHT_KEY: &str = "petramond:is_night";
pub const FROZEN_KEY: &str = "petramond:time_frozen";
pub const SKY_TIME_PARAM: &str = "petramond:time";
pub const SKY_LIGHT_PARAM: &str = "petramond:light";

const TRANSITION: f32 = 0.04;
const NIGHT_SKY_SCALE: f32 = 0.04;
const NIGHT_SKY_COLOR: [f32; 3] = [0.52, 0.62, 1.0];

pub const fn cycle_ticks_for_day_minutes(minutes: u32) -> u64 {
    let m = if minutes < 10 {
        10
    } else if minutes > 30 {
        30
    } else {
        minutes
    };
    m as u64 * 60 * 20 * 2
}

pub const fn fresh_clock(cycle: u64) -> u64 {
    cycle / 20
}

pub fn morning_after(clock: u64, cycle: u64) -> u64 {
    (clock / cycle + 1) * cycle + fresh_clock(cycle)
}

pub fn clock_from_fraction(t: f32, current_clock: u64, cycle: u64) -> u64 {
    let day = current_clock / cycle;
    let tick = (t.rem_euclid(1.0) * cycle as f32).round() as u64 % cycle;
    day * cycle + tick
}

pub fn day_fraction(clock: u64, cycle: u64) -> f32 {
    (clock % cycle) as f32 / cycle as f32
}

pub fn moon_phase(clock: u64, cycle: u64) -> f32 {
    ((clock / cycle) % MOON_PHASES) as f32
}

pub fn sky_params(day_fraction: f32, moon_phase: f32) -> ([f32; 4], [f32; 4]) {
    let t = day_fraction.rem_euclid(1.0);
    let day = daylight(t);
    let scale = NIGHT_SKY_SCALE + (1.0 - NIGHT_SKY_SCALE) * day;
    (
        [t, day, moon_phase, 0.0],
        [
            scale,
            lerp(NIGHT_SKY_COLOR[0], 1.0, day),
            lerp(NIGHT_SKY_COLOR[1], 1.0, day),
            lerp(NIGHT_SKY_COLOR[2], 1.0, day),
        ],
    )
}

/// The daylight the server last published; full day before the first publish.
pub fn published_daylight(world: &petramond_world::world::WorldData) -> f32 {
    world
        .world_kv_get(DAYLIGHT_KEY)
        .and_then(|b| <[u8; 4]>::try_from(b).ok())
        .map_or(1.0, f32::from_le_bytes)
}

pub fn daylight(t: f32) -> f32 {
    let h = (std::f32::consts::PI * TRANSITION).sin();
    smoothstep(-h, h, (std::f32::consts::TAU * t).sin())
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: u64 = DEFAULT_CYCLE_TICKS;

    #[test]
    fn a_fresh_server_world_starts_on_the_default_cycle() {
        assert_eq!(C, crate::world::session::DEFAULT_DAY_CYCLE_TICKS);
    }

    #[test]
    fn fraction_round_trips_within_the_current_day() {
        let clock = 3 * C + 100;
        assert_eq!(clock_from_fraction(0.25, clock, C), 3 * C + C / 4);
        assert_eq!(
            clock_from_fraction(day_fraction(clock, C), clock, C),
            clock,
            "a fraction read off a clock maps back onto it"
        );
        assert_eq!(moon_phase(clock, C), 3.0);
        assert_eq!(moon_phase((MOON_PHASES + 1) * C, C), 1.0, "phases wrap");
    }
}
