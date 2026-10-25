//! Coral heads on a site lattice with sand channels between them, the way a real reef stands in
//! bommies rather than lying as a carpet. A head is a dome of reef rock, the larger ones undercut
//! into a cap on a stem. Its surface splits into four colonies, each one species in one colour of
//! the head's theme: a dominant colour with its neighbours on the colour wheel and now and then a
//! contrasting accent, so colour comes in clumps on a calm framework and neighbouring heads
//! differ. A bed's [`ReefField`] sets how densely heads stand: a reef proper, or the odd patch
//! reef in open warm water. Amber glowpods shelter under the caps and around the bases.
//!
//! The palette is data: every block row with `exploration:coral` row data is one form of one
//! colour ([`Coral`]); a colour grows only the forms it has rows for.
//!
//! SEAM CONTRACT (see `flora`): a column's reef is a pure function of `(seed, column, room)`.

use mod_sdk::build::Noise2;
use mod_sdk::*;
use serde::Deserialize;

const SALT_HEAD: u64 = 0x0CEA_0000_0000_0020;
const SALT_CELL: u64 = 0x0CEA_0000_0000_0021;
const SALT_OUTLINE: u32 = 0x0CEA_0021;
const SALT_LUMP: u32 = 0x0CEA_0023;

/// Heads this wide or wider stand on a stem under an overhanging cap.
const UNDERCUT_FROM: i32 = 5;
/// The share of the radius that stands at full height before the rim slopes away.
const MESA: f32 = 0.5;
/// How far the rim falls, as a share of the head's height.
const RIM_FALL: f32 = 0.85;
/// The stem's share of the head's radius under a cap.
const STEM: f32 = 0.55;
/// The deepest the water under a cap reaches, as a share of the column's height.
const UNDERCUT_DEPTH: f32 = 0.6;
/// How far the outline wanders, in blocks, so no head is a compass circle.
const OUTLINE_WANDER: f32 = 1.3;
/// The sand ring around a head where its fringe lies.
const FRINGE: f32 = 1.8;
/// A glowpod on the floor under a cap with room for it.
const LANTERN_PER_MILLE: u64 = 230;
/// A glowpod on a head's top where its species grows nothing there.
const TOP_LANTERN_PER_MILLE: u64 = 50;
/// Cells of a head's side crusted in its colours where the surface is bare rock.
const MOTTLE_PER_MILLE: u64 = 600;
/// Lumps on a head's top, in blocks either way.
const LUMP: f32 = 0.9;
/// The fringe, per mille of its cells, by kind.
const FRINGE_LANTERN: u64 = 45;
const FRINGE_RUBBLE: u64 = 60;
const FRINGE_FAN: u64 = 80;
const FRINGE_ANEMONE: u64 = 35;
const FRINGE_BRAIN: u64 = 30;
const FRINGE_TUBE: u64 = 35;
/// A theme's accent from across the colour wheel, one head in this many.
const CONTRAST_IN: i32 = 4;

/// How densely heads stand on a bed, and how big they grow.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ReefField {
    /// At most one head per lattice cell this wide.
    pub lattice: i32,
    /// Lattice cells that hold a head, per mille.
    pub per_mille: u64,
    pub radius: (i32, i32),
}

/// One form of one coral colour: a block row's `exploration:coral` data.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CoralRow {
    pub colour: String,
    /// Where the colour sits on the colour wheel, degrees: themes pair neighbours.
    pub hue: f32,
    pub form: Form,
}

#[derive(Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    /// A cube of coral that crusts a head.
    Block,
    /// A soft fan.
    Fan,
    /// Massive brain coral.
    Brain,
    /// A young table coral: a flat plate on its own stalk.
    Table,
    /// A flat plate with no stalk: the rest of a wide table, or a bracket off a rock wall.
    Plate,
    /// A cluster of tubes.
    Tube,
    /// A tall, swaying sea whip.
    Whip,
}

pub const FORMS: usize = 7;
pub const FORM_LIST: [Form; FORMS] = [
    Form::Block,
    Form::Fan,
    Form::Brain,
    Form::Table,
    Form::Plate,
    Form::Tube,
    Form::Whip,
];

