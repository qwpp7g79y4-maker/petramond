//! Shoal rocks: the warm shallows' landscape inside the water. Pale limestone rises from the sand
//! as arches to swim through, mushroom rocks under overhanging caps, spires reaching for the
//! light, boulders hollowed into grottos and long ridges holed by windows. They gather in rock
//! gardens with open sand and seagrass between, and life grows on them: coral gardens on their
//! tops in the reef's colour themes, plate corals bracketing out from their walls, warm glowpods
//! sheltering under the overhangs, rubble and anemones round their feet.
//!
//! A site's plan is derived once from positional reads (the floor around it, the biome at its
//! centre) and clipped into every section it reaches.

use std::collections::BTreeSet;

use mod_sdk::build::{Derived, Draw, Heights, Material, Noise2, Noise3, Site, SiteGrid};
use mod_sdk::*;

use super::reef::{self, Coral, CoralRow, Crown, Form, Species};
use super::sited::{HostTerrain, Sited, Terrain};
use super::sunken::Sunken;
use crate::keys;

/// Sections a shoal can write: the warm shallows' floors up to the waterline.
pub const GEN_FILTER: GenFeatureFilter = GenFeatureFilter::y_band(-16, 72)
    .without_blocks()
    .without_columns();

/// The farthest a formation reaches from its site: the widest arch or longest ridge, its lumps
/// and the life on its flanks.
const REACH: i32 = 22;

/// Candidate sites, fully jittered; the rock-garden field decides which hold rock.
const GRID: SiteGrid = SiteGrid {
    salt: GenRng::salt("exploration:shoal"),
    spacing: 17,
    border: 2,
    chance: 1.0,
    reach: REACH,
};

/// Rock gardens: low-frequency noise of where the shallows turn rocky.
const SALT_FIELD: u32 = 0x5A01_6A2D;
const FIELD_SCALE: f32 = 1.0 / 100.0;
/// Field strength from which a site may hold rock, and at which it is as rocky as it gets.
const FIELD_OPEN: f32 = -0.75;
const FIELD_FULL: f32 = 0.1;
/// The rockiest garden's share of sites holding a formation.
const CROWDED: f32 = 0.9;

const SALT_LUMP: u32 = 0x5A01_6A2E;
const SALT_GARDEN: u32 = 0x5A01_6A2F;
const SALT_WALL: u32 = 0x5A01_6A31;
const SALT_DRESS: u64 = 0x5A01_6A30;

/// Water kept over the highest rock.
const HEADROOM: i32 = 2;
/// Shallower water than this grows no formation.
const MIN_ROOM: i32 = 5;
/// The smallest formation worth building, in cells.
const MIN_CELLS: usize = 12;

/// Rock tops a colony's crust or brain coral covers: life comes in patches on pale rock.
const SKIN_PER_MILLE: u64 = 450;
/// Rock tops crusted by coral rubble where no colony grows.
const CRUST_PER_MILLE: u64 = 250;
/// The share of a reef colony's crowns a shoal top grows.
const CROWN_SHARE_PER_MILLE: u64 = 600;
/// Rock sides crusted the same way.
const SIDE_CRUST_PER_MILLE: u64 = 150;
/// A glowpod on a bare top.
const TOP_LANTERN_PER_MILLE: u64 = 30;
/// Plate corals on a wall where its bracket cluster grows.
const PLATE_PER_MILLE: u64 = 420;
/// The wall cluster's threshold: plates come in clumps up a wall, not dotted over it.
const PLATE_CLUSTER: f32 = 0.15;
/// Under an overhang: a glowpod, else an anemone.
const SHELTER_LANTERN: u64 = 350;
const SHELTER_ANEMONE: u64 = 150;
/// Round a formation's foot.
const FOOT_RUBBLE: u64 = 100;
const FOOT_ANEMONE: u64 = 50;
const FOOT_SEAGRASS: u64 = 300;
/// A table coral seeding a wide table over a broad top.
const WIDE_TABLE_PER_MILLE: u64 = 300;

