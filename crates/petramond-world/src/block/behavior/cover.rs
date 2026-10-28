//! Ground covers (grass on dirt). A cover row carries `petramond:cover {bare, spread}`: behavior
//! `cover` turns it back into `bare` once smothered or submerged, and behavior `bare` on the bare
//! row takes the first cover standing within that cover's `spread`, unless it is smothered or
//! submerged itself.

use serde::Deserialize;

use super::{BehaviorWorld, BlockBehavior};
use crate::block::{Block, BlockTag};
use crate::mathh::IVec3;
use crate::world::data::WorldData;

pub const COVER_KEY: &str = "petramond:cover";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCover {
    bare: String,
    spread: i32,
}

#[derive(Default)]
struct Covers {
    /// Per block id: the bare block a cover turns back into.
    bare_of: Vec<Option<Block>>,
    /// Per bare block id: the covers that take it, with how far each spreads.
    onto: Vec<Vec<(Block, i32)>>,
}

static COVERS: crate::content::Slot<Covers> =
    crate::content::Slot::new("ground covers", &[crate::content::stage::BLOCKS], |_| {
        Ok(covers())
    });

fn covers() -> Covers {
    let all = Block::all();
    let mut table = Covers {
        bare_of: vec![None; all.len()],
        onto: vec![Vec::new(); all.len()],
    };
    for &cover in all {
        let Some(text) = cover.data_value(COVER_KEY) else {
            continue;
        };
        let raw: RawCover = match serde_json::from_str(text) {
            Ok(raw) => raw,
            Err(e) => {
                log::warn!("{COVER_KEY} on {cover:?}: {e}");
                continue;
            }
        };
        let Some(bare) = crate::registry::names().blocks.id(&raw.bare).map(Block) else {
            log::warn!(
                "{COVER_KEY} on {cover:?}: unknown bare block '{}'",
                raw.bare
            );
            continue;
        };
        table.bare_of[cover.id() as usize] = Some(bare);
        table.onto[bare.id() as usize].push((cover, raw.spread.max(0)));
    }
    table
}

/// The bare block a ground cover turns back into, for a cover row.
pub fn cover_bare(block: Block) -> Option<Block> {
    COVERS
        .current()
        .bare_of
        .get(block.id() as usize)
        .copied()
        .flatten()
}

pub struct Cover;

impl BlockBehavior for Cover {
    fn key(&self) -> &'static str {
        "cover"
    }

    fn has_random_tick(&self) -> bool {
        true
    }

    fn random_tick(&self, world: &mut dyn BehaviorWorld, pos: IVec3) {
        let Some(cover) = world.data().block_if_loaded(pos.x, pos.y, pos.z) else {
            return;
        };
        if let Some(bare) = cover_bare(cover) {
            if smothered(world.data(), pos) || submerged(world.data(), pos) {
                world.set_block_world(pos.x, pos.y, pos.z, bare);
            }
        }
    }
}

pub static COVER: Cover = Cover;

pub struct Bare;

impl BlockBehavior for Bare {
    fn key(&self) -> &'static str {
        "bare"
    }

    fn has_random_tick(&self) -> bool {
        true
    }

    fn random_tick(&self, world: &mut dyn BehaviorWorld, pos: IVec3) {
        let Some(bare) = world.data().block_if_loaded(pos.x, pos.y, pos.z) else {
            return;
        };
        if smothered(world.data(), pos) || submerged(world.data(), pos) {
            return;
        }
        let covers = COVERS.current().onto.get(bare.id() as usize);
        if let Some(cover) = covers.and_then(|c| cover_within(world.data(), pos, c)) {
            world.set_block_world(pos.x, pos.y, pos.z, cover);
        }
    }
}

pub static BARE: Bare = Bare;

/// The first of `covers` standing within its own spread of `center`.
fn cover_within(world: &WorldData, center: IVec3, covers: &[(Block, i32)]) -> Option<Block> {
    let radius = covers.iter().map(|(_, r)| *r).max()?;
    for dy in -radius..=radius {
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }
                let p = center + IVec3::new(dx, dy, dz);
                let Some(found) = world.block_if_loaded(p.x, p.y, p.z) else {
                    continue;
                };
                let reach = dx.abs().max(dy.abs()).max(dz.abs());
                if let Some(&(cover, _)) = covers.iter().find(|(c, r)| *c == found && reach <= *r) {
                    return Some(cover);
                }
            }
        }
    }
    None
}

fn smothered(world: &WorldData, pos: IVec3) -> bool {
    world
        .block_if_loaded(pos.x, pos.y + 1, pos.z)
        .is_some_and(|b| b.is_solid() && !b.has_tag(BlockTag::NO_GRASS_DECAY))
}

fn submerged(world: &WorldData, pos: IVec3) -> bool {
    world.block_if_loaded(pos.x, pos.y + 1, pos.z).is_some()
        && world.cell_fluid(pos.x, pos.y + 1, pos.z).is_some()
}

#[cfg(test)]
mod tests;