/// One coral colour in every form it has rows for, as block ids or as what a plan writes.
#[derive(Clone, Debug, PartialEq)]
pub struct Coral<B = BlockId> {
    pub hue: f32,
    forms: [Option<B>; FORMS],
}

impl<B> Default for Coral<B> {
    fn default() -> Self {
        Coral {
            hue: 0.0,
            forms: std::array::from_fn(|_| None),
        }
    }
}

impl<B: Clone> Coral<B> {
    pub fn form(&self, form: Form) -> Option<B> {
        self.forms[form as usize].clone()
    }

    pub fn has(&self, form: Form) -> bool {
        self.forms[form as usize].is_some()
    }

    pub fn set(&mut self, form: Form, block: Option<B>) {
        self.forms[form as usize] = block;
    }

    /// Gathers rows into colours, ordered around the colour wheel.
    pub fn palette(rows: impl IntoIterator<Item = (B, CoralRow)>) -> Vec<Coral<B>> {
        let mut colours: Vec<(String, Coral<B>)> = Vec::new();
        for (block, row) in rows {
            let at = match colours.iter().position(|(c, _)| *c == row.colour) {
                Some(i) => i,
                None => {
                    colours.push((row.colour.clone(), Coral::default()));
                    colours.len() - 1
                }
            };
            let coral = &mut colours[at].1;
            coral.hue = row.hue;
            coral.set(row.form, Some(block));
        }
        let mut palette: Vec<Coral<B>> = colours.into_iter().map(|(_, c)| c).collect();
        palette.sort_by(|a, b| a.hue.total_cmp(&b.hue));
        palette
    }
}

/// What colonises a stretch of a head's surface, in a palette colour where it has one.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Species {
    /// Massive brain coral: its own block two deep, and down the head's sides.
    Brain(usize),
    /// A crust of coral block, down the head's sides too.
    Crust(usize),
    /// A thicket of white staghorn on bare rock.
    Staghorn,
    /// A garden of fans on bare rock.
    Garden(usize),
    /// Table corals standing in tiers on bare rock.
    Tables(usize),
    /// Tube clusters on bare rock.
    Tubes(usize),
    /// Sea whips on bare rock.
    Whips(usize),
    /// Bubble anemones on bare rock.
    Anemones,
}

impl Species {
    /// The plant (or plate, or tube) on top of a column this species holds, and how often, per
    /// mille.
    pub(super) fn crown(self) -> (Option<Crown>, u64) {
        match self {
            Species::Brain(_) => (None, 0),
            Species::Crust(c) => (Some(Crown::Fan(c)), 140),
            Species::Staghorn => (Some(Crown::Staghorn), 820),
            Species::Garden(c) => (Some(Crown::Fan(c)), 760),
            Species::Tables(c) => (Some(Crown::Table(c)), 700),
            Species::Tubes(c) => (Some(Crown::Tube(c)), 640),
            Species::Whips(c) => (Some(Crown::Whip(c)), 560),
            Species::Anemones => (Some(Crown::Anemone), 560),
        }
    }