/// The warm seas that grow shoals, and each one's share of the rock gardens' sites.
const SEAS: [(&str, f32); 2] = [(keys::WARM_OCEAN, 1.0), (keys::CORAL_REEF, 0.35)];

pub fn warm_seas() -> Vec<(u8, f32)> {
    SEAS.into_iter()
        .filter_map(|(key, share)| Some((resolve_surface_biome(key)?, share)))
        .collect()
}

/// What a shoal is built of and dressed with.
pub struct Look {
    pub rock: Material,
    /// Coral rubble crusting the rock.
    pub crust: Material,
    pub lantern: Material,
    pub anemone: Material,
    pub staghorn: Material,
    pub seagrass: Material,
    pub palette: Vec<Coral<Material>>,
}

impl Look {
    pub fn resolve() -> Option<Look> {
        let ids = Coral::palette(blocks_with_data_as::<CoralRow>(keys::CORAL));
        let flat: Vec<BlockId> = ids
            .iter()
            .flat_map(|c| reef::FORM_LIST.iter().filter_map(|&f| c.form(f)))
            .collect();
        let names: FxHashMap<BlockId, String> = flat
            .iter()
            .copied()
            .zip(block_names(flat.clone()))
            .filter_map(|(id, name)| Some((id, name?)))
            .collect();
        let palette = ids
            .iter()
            .map(|c| {
                let mut out = Coral::default();
                out.hue = c.hue;
                for &form in &reef::FORM_LIST {
                    out.set(
                        form,
                        c.form(form)
                            .and_then(|id| names.get(&id))
                            .map(|n| Material::named(n)),
                    );
                }
                out
            })
            .collect();
        for key in [
            keys::LIMESTONE,
            keys::REEF_ROCK,
            keys::GLOWPOD_AMBER,
            keys::ANEMONE,
            keys::STAGHORN_CORAL,
            keys::SEAGRASS,
        ] {
            resolve_block_logged(key)?;
        }
        Some(Look {
            rock: Material::named(keys::LIMESTONE),
            crust: Material::named(keys::REEF_ROCK),
            lantern: Material::named(keys::GLOWPOD_AMBER),
            anemone: Material::named(keys::ANEMONE),
            staghorn: Material::named(keys::STAGHORN_CORAL),
            seagrass: Material::named(keys::SEAGRASS),
            palette,
        })
    }
}

pub struct Shoals {
    seas: Vec<(u8, f32)>,
    look: Look,
    sites: Sited,
}

impl Shoals {
    pub fn new(seas: Vec<(u8, f32)>, look: Look) -> Option<Shoals> {
        (!seas.is_empty()).then(|| Shoals {
            seas,
            look,
            sites: Sited::new("exploration_shoal"),
        })
    }

    pub fn generate(&mut self, ctx: &GenCtx) -> GenOutput {
        let (seed, sea, seas, look) = (ctx.seed(), ctx.sea_level(), &self.seas, &self.look);
        self.sites.generate(&GRID, ctx, |site| {
            derive(seed, sea, site, seas, look, &HostTerrain)
        })
    }
}

