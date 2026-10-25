//! The ocean: seafloor beds per sea biome (kelp forests, seagrass meadows,
//! coral reefs, glowpods in the dark deep) and kelp that grows and falls.
//!
//! The sub-biomes themselves are data (`pack/biomes.json` + the
//! `pack/climate_table.json` layer). This module only dresses their floors.
//! Each bed is a row in [`flora::BEDS`] keyed by biome; a biome without a row
//! keeps a bare floor.

use mod_sdk::*;

use crate::keys;

pub mod flora;
pub mod iceberg;
pub mod kelp;
pub mod lighthouse;
pub mod reef;
pub mod ripen;
pub mod ruin;
pub mod sea_cave;
pub mod sea_life;
pub mod shoal;
mod sited;
pub mod sting;
mod sunken;
pub mod swim;
pub mod take_out;
pub mod trench;
pub mod undercut;
pub mod wreck;

/// Resolved ids the ocean code writes and reads.
pub struct Ocean {
    pub water: BlockId,
    /// What a trench's carved floor is re-skinned with.
    pub trench_floor: BlockId,
    pub kelp: BlockId,
    pub seagrass: BlockId,
    /// Reef palette: every coral colour in the forms it has rows for, around the colour wheel.
    pub corals: Vec<reef::Coral>,
    pub glowpods: Vec<BlockId>,
    /// `None` without every reef block: warm seas then grow no coral heads.
    pub reef: Option<reef::ReefBlocks>,
    /// A clam shut, and one open on its pearl.
    pub clams: [BlockId; 2],
    pub beds: Vec<flora::ResolvedBed>,
}

impl Ocean {
    pub fn resolve() -> Option<Ocean> {
        let corals = reef::Coral::palette(blocks_with_data_as::<reef::CoralRow>(keys::CORAL));
        let glowpods = keys::GLOWPODS
            .iter()
            .filter_map(|key| resolve_block_logged(key))
            .collect::<Vec<_>>();
        let beds = flora::BEDS
            .iter()
            .filter_map(|bed| {
                let biome = resolve_surface_biome(bed.biome);
                if biome.is_none() {
                    log(&format!(
                        "exploration: sea biome '{}' is not registered",
                        bed.biome
                    ));
                }
                Some(flora::ResolvedBed { biome: biome?, bed })
            })
            .collect::<Vec<_>>();
        Some(Ocean {
            water: resolve_block_logged(keys::WATER)?,
            trench_floor: resolve_block_logged(keys::GRAVEL)?,
            kelp: resolve_block_logged(keys::KELP)?,
            seagrass: resolve_block_logged(keys::SEAGRASS)?,
            corals,
            glowpods,
            reef: resolve_reef(),
            clams: [
                resolve_block_logged(keys::CLAM)?,
                resolve_block_logged(keys::CLAM_OPEN)?,
            ],
            beds,
        })
    }

    pub fn bed(&self, biome: u8) -> Option<&'static flora::Bed> {
        self.beds.iter().find(|b| b.biome == biome).map(|b| b.bed)
    }

    /// This section's seabed: trenches and cliff notches cut first, then flora planted on the
    /// floor they leave.
    pub fn seabed(&self, ctx: &GenCtx) -> GenOutput {
        let trenched = |biome: u8| self.bed(biome).is_some_and(|b| b.trench);
        let worn = self.worn(ctx);
        let carve = trench::carve(
            ctx,
            self.water,
            self.trench_floor,
            trenched,
            &worn,
            &trench::HostBeyond,
        );
        let mut out = GenOutput::from(carve.writes.clone());
        flora::generate(self, ctx, &carve, &mut out);
        out
    }

    /// The notches the sea wears into this section's submerged cliffs.
    fn worn(&self, ctx: &GenCtx) -> Vec<[i32; 3]> {
        let [ox, oy, oz] = ctx.origin_world();
        let column = |i: i32| [ox + i % 16, oz + i / 16];
        let sea = |[x, z]: [i32; 2]| ctx.biome(x, z).is_some_and(|b| self.bed(b).is_some());
        let top = (0..256)
            .map(column)
            .filter(|&c| sea(c))
            .filter_map(|[x, z]| ctx.surface_y(x, z))
            .max();
        if top.is_none_or(|top| oy > top) {
            return Vec::new();
        }
        let reach = undercut::REACH;
        let min = [ox - reach, oz - reach];
        let max = [ox + 15 + reach, oz + 15 + reach];
        let Some(floors) = mod_sdk::build::Heights::load(min, max) else {
            return Vec::new();
        };
        undercut::cells(ctx.seed(), ctx.sea_level(), &floors, [ox, oz])
            .into_iter()
            .filter(|c| (oy..oy + 16).contains(&c[1]) && sea([c[0], c[2]]))
            .collect()
    }

    pub fn gen_filter(&self) -> GenFeatureFilter {
        flora::GEN_FILTER.in_surface_biomes(self.beds.iter().map(|b| b.biome))
    }
}

fn resolve_reef() -> Option<reef::ReefBlocks> {
    Some(reef::ReefBlocks {
        rock: resolve_block_logged(keys::REEF_ROCK)?,
        staghorn: resolve_block_logged(keys::STAGHORN_CORAL)?,
        anemone: resolve_block_logged(keys::ANEMONE)?,
        lantern: resolve_block_logged(keys::GLOWPOD_AMBER)?,
    })
}