    /// The colour this species shows, if it has one.
    #[cfg(test)]
    fn colour(self) -> Option<usize> {
        match self {
            Species::Brain(c)
            | Species::Crust(c)
            | Species::Garden(c)
            | Species::Tables(c)
            | Species::Tubes(c)
            | Species::Whips(c) => Some(c),
            Species::Staghorn | Species::Anemones => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Crown {
    Staghorn,
    Fan(usize),
    Table(usize),
    Tube(usize),
    Whip(usize),
    Anemone,
    Lantern,
}

/// One column of a head, in cells above the floor.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Column {
    /// The lowest filled cell: 1 stands on the floor, higher hangs from a cap.
    pub base: i32,
    pub top: i32,
    /// On the head's edge, where its sides show.
    pub rim: bool,
    pub species: Species,
    pub crown: Option<Crown>,
    /// A glowpod on the floor beneath the cap.
    pub lantern: bool,
    /// The head's crust colours, crusting the bare-rock side cells set in `mottle` (bit per cell
    /// up): the first, or where `tone` has the cell's bit, the second.
    pub accents: [Option<usize>; 2],
    pub mottle: u32,
    pub tone: u32,
}

impl Column {
    pub fn height(self) -> i32 {
        self.top + i32::from(self.crown.is_some())
    }
}

/// One cell on the sand at a head's foot.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Fringe {
    Lantern,
    Rubble,
    Fan(usize),
    Anemone,
    /// A small brain mound.
    Brain(usize),
    Tube(usize),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Reef {
    Head(Column),
    Fringe(Fringe),
}

/// A head's own roll.
struct Head {
    center: [i32; 2],
    radius: i32,
    height: i32,
    undercut: bool,
    colonies: [Species; 4],
    /// The theme's colours: dominant, accent, third.
    colours: [usize; 3],
    colonies_salt: u32,
}

impl Head {
    fn at(seed: u32, cell: [i32; 2], field: &ReefField, palette: &[Coral]) -> Option<Head> {
        let mut rng = GenRng::positional(seed, SALT_HEAD, cell[0], 0, cell[1]);
        if rng.next_u64() % 1000 >= field.per_mille {
            return None;
        }
        let margin = (field.lattice / 5).max(1);
        let center =
            cell.map(|c| c * field.lattice + rng.next_i32(margin, field.lattice - 1 - margin));
        let radius = rng.next_i32(field.radius.0, field.radius.1);
        let height = (radius + rng.next_i32(-1, 2)).clamp(2, 9);
        let undercut = radius >= UNDERCUT_FROM && rng.next_i32(0, 2) != 0;
        let colours = theme_colours(&mut rng, palette.len());
        Some(Head {
            center,
            radius,
            height,
            undercut,
            colonies: colonies(&mut rng, palette, colours),
            colours,
            colonies_salt: rng.next_u64() as u32,
        })
    }

    /// The colours the head's bare rock is crusted with: the first two of its colours with a
    /// crust.
    fn accents(&self, palette: &[Coral]) -> [Option<usize>; 2] {
        let mut crusts = self
            .colours
            .into_iter()
            .filter(|&c| palette.get(c).is_some_and(|p| p.has(Form::Block)));
        let first = crusts.next();
        [first, crusts.next().or(first)]
    }
}

/// A theme: a dominant colour, an accent beside it on the wheel (or, one head in
/// [`CONTRAST_IN`], across it), and a third on the dominant's other side.
pub(super) fn theme_colours(rng: &mut GenRng, n: usize) -> [usize; 3] {
    let n = n.max(1) as i32;
    let dominant = rng.next_i32(0, n - 1);
    let side = if rng.next_i32(0, 1) == 0 { 1 } else { -1 };
    let accent = if rng.next_i32(0, CONTRAST_IN - 1) == 0 {
        dominant + n / 2
    } else {
        dominant + side
    };
    let third = dominant - side;
    [dominant, accent, third].map(|c| c.rem_euclid(n) as usize)
}

/// Four colonies in the theme: a framework, a showy one and two more, each the first of its
/// choices the palette has the form for.
pub(super) fn colonies<B: Clone>(
    rng: &mut GenRng,
    palette: &[Coral<B>],
    [d, a, t]: [usize; 3],
) -> [Species; 4] {
    let has = |s: &Species| match *s {
        Species::Brain(c) => palette.get(c).is_some_and(|p| p.has(Form::Brain)),
        Species::Crust(c) => palette.get(c).is_some_and(|p| p.has(Form::Block)),
        Species::Garden(c) => palette.get(c).is_some_and(|p| p.has(Form::Fan)),
        Species::Tables(c) => palette.get(c).is_some_and(|p| p.has(Form::Table)),
        Species::Tubes(c) => palette.get(c).is_some_and(|p| p.has(Form::Tube)),
        Species::Whips(c) => palette.get(c).is_some_and(|p| p.has(Form::Whip)),
        Species::Staghorn | Species::Anemones => true,
    };
    let mut pick = |choices: &[Species]| {
        let start = rng.next_i32(0, choices.len() as i32 - 1) as usize;
        (0..choices.len())
            .map(|i| choices[(start + i) % choices.len()])
            .find(|s| has(s))
            .unwrap_or(Species::Staghorn)
    };
    [
        pick(&[Species::Crust(d), Species::Crust(d), Species::Brain(d)]),
        pick(&[
            Species::Garden(a),
            Species::Tables(a),
            Species::Tubes(a),
            Species::Staghorn,
        ]),
        pick(&[
            Species::Whips(t),
            Species::Anemones,
            Species::Brain(a),
            Species::Crust(a),
        ]),
        pick(&[
            Species::Tubes(t),
            Species::Garden(d),
            Species::Tables(d),
            Species::Staghorn,
        ]),
    ]
}

/// The head a column falls in or rings, with its distance from the head's centre: the one the
/// column is deepest inside.
fn nearest(
    seed: u32,
    wx: i32,
    wz: i32,
    field: &ReefField,
    palette: &[Coral],
) -> Option<(Head, f32)> {
    let reach = field.radius.1 + FRINGE.ceil() as i32 + OUTLINE_WANDER.ceil() as i32;
    let cell = |v: i32| v.div_euclid(field.lattice.max(1));
    let mut best: Option<(Head, f32)> = None;
    for lz in cell(wz - reach)..=cell(wz + reach) {
        for lx in cell(wx - reach)..=cell(wx + reach) {
            let Some(head) = Head::at(seed, [lx, lz], field, palette) else {
                continue;
            };
            let (dx, dz) = ((wx - head.center[0]) as f32, (wz - head.center[1]) as f32);
            let wander =
                Noise2(seed ^ SALT_OUTLINE).at(wx as f32 * 0.4, wz as f32 * 0.4) * OUTLINE_WANDER;
            let d = (dx * dx + dz * dz).sqrt() + wander;
            let inside = d - head.radius as f32;
            if best
                .as_ref()
                .is_none_or(|(h, bd)| inside < bd - h.radius as f32)
            {
                best = Some((head, d));
            }
        }
    }
    best
}

/// The reef in a column with `room` cells of water above its floor, if any.
pub fn plan(
    seed: u32,
    wx: i32,
    wz: i32,
    room: i32,
    field: &ReefField,
    palette: &[Coral],
) -> Option<Reef> {
    let (head, d) = nearest(seed, wx, wz, field, palette)?;
    let accents = head.accents(palette);
    let r = head.radius as f32;
    let mut rng = GenRng::positional(seed, SALT_CELL, wx, 0, wz);
    let roll = |rng: &mut GenRng, per_mille: u64| rng.next_u64() % 1000 < per_mille;
    if d > r {
        if d > r + FRINGE {
            return None;
        }
        let mut pick = rng.next_u64() % 1000;
        let [d, a, _] = head.colours;
        let has = |c: usize, form: Form| palette.get(c).is_some_and(|p| p.has(form));
        for (per_mille, fringe) in [
            (FRINGE_LANTERN, Some(Fringe::Lantern)),
            (FRINGE_RUBBLE, Some(Fringe::Rubble)),
            (FRINGE_FAN, has(a, Form::Fan).then_some(Fringe::Fan(a))),
            (FRINGE_ANEMONE, Some(Fringe::Anemone)),
            (
                FRINGE_BRAIN,
                has(d, Form::Brain).then_some(Fringe::Brain(d)),
            ),
            (FRINGE_TUBE, has(a, Form::Tube).then_some(Fringe::Tube(a))),
        ] {
            if pick < per_mille {
                return fringe.map(Reef::Fringe);
            }
            pick -= per_mille;
        }
        return None;
    }
    if room < 2 {
        return None;
    }
    let t = (d / r).clamp(0.0, 1.0);
    let slope = smoothstep01(((t - MESA) / (1.0 - MESA)).clamp(0.0, 1.0));
    let lump = Noise2(seed ^ SALT_LUMP).at(wx as f32 * 0.5, wz as f32 * 0.5) * LUMP;
    let top =
        ((head.height as f32 * (1.0 - RIM_FALL * slope) + lump).round() as i32).clamp(1, room - 1);
    let base = if head.undercut && t > STEM {
        let cap = 2 + i32::from(head.height >= 7);
        (top - cap).clamp(1, 1 + (top as f32 * UNDERCUT_DEPTH) as i32)
    } else {
        1
    };
    let patch = Noise2(head.colonies_salt).at(wx as f32 * 0.22, wz as f32 * 0.22);
    let species = head.colonies[match patch {
        p if p < -0.3 => 0,
        p if p < 0.05 => 1,
        p if p < 0.4 => 2,
        _ => 3,
    }];
    let (crown, per_mille) = species.crown();
    let crown = if top >= room {
        None
    } else if roll(&mut rng, per_mille) {
        crown
    } else {
        roll(&mut rng, TOP_LANTERN_PER_MILLE).then_some(Crown::Lantern)
    };
    let lantern = base >= 3 && roll(&mut rng, LANTERN_PER_MILLE);
    let mottle = (0..32).fold(0u32, |bits, i| {
        bits | (u32::from(roll(&mut rng, MOTTLE_PER_MILLE)) << i)
    });
    let tone = rng.next_u64() as u32;
    Some(Reef::Head(Column {
        base,
        top,
        rim: d > r - 1.5,
        species,
        crown,
        lantern,
        accents,
        mottle,
        tone,
    }))
}

/// The blocks a reef is built from besides its palette.
#[derive(Copy, Clone, Debug)]
pub struct ReefBlocks {
    pub rock: BlockId,
    pub staghorn: BlockId,
    pub anemone: BlockId,
    pub lantern: BlockId,
}

fn form(palette: &[Coral], colour: usize, form: Form) -> Option<BlockId> {
    palette.get(colour)?.form(form)
}

/// The block at `dy` cells above the floor in a head column, if it fills that cell.
pub fn head_cell(
    column: Column,
    dy: i32,
    blocks: &ReefBlocks,
    palette: &[Coral],
) -> Option<BlockId> {
    if dy < column.base {
        return (dy == 1 && column.lantern).then_some(blocks.lantern);
    }
    if dy > column.top {
        if dy != column.top + 1 {
            return None;
        }
        return match column.crown? {
            Crown::Staghorn => Some(blocks.staghorn),
            Crown::Fan(c) => form(palette, c, Form::Fan),
            Crown::Table(c) => form(palette, c, Form::Table),
            Crown::Tube(c) => form(palette, c, Form::Tube),
            Crown::Whip(c) => form(palette, c, Form::Whip),
            Crown::Anemone => Some(blocks.anemone),
            Crown::Lantern => Some(blocks.lantern),
        };
    }
    let brain = matches!(column.species, Species::Brain(_));
    let skin = dy == column.top || column.rim || (dy == column.top - 1 && brain);
    let mottled = column.rim && column.mottle & (1 << (dy as u32 % 32)) != 0;
    let grown = match (skin, column.species) {
        (true, Species::Brain(c)) => form(palette, c, Form::Brain),
        (true, Species::Crust(c)) => form(palette, c, Form::Block),
        _ => None,
    };
    let mottle = || {
        let bit = dy as u32 % 32;
        let accent = column.accents[usize::from(column.tone & (1 << bit) != 0)];
        accent
            .filter(|_| mottled && dy < column.top)
            .and_then(|c| form(palette, c, Form::Block))
    };
    Some(grown.or_else(mottle).unwrap_or(blocks.rock))
}

/// The block a fringe cell lays on the floor.
pub fn fringe_cell(fringe: Fringe, blocks: &ReefBlocks, palette: &[Coral]) -> Option<BlockId> {
    match fringe {
        Fringe::Lantern => Some(blocks.lantern),
        Fringe::Rubble => Some(blocks.rock),
        Fringe::Fan(c) => form(palette, c, Form::Fan),
        Fringe::Anemone => Some(blocks.anemone),
        Fringe::Brain(c) => form(palette, c, Form::Brain),
        Fringe::Tube(c) => form(palette, c, Form::Tube),
    }
}

#[cfg(test)]
mod tests;