/// The rock at `site`: one formation dressed with life, or nothing where the field leaves open
/// sand, the water is shallow or the sea is not warm.
pub fn derive(
    seed: u32,
    sea: i32,
    site: Site,
    seas: &[(u8, f32)],
    look: &Look,
    terrain: &dyn Terrain,
) -> Derived {
    let [cx, cz] = site.center;
    let mut rng = GRID.rng(seed, site);
    let field = Noise2(seed ^ SALT_FIELD).at(cx as f32 * FIELD_SCALE, cz as f32 * FIELD_SCALE);
    let rocky = ((field - FIELD_OPEN) / (FIELD_FULL - FIELD_OPEN)).clamp(0.0, 1.0);
    if !rng.roll(rocky * CROWDED) {
        return Derived::Nothing;
    }
    let Some(biomes) = terrain.biomes(vec![site.center]) else {
        return Derived::Unavailable;
    };
    let Some(&(_, share)) = seas.iter().find(|(b, _)| Some(b) == biomes.first()) else {
        return Derived::Nothing;
    };
    if !rng.roll(share) {
        return Derived::Nothing;
    }
    let Some(floor) = terrain.heights([cx - REACH, cz - REACH], [cx + REACH, cz + REACH]) else {
        return Derived::Unavailable;
    };
    let waterline = sea - 1;
    let Some(f0) = floor.get(cx, cz) else {
        return Derived::Nothing;
    };
    let room = waterline - HEADROOM - f0;
    if room < MIN_ROOM {
        return Derived::Nothing;
    }
    let formation = Formation::roll(&mut rng, seed, room);
    let cells = formation.cells(site.center, f0, waterline - HEADROOM, &floor);
    if cells.len() < MIN_CELLS {
        return Derived::Nothing;
    }
    let theme = Theme::roll(&mut rng, &look.palette);
    let mut sunken = Sunken::new(sea, &floor);
    dress(seed, &cells, &floor, waterline, &theme, look, &mut sunken);
    let plan = sunken.finish();
    if plan.is_empty() {
        Derived::Nothing
    } else {
        Derived::Plan(Box::new(plan))
    }
}

/// What a formation is.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A natural bridge to swim through, sometimes with one leg fallen.
    Arch,
    /// An eroded stem under a wide overhanging cap.
    Mushroom,
    /// A tapering pinnacle reaching for the light, ringed by a ledge.
    Spire,
    /// A boulder hollowed into a grotto, bored through so it can be swum.
    Grotto,
    /// A long, wandering ridge, holed by a window where it stands tall.
    Ridge,
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Arch {
        dir: [f32; 2],
        span: f32,
        rise: f32,
        tube: f32,
        broken: bool,
    },
    Mushroom {
        stem: f32,
        height: f32,
        cap: f32,
        thick: f32,
        lean: [f32; 2],
    },
    Spire {
        base: f32,
        height: f32,
        lean: [f32; 2],
        ledge: (f32, f32),
    },
    Grotto {
        dir: [f32; 2],
        radii: [f32; 3],
        shell: f32,
        bore: f32,
    },
    Ridge {
        dir: [f32; 2],
        half: f32,
        width: f32,
        height: f32,
        wave: f32,
        phase: f32,
        window: Option<f32>,
    },
}

/// One formation's shape, in blocks around its site and above its floor, eroded by lumps.
#[derive(Clone, Copy, Debug)]
pub struct Formation {
    shape: Shape,
    lumps: u32,
}

