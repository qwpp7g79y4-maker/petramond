//! Mob census: runs the real game (installed packs, real terrain, real spawner and AI) with a
//! player standing in sites of one surface biome, and reports what lives there.
//!
//! `mobcensus <seed> <biome> [sites] [minutes] [stand] [tag=<key>] [reach=<blocks>]`
//! - `biome`: a surface biome key; sites are found inside it, away from its border.
//! - `stand`: `submerged` (default: a few blocks under the sea surface, or just above the floor
//!   in shallows) or `surface` (on the ground or on the water).
//! - `tag=<key>`: also count each species' values of that mob tag (what a guard carries, say).
//! - `reach=<blocks>`: count mobs this far out (default 64). Land animals spawn 50–128 blocks
//!   away, so judge them with a wider reach.
//!
//! Per site and per species: how many spawned within reach, how many are alive at the end,
//! the share of samples whose body sat in fluid, mean speed and the height band they kept.
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use petramond::tooling::biome::Biome;
use petramond::tooling::sim::{Sim, SimMob};
use petramond::tooling::worldgen::macro_surface_map;
use petramond_world::chunk::SEA_LEVEL;

const RENDER_DIST: i32 = 6;
/// Mobs farther than this from the player are not counted, unless `reach=` says otherwise.
const REACH: f64 = 64.0;
const SAMPLE_EVERY: u32 = 20;
const LOAD_TIMEOUT: Duration = Duration::from_secs(120);
/// Map cells (of `STRIDE` blocks) around a site that should share its biome; a biome too thin
/// for that (a beach) is searched with less, down to the site's own cell.
const INTERIOR: i32 = 5;
const STRIDE: i32 = 8;
const MAP_SIDE: usize = 400;
const SITE_SPACING: i32 = 400;

#[derive(Default)]
struct Track {
    key: &'static str,
    /// The census tag's value, when asked for and set.
    tagged: Option<String>,
    samples: u32,
    in_fluid: u32,
    speed: f64,
    y: (f64, f64),
    alive_at_end: bool,
}

fn main() {
    if let Err(e) = petramond::content::install_from_env(&[]) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    let mut args = std::env::args().skip(1);
    let seed: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let biome = args
        .next()
        .expect("usage: mobcensus <seed> <biome> [sites] [minutes] [stand]");
    let sites: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(3);
    let minutes: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(3.0);
    let rest: Vec<String> = args.collect();
    let surface = rest.iter().any(|a| a == "surface");
    let tag = rest
        .iter()
        .find_map(|a| a.strip_prefix("tag="))
        .map(str::to_owned);
    let reach = rest
        .iter()
        .find_map(|a| a.strip_prefix("reach="))
        .and_then(|r| r.parse().ok())
        .unwrap_or(REACH);

    let found = find_sites(seed, &biome, sites);
    if found.is_empty() {
        eprintln!(
            "no interior site of {biome} within {} blocks",
            MAP_SIDE as i32 * STRIDE / 2
        );
        std::process::exit(1);
    }
    let mut sim = Sim::new(seed, RENDER_DIST);
    let mut totals: BTreeMap<&'static str, u32> = BTreeMap::new();
    for (n, [x, z]) in found.into_iter().enumerate() {
        let started = Instant::now();
        let mut pos = [
            f64::from(x) + 0.5,
            f64::from(SEA_LEVEL) + 1.0,
            f64::from(z) + 0.5,
        ];
        while !sim.ready_at(pos) && started.elapsed() < LOAD_TIMEOUT {
            sim.run(pos, 10);
        }
        let floor = sim.floor_y(x, z);
        pos[1] = stand_y(floor, surface);
        println!(
            "site {} at {x},{:.0},{z} ({biome}, floor {}) loaded in {:.1}s",
            n + 1,
            pos[1],
            floor.map_or("?".into(), |f| f.to_string()),
            started.elapsed().as_secs_f32()
        );
        let tracks = census(&mut sim, pos, minutes, tag.as_deref(), reach);
        report(&tracks, tag.as_deref(), reach);
        for t in tracks.values() {
            *totals.entry(t.key).or_default() += 1;
        }
    }
    println!("all sites:");
    for (key, n) in totals {
        println!("  {key:<34} {n}");
    }
}

/// Where the player stands: on the floor or the water when `surface`, else a few blocks under
/// the sea surface (or just above a shallow floor).
fn stand_y(floor: Option<i32>, surface: bool) -> f64 {
    let floor = floor.unwrap_or(SEA_LEVEL - 10);
    if surface {
        return f64::from(floor.max(SEA_LEVEL)) + 1.0;
    }
    f64::from((SEA_LEVEL - 4).max(floor + 2).min(SEA_LEVEL - 1))
}

