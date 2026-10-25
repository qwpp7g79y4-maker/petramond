//! What every site-placed ocean structure shares: its memoized plans and the
//! session's block ids for the registry names those plans write.

use mod_sdk::build::{Derived, Heights, PlanCache, Site, SiteGrid};
use mod_sdk::*;

/// The terrain a site's plan is derived from; tests stand in a seabed of their own.
pub trait Terrain {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights>;
    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>>;
}

/// The world's own terrain, read through the host.
pub struct HostTerrain;

impl Terrain for HostTerrain {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights> {
        Heights::load(min, max)
    }

    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>> {
        mod_sdk::build::surface_biomes(columns)
    }
}

pub struct Sited {
    cache: PlanCache,
    ids: FxHashMap<String, Option<BlockId>>,
}

impl Sited {
    /// `name` keeps this structure's memo keys apart from the pack's others.
    pub fn new(name: &str) -> Sited {
        Sited {
            cache: PlanCache::new(name, 16),
            ids: FxHashMap::default(),
        }
    }

    /// This section's share of every site on `grid`, deriving plans with `derive` as needed.
    pub fn generate(
        &mut self,
        grid: &SiteGrid,
        ctx: &GenCtx,
        derive: impl FnMut(Site) -> Derived,
    ) -> GenOutput {
        let ids = &mut self.ids;
        let mut resolve = move |name: &str| match ids.get(name) {
            Some(&id) => id,
            None => *ids.entry(name.to_string()).or_insert(resolve_block(name)),
        };
        self.cache
            .generate(grid, ctx, derive, &mut resolve)
            .unwrap_or_else(|_| GenOutput::deferred())
    }
}