impl Formation {
    /// A formation that fits `room` cells of water over the floor.
    pub fn roll(rng: &mut GenRng, seed: u32, room: i32) -> Formation {
        let room = room as f32;
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let dir = [angle.cos(), angle.sin()];
        let mut kinds: Vec<(Kind, f32)> = vec![(Kind::Ridge, 3.0), (Kind::Mushroom, 2.5)];
        if room >= 6.0 {
            kinds.extend([(Kind::Arch, 2.2), (Kind::Grotto, 2.0)]);
        }
        if room >= 8.0 {
            kinds.push((Kind::Spire, 1.6));
        }
        let total: f32 = kinds.iter().map(|(_, w)| w).sum();
        let mut pick = rng.range(0.0, total);
        let kind = kinds
            .iter()
            .find(|(_, w)| {
                pick -= w;
                pick < 0.0
            })
            .map_or(Kind::Ridge, |(k, _)| *k);
        let lean = {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let l = rng.range(0.0, 0.18);
            [a.cos() * l, a.sin() * l]
        };
        let shape = match kind {
            Kind::Arch => {
                let tube = rng.range(1.4, 2.2);
                let rise = rng.range(3.5, (room - tube - 0.5).clamp(3.6, 9.0));
                Shape::Arch {
                    dir,
                    span: rng.range(3.5, 8.0),
                    rise,
                    tube,
                    broken: rng.roll(0.25),
                }
            }
            Kind::Mushroom => {
                let thick = rng.range(1.5, 2.5);
                Shape::Mushroom {
                    stem: rng.range(1.2, 2.2),
                    height: rng.range(2.5, (room - thick).clamp(2.6, 7.0)),
                    cap: rng.range(3.0, 5.5),
                    thick,
                    lean,
                }
            }
            Kind::Spire => {
                let height = rng.range(room * 0.7, room);
                Shape::Spire {
                    base: rng.range(2.0, 3.5),
                    height,
                    lean,
                    ledge: (rng.range(height * 0.3, height * 0.7), rng.range(0.8, 1.6)),
                }
            }
            Kind::Grotto => {
                let tall = rng.range(3.5, (room * 0.8).clamp(3.6, 6.0));
                Shape::Grotto {
                    dir,
                    radii: [rng.range(5.0, 7.5), tall, rng.range(4.0, 6.0)],
                    shell: rng.range(1.6, 2.2),
                    bore: rng.range(1.4, 1.9),
                }
            }
            Kind::Ridge => {
                let height = rng.range(2.5, (room * 0.75).clamp(2.6, 7.0));
                Shape::Ridge {
                    dir,
                    half: rng.range(6.0, 11.0),
                    width: rng.range(1.8, 3.2),
                    height,
                    wave: rng.range(0.5, 2.5),
                    phase: rng.range(0.0, std::f32::consts::TAU),
                    window: (height >= 4.0 && rng.roll(0.6)).then(|| rng.range(-0.4, 0.4)),
                }
            }
        };
        Formation {
            shape,
            lumps: seed ^ SALT_LUMP,
        }
    }

    #[cfg(test)]
    pub fn kind(&self) -> Kind {
        match self.shape {
            Shape::Arch { .. } => Kind::Arch,
            Shape::Mushroom { .. } => Kind::Mushroom,
            Shape::Spire { .. } => Kind::Spire,
            Shape::Grotto { .. } => Kind::Grotto,
            Shape::Ridge { .. } => Kind::Ridge,
        }
    }

    /// Distance to the shape's surface (negative inside) at `p`, blocks from the site at floor
    /// level; `w` is the world point, for lumps that line up across sections.
    fn distance(&self, p: [f32; 3], w: [f32; 3]) -> f32 {
        let lumps = Noise3(self.lumps).at(w[0] * 0.3, w[1] * 0.4, w[2] * 0.3) * 0.8
            + Noise3(self.lumps ^ 0x55).at(w[0] * 0.7, w[1] * 0.7, w[2] * 0.7) * 0.35;
        self.shape.distance(p) + lumps
    }

    /// The rock cells of a formation at `site` over a floor at `f0`, no higher than `top`, every
    /// one joined to the sea floor.
    pub fn cells(&self, site: [i32; 2], f0: i32, top: i32, floor: &Heights) -> BTreeSet<[i32; 3]> {
        let [cx, cz] = site;
        let mut solid = BTreeSet::new();
        for z in cz - REACH..=cz + REACH {
            for x in cx - REACH..=cx + REACH {
                let Some(fx) = floor.get(x, z) else { continue };
                for y in fx..=top {
                    let p = [(x - cx) as f32, (y - f0) as f32 - 0.5, (z - cz) as f32];
                    let w = [x as f32, y as f32, z as f32];
                    if self.distance(p, w) < 0.0 {
                        solid.insert([x, y, z]);
                    }
                }
            }
        }
        attached(solid, floor)
    }
}