fn census(
    sim: &mut Sim,
    pos: [f64; 3],
    minutes: f64,
    tag: Option<&str>,
    reach: f64,
) -> BTreeMap<u64, Track> {
    let ticks = (minutes * 60.0 * 20.0) as u32;
    let mut tracks: BTreeMap<u64, Track> = BTreeMap::new();
    let mut ran = 0;
    while ran < ticks {
        sim.run(pos, SAMPLE_EVERY);
        ran += SAMPLE_EVERY;
        let near: Vec<SimMob> = sim
            .mobs()
            .into_iter()
            .filter(|m| within(m, pos, reach))
            .collect();
        for t in tracks.values_mut() {
            t.alive_at_end = false;
        }
        for m in near {
            let t = tracks.entry(m.id).or_insert_with(|| Track {
                key: m.key,
                y: (m.pos[1], m.pos[1]),
                ..Track::default()
            });
            if let Some(tag) = tag {
                if let Some((_, v)) = m.tags.iter().find(|(k, _)| k == tag) {
                    t.tagged = Some(v.clone());
                }
            }
            t.samples += 1;
            t.in_fluid += u32::from(m.in_fluid);
            t.speed += f64::from(m.vel.iter().map(|v| v * v).sum::<f32>().sqrt());
            t.y = (t.y.0.min(m.pos[1]), t.y.1.max(m.pos[1]));
            t.alive_at_end = true;
        }
    }
    tracks
}

fn within(m: &SimMob, pos: [f64; 3], reach: f64) -> bool {
    let (dx, dz) = (m.pos[0] - pos[0], m.pos[2] - pos[2]);
    dx * dx + dz * dz <= reach * reach
}

fn report(tracks: &BTreeMap<u64, Track>, tag: Option<&str>, reach: f64) {
    let mut by_key: BTreeMap<&str, Vec<&Track>> = BTreeMap::new();
    for t in tracks.values() {
        by_key.entry(t.key).or_default().push(t);
    }
    if by_key.is_empty() {
        println!("  nothing came within {reach} blocks");
        return;
    }
    println!(
        "  {:<34} {:>7} {:>6} {:>9} {:>7} {:>13}",
        "species", "spawned", "alive", "in fluid", "speed", "height"
    );
    for (key, ts) in by_key {
        let samples: u32 = ts.iter().map(|t| t.samples).sum();
        let in_fluid: u32 = ts.iter().map(|t| t.in_fluid).sum();
        let speed: f64 = ts.iter().map(|t| t.speed).sum::<f64>() / f64::from(samples.max(1));
        let lo = ts.iter().map(|t| t.y.0).fold(f64::MAX, f64::min);
        let hi = ts.iter().map(|t| t.y.1).fold(f64::MIN, f64::max);
        println!(
            "  {key:<34} {:>7} {:>6} {:>8.0}% {:>7.2} {:>6.0}..{:<6.0}",
            ts.len(),
            ts.iter().filter(|t| t.alive_at_end).count(),
            100.0 * f64::from(in_fluid) / f64::from(samples.max(1)),
            speed,
            lo,
            hi
        );
        if let Some(tag) = tag {
            let mut values: BTreeMap<&str, u32> = BTreeMap::new();
            for t in &ts {
                if let Some(v) = &t.tagged {
                    *values.entry(v.as_str()).or_default() += 1;
                }
            }
            if !values.is_empty() {
                let list: Vec<String> = values.iter().map(|(v, n)| format!("{v} {n}")).collect();
                println!("      {tag}: {}", list.join(", "));
            }
        }
    }
}

/// Up to `count` points as deep inside `biome` as it allows, nearest the origin first,
/// `SITE_SPACING` apart.
fn find_sites(seed: u32, biome: &str, count: usize) -> Vec<[i32; 2]> {
    let map = macro_surface_map(seed, MAP_SIDE, STRIDE);
    (0..=INTERIOR)
        .rev()
        .map(|interior| sites_inside(&map, biome, count, interior))
        .find(|sites| !sites.is_empty())
        .unwrap_or_default()
}

fn sites_inside(
    map: &petramond::tooling::worldgen::MacroSurfaceMap,
    biome: &str,
    count: usize,
    interior: i32,
) -> Vec<[i32; 2]> {
    let side = map.side as i32;
    let at = |gx: i32, gz: i32| -> Option<&'static str> {
        ((0..side).contains(&gx) && (0..side).contains(&gz))
            .then(|| Biome::from_id(map.biomes[(gz * side + gx) as usize]).key())
    };
    let half = side / 2 * STRIDE;
    let mut candidates: Vec<[i32; 2]> = Vec::new();
    for gz in 0..side {
        for gx in 0..side {
            if at(gx, gz) != Some(biome) {
                continue;
            }
            let inside = (-interior..=interior)
                .all(|dz| (-interior..=interior).all(|dx| at(gx + dx, gz + dz) == Some(biome)));
            if inside {
                candidates.push([gx * STRIDE - half, gz * STRIDE - half]);
            }
        }
    }
    candidates.sort_by_key(|[x, z]| i64::from(*x).pow(2) + i64::from(*z).pow(2));
    let mut sites: Vec<[i32; 2]> = Vec::new();
    for c in candidates {
        let far = sites
            .iter()
            .all(|s| (s[0] - c[0]).abs().max((s[1] - c[1]).abs()) >= SITE_SPACING);
        if far {
            sites.push(c);
            if sites.len() == count {
                break;
            }
        }
    }
    sites
}