impl Shape {
    fn distance(&self, p: [f32; 3]) -> f32 {
        match *self {
            Shape::Arch {
                dir,
                span,
                rise,
                tube,
                broken,
            } => arch(p, dir, span, rise, tube, broken),
            Shape::Mushroom {
                stem,
                height,
                cap,
                thick,
                lean,
            } => mushroom(p, stem, height, cap, thick, lean),
            Shape::Spire {
                base,
                height,
                lean,
                ledge,
            } => spire(p, base, height, lean, ledge),
            Shape::Grotto {
                dir,
                radii,
                shell,
                bore,
            } => grotto(p, dir, radii, shell, bore),
            Shape::Ridge {
                dir,
                half,
                width,
                height,
                wave,
                phase,
                window,
            } => ridge(p, dir, [half, width, height], wave, phase, window),
        }
    }
}

fn length(a: f32, b: f32) -> f32 {
    (a * a + b * b).sqrt()
}

/// `p` along `dir` and across it.
fn turned([x, _, z]: [f32; 3], dir: [f32; 2]) -> (f32, f32) {
    (x * dir[0] + z * dir[1], -x * dir[1] + z * dir[0])
}

/// A half-ellipse tube standing in the plane of `dir`, its legs thicker; a broken one has lost
/// its far leg below the shoulder and hangs as a hook.
fn arch(p: [f32; 3], dir: [f32; 2], span: f32, rise: f32, tube: f32, broken: bool) -> f32 {
    let (along, across) = turned(p, dir);
    let y = p[1];
    if broken && along > 0.0 && y < rise * 0.55 {
        return 1.0;
    }
    let on_curve = if y < 0.0 {
        along.abs() - span
    } else {
        (length(along / span, y / rise) - 1.0) * span.min(rise)
    };
    let thick = tube * (1.0 + 0.35 * (1.0 - y / rise).clamp(0.0, 1.0));
    length(on_curve, across) - thick
}

/// A waisted stem under a domed cap flattened on top.
fn mushroom(
    [x, y, z]: [f32; 3],
    stem: f32,
    height: f32,
    cap: f32,
    thick: f32,
    lean: [f32; 2],
) -> f32 {
    let (sx, sz) = (x - lean[0] * y, z - lean[1] * y);
    let waist = 0.8 + 0.4 * ((y / height) - 0.5).abs();
    let stalk = (length(sx, sz) - stem * waist).max(y - height);
    let (kx, kz) = (x - lean[0] * height, z - lean[1] * height);
    let ky = y - height - thick * 0.35;
    let dome = (length(length(kx / cap, kz / cap), ky / thick) - 1.0) * thick;
    stalk.min(dome.max(y - height - thick))
}

/// A leaning cone with a ledge ringing it.
fn spire([x, y, z]: [f32; 3], base: f32, height: f32, lean: [f32; 2], ledge: (f32, f32)) -> f32 {
    let (sx, sz) = (x - lean[0] * y, z - lean[1] * y);
    let t = (y / height).clamp(0.0, 1.0);
    let bulge = ledge.1 * (1.0 - (y - ledge.0).abs() / 1.2).max(0.0);
    let radius = base * (1.0 - t).powf(0.75) + 0.6 + bulge;
    (length(sx, sz) - radius).max(y - height)
}

/// An ellipsoid shell, hollow, bored through along `dir`.
fn grotto(p: [f32; 3], dir: [f32; 2], radii: [f32; 3], shell: f32, bore: f32) -> f32 {
    let (along, across) = turned(p, dir);
    let dy = p[1] - (radii[1] * 0.5 - 0.5);
    let ellipse = |r: [f32; 3]| {
        (length(length(along / r[0], dy / r[1]), across / r[2]) - 1.0) * r[0].min(r[1]).min(r[2])
    };
    let outer = ellipse(radii);
    let inner = ellipse(radii.map(|r| (r - shell).max(0.5)));
    let tunnel = length(across, dy) - bore;
    outer.max(-inner).max(-tunnel)
}

/// A wandering ridge, tapering to its ends, holed by a window where it stands tall.
fn ridge(
    p: [f32; 3],
    dir: [f32; 2],
    [half, width, height]: [f32; 3],
    wave: f32,
    phase: f32,
    window: Option<f32>,
) -> f32 {
    let (along, across) = turned(p, dir);
    let y = p[1];
    let t = along / half;
    if t.abs() > 1.3 {
        return t.abs() - 1.3 + 1.0;
    }
    let fall = (1.0 - t * t).max(0.0);
    let w = (width * fall.sqrt()).max(0.9);
    let h = (height * fall.powf(0.6)).max(1.0);
    let off = wave * (t * std::f32::consts::PI * 1.3 + phase).sin();
    let dx = across - off;
    let body = if y < 0.0 {
        dx.abs() - w
    } else {
        (length(dx / w, y / h) - 1.0) * w.min(h)
    };
    match window {
        Some(at) if h >= 4.0 => body.max(-(length(along - at * half, y - h * 0.45) - 1.3)),
        _ => body,
    }
}

/// The cells joined, face to face, to one standing on (or in) the sea floor: no rock floats.
fn attached(solid: BTreeSet<[i32; 3]>, floor: &Heights) -> BTreeSet<[i32; 3]> {
    let mut kept = BTreeSet::new();
    let mut queue: Vec<[i32; 3]> = solid
        .iter()
        .copied()
        .filter(|&[x, y, z]| floor.get(x, z).is_some_and(|f| y <= f + 1))
        .collect();
    kept.extend(queue.iter().copied());
    while let Some([x, y, z]) = queue.pop() {
        for n in [
            [x + 1, y, z],
            [x - 1, y, z],
            [x, y + 1, z],
            [x, y - 1, z],
            [x, y, z + 1],
            [x, y, z - 1],
        ] {
            if solid.contains(&n) && kept.insert(n) {
                queue.push(n);
            }
        }
    }
    kept
}

/// A formation's colours: the reef's theme of a dominant colour and its neighbours on the wheel,
/// grown as four colonies over the rock's tops.
struct Theme {
    colours: [usize; 3],
    colonies: [Species; 4],
    salt: u32,
}

impl Theme {
    fn roll(rng: &mut GenRng, palette: &[Coral<Material>]) -> Theme {
        let colours = reef::theme_colours(rng, palette.len());
        Theme {
            colours,
            colonies: reef::colonies(rng, palette, colours),
            salt: rng.next_u64() as u32,
        }
    }

    fn species(&self, x: i32, z: i32) -> Species {
        let patch = Noise2(self.salt ^ SALT_GARDEN).at(x as f32 * 0.25, z as f32 * 0.25);
        self.colonies[match patch {
            p if p < -0.3 => 0,
            p if p < 0.05 => 1,
            p if p < 0.4 => 2,
            _ => 3,
        }]
    }

    /// The colour of the plate corals bracketing out from the walls.
    fn plates(&self, palette: &[Coral<Material>]) -> Option<Material> {
        self.colours
            .into_iter()
            .skip(1)
            .chain(Some(self.colours[0]))
            .find_map(|c| palette.get(c)?.form(Form::Plate))
    }
}

const SIDES: [[i32; 2]; 4] = [[1, 0], [-1, 0], [0, 1], [0, -1]];

/// Builds the rock and grows life on it.
fn dress(
    seed: u32,
    cells: &BTreeSet<[i32; 3]>,
    floor: &Heights,
    waterline: i32,
    theme: &Theme,
    look: &Look,
    out: &mut Sunken,
) {
    let d = Dresser {
        seed,
        cells,
        floor,
        waterline,
        theme,
        look,
    };
    d.rock(out);
    d.crowns(out);
    d.brackets(out);
    d.shelter(out);
    d.feet(out);
}

/// What dressing one formation reads.
struct Dresser<'a> {
    seed: u32,
    cells: &'a BTreeSet<[i32; 3]>,
    floor: &'a Heights,
    waterline: i32,
    theme: &'a Theme,
    look: &'a Look,
}

impl Dresser<'_> {
    fn solid(&self, c: [i32; 3]) -> bool {
        self.cells.contains(&c)
    }

    fn top(&self, c: [i32; 3]) -> bool {
        self.solid(c) && !self.solid([c[0], c[1] + 1, c[2]])
    }

    fn roll(&self, c: [i32; 3], salt: u64, per_mille: u64) -> bool {
        GenRng::positional(self.seed, SALT_DRESS ^ salt, c[0], c[1], c[2]).next_u64() % 1000
            < per_mille
    }

    /// Water this formation may grow into: not rock, not grown yet, under the waterline, over
    /// the floor.
    fn open(&self, c: [i32; 3], out: &Sunken) -> bool {
        !self.solid(c)
            && !out.plan.occupied(c)
            && c[1] < self.waterline
            && self.floor.get(c[0], c[2]).is_some_and(|f| c[1] > f)
    }

    fn form(&self, colour: usize, form: Form) -> Option<Material> {
        self.look.palette.get(colour)?.form(form)
    }

    fn crown(&self, crown: Crown) -> Option<Material> {
        match crown {
            Crown::Staghorn => Some(self.look.staghorn),
            Crown::Fan(c) => self.form(c, Form::Fan),
            Crown::Table(c) => self.form(c, Form::Table),
            Crown::Tube(c) => self.form(c, Form::Tube),
            Crown::Whip(c) => self.form(c, Form::Whip),
            Crown::Anemone => Some(self.look.anemone),
            Crown::Lantern => Some(self.look.lantern),
        }
    }

    /// The rock itself, its tops crusted in patches by colonies or coral rubble.
    fn rock(&self, out: &mut Sunken) {
        let look = self.look;
        for &c @ [x, y, z] in self.cells {
            let exposed_side = SIDES
                .iter()
                .any(|[dx, dz]| !self.solid([x + dx, y, z + dz]));
            let species = self.theme.species(x, z);
            let material = if self.top(c) {
                let grown = match species {
                    Species::Brain(k) if self.roll(c, 10, SKIN_PER_MILLE) => {
                        self.form(k, Form::Brain)
                    }
                    Species::Crust(k) if self.roll(c, 10, SKIN_PER_MILLE) => {
                        self.form(k, Form::Block)
                    }
                    _ => None,
                };
                grown.unwrap_or(if self.roll(c, 1, CRUST_PER_MILLE) {
                    look.crust
                } else {
                    look.rock
                })
            } else if let (Species::Brain(k), true) = (
                species,
                self.top([x, y + 1, z]) && self.roll([x, y + 1, z], 10, SKIN_PER_MILLE),
            ) {
                // Brain coral grows two deep.
                self.form(k, Form::Brain).unwrap_or(look.rock)
            } else if exposed_side && self.roll(c, 2, SIDE_CRUST_PER_MILLE) {
                look.crust
            } else {
                look.rock
            };
            out.put(c, &material);
        }
    }

    /// Crowns on the tops: the colony's plants, now and then a wide table.
    fn crowns(&self, out: &mut Sunken) {
        for &c @ [x, y, z] in self.cells {
            let above = [x, y + 1, z];
            if !self.top(c) || !self.open(above, out) {
                continue;
            }
            let (crown, per_mille) = self.theme.species(x, z).crown();
            let per_mille = per_mille * CROWN_SHARE_PER_MILLE / 1000;
            let crown = if self.roll(c, 3, per_mille) {
                crown
            } else {
                self.roll(c, 4, TOP_LANTERN_PER_MILLE)
                    .then_some(Crown::Lantern)
            };
            let Some(crown) = crown else { continue };
            if let Crown::Table(k) = crown {
                self.widen_table(k, above, out);
            }
            if let Some(m) = self.crown(crown) {
                out.put(above, &m);
            }
        }
    }

    /// A table coral's plate spread over the eight cells around its stalk, where all are open.
    fn widen_table(&self, colour: usize, at: [i32; 3], out: &mut Sunken) {
        let Some(plate) = self.form(colour, Form::Plate) else {
            return;
        };
        if !self.roll(at, 5, WIDE_TABLE_PER_MILLE) {
            return;
        }
        let ring: Vec<[i32; 3]> = (-1..=1)
            .flat_map(|dz| (-1..=1).map(move |dx| [at[0] + dx, at[1], at[2] + dz]))
            .filter(|&n| n != at)
            .collect();
        if ring.iter().all(|&n| self.open(n, out)) {
            for n in ring {
                out.put(n, &plate);
            }
        }
    }

    /// Plate corals bracketing out from the walls, in clumps.
    fn brackets(&self, out: &mut Sunken) {
        let Some(plate) = self.theme.plates(&self.look.palette) else {
            return;
        };
        for &[x, y, z] in self.cells {
            for [dx, dz] in SIDES {
                let n = [x + dx, y, z + dz];
                let below = [n[0], y - 1, n[2]];
                let on_wall = self.floor.get(n[0], n[2]).is_some_and(|f| y > f + 1)
                    && !self.solid(below)
                    && out.plan.get(below).is_none_or(|m| *m != plate);
                let cluster = Noise2(self.theme.salt ^ SALT_WALL)
                    .at(n[0] as f32 * 0.4, (n[2] + y * 3) as f32 * 0.4);
                if on_wall
                    && self.open(n, out)
                    && cluster > PLATE_CLUSTER
                    && self.roll(n, 6, PLATE_PER_MILLE)
                {
                    out.put(n, &plate);
                }
            }
        }
    }

    /// Shelter under the overhangs: glowpods and anemones on the sand beneath.
    fn shelter(&self, out: &mut Sunken) {
        for &[x, y, z] in self.cells {
            let Some(f) = self.floor.get(x, z) else {
                continue;
            };
            let ground = [x, f + 1, z];
            let overhang = !self.solid([x, y - 1, z])
                && y - 1 > f
                && (f + 1..y).all(|yy| !self.solid([x, yy, z]));
            if !overhang || !self.open(ground, out) {
                continue;
            }
            if self.roll(ground, 7, SHELTER_LANTERN) {
                out.put(ground, &self.look.lantern);
            } else if self.roll(ground, 8, SHELTER_LANTERN + SHELTER_ANEMONE) {
                out.put(ground, &self.look.anemone);
            }
        }
    }

    /// Rubble, anemones and seagrass round the formation's foot.
    fn feet(&self, out: &mut Sunken) {
        let feet: BTreeSet<[i32; 3]> = self
            .cells
            .iter()
            .flat_map(|&[x, _, z]| SIDES.map(|[dx, dz]| [x + dx, z + dz]))
            .filter_map(|[x, z]| Some([x, self.floor.get(x, z)? + 1, z]))
            .filter(|&[x, y, z]| SIDES.iter().any(|[dx, dz]| self.solid([x + dx, y, z + dz])))
            .collect();
        for c in feet {
            if !self.open(c, out) {
                continue;
            }
            let pick =
                GenRng::positional(self.seed, SALT_DRESS ^ 9, c[0], c[1], c[2]).next_u64() % 1000;
            let m = if pick < FOOT_RUBBLE {
                self.look.crust
            } else if pick < FOOT_RUBBLE + FOOT_ANEMONE {
                self.look.anemone
            } else if pick < FOOT_RUBBLE + FOOT_ANEMONE + FOOT_SEAGRASS {
                self.look.seagrass
            } else {
                continue;
            };
            out.put(c, &m);
        }
    }
}

#[cfg(test)]
mod tests;
