#![warn(clippy::too_many_lines)]

use mod_sdk::*;

mod cascade;
mod cavern;
mod content;
mod dripstone;
mod fluids;
mod keys;
mod ocean;
mod probe;
mod shroom;
mod spores;

use content::Content;
use dripstone::Dripstone;

const GEN_CAVERN: u32 = 1;
const GEN_DRIPSTONE: u32 = 2;
const GEN_OCEAN_FLORA: u32 = 3;
const GEN_SHOAL: u32 = 10;
const HOOK_DRIPSTONE: u32 = 1;
const HOOK_KELP: u32 = 2;
const HANDLER_PROJECTILE: u32 = 1;
const HANDLER_PLAYER_DAMAGE: u32 = 2;
const HANDLER_MOB_DAMAGE: u32 = 3;
const HANDLER_PLACED: u32 = 4;

pub(crate) use keys::MUSHROOM_CAVERN as BIOME_KEY;

pub const BIOME_TOP_Y: i32 = 96;

#[derive(Default)]
struct Exploration {
    content: Option<Content>,
    dripstone: Option<Dripstone>,
    ocean: Option<ocean::Ocean>,
    shoals: Option<ocean::shoal::Shoals>,
    spores: spores::Spores,
}

impl Exploration {
    /// The site-placed sea structures, each registered when its content resolves.
    fn init_sea_structures(&mut self) {
        if let Some(shoals) = ocean::shoal::Look::resolve()
            .and_then(|look| ocean::shoal::Shoals::new(ocean::shoal::warm_seas(), look))
        {
            register_worldgen_feature(WorldgenStage::Trees, GEN_SHOAL, ocean::shoal::GEN_FILTER);
            self.shoals = Some(shoals);
        }
    }
}

impl Mod for Exploration {
    fn init(&mut self) {
        if runtime_side() == RuntimeSide::Client {
            self.spores.init();
            return;
        }
        let fluids = fluids::Fluids::resolve();
        match Content::resolve(fluids.clone()) {
            Some(content) => {
                self.content = Some(content);
                register_worldgen_feature(WorldgenStage::Trees, GEN_CAVERN, cavern::gen_filter());
            }
            None => {
                log("exploration: mushroom content failed to resolve; mushroom decoration disabled")
            }
        }
        match Dripstone::resolve(fluids) {
            Some(dripstone) => {
                let filter = dripstone::gen::gen_filter(dripstone.biome);
                self.dripstone = Some(dripstone);
                register_worldgen_feature(WorldgenStage::Trees, GEN_DRIPSTONE, filter);
                register_block_behavior(keys::POINTED_DRIPSTONE_HOOK, HOOK_DRIPSTONE);
                register_event_handler(EventKind::ProjectileHit, 0, HANDLER_PROJECTILE);
                register_event_handler(EventKind::PlayerDamagePre, 0, HANDLER_PLAYER_DAMAGE);
                register_event_handler(EventKind::MobDamagePre, 0, HANDLER_MOB_DAMAGE);
                register_event_handler(EventKind::BlockPlaced, 0, HANDLER_PLACED);
            }
            None => log("exploration: dripstone content failed to resolve; dripstone disabled"),
        }
        match ocean::Ocean::resolve() {
            Some(ocean) => {
                register_worldgen_feature(
                    WorldgenStage::Vegetation,
                    GEN_OCEAN_FLORA,
                    ocean.gen_filter(),
                );
                register_block_behavior(keys::KELP_HOOK, HOOK_KELP);
                self.ocean = Some(ocean);
            }
            None => log("exploration: ocean content failed to resolve; seafloor bare"),
        }
        self.init_sea_structures();
    }

    fn gen_feature(&mut self, feature_id: u32, ctx: &GenCtx) -> GenOutput {
        match feature_id {
            GEN_CAVERN => match self.content.as_ref().map(|c| cavern::generate(c, ctx)) {
                Some(Ok(writes)) => writes.into(),
                Some(Err(probe::Deferred)) => GenOutput::deferred(),
                None => GenOutput::default(),
            },
            GEN_DRIPSTONE => self
                .dripstone
                .as_ref()
                .map_or_else(GenOutput::default, |d| {
                    dripstone::gen::generate(d, ctx).into()
                }),
            GEN_SHOAL => self
                .shoals
                .as_mut()
                .map_or_else(GenOutput::default, |s| s.generate(ctx)),
            GEN_OCEAN_FLORA => self
                .ocean
                .as_ref()
                .map_or_else(GenOutput::default, |o| o.seabed(ctx)),
            _ => GenOutput::default(),
        }
    }

    fn block_hook(&mut self, callback_id: u32, kind: BlockHookKind, pos: [i32; 3]) {
        match (callback_id, &self.dripstone, &self.ocean) {
            (HOOK_DRIPSTONE, Some(d), _) => dripstone::behavior::on_hook(d, kind, pos),
            (HOOK_KELP, _, Some(o)) => ocean::kelp::on_hook(o, kind, pos),
            _ => {}
        }
    }

    fn handle_event(&mut self, handler_id: u32, payload: &mut EventPayload) -> Outcome {
        let Some(d) = &self.dripstone else {
            return Outcome::Continue;
        };
        match handler_id {
            HANDLER_PROJECTILE => dripstone::hazard::on_projectile_hit(payload),
            HANDLER_PLAYER_DAMAGE => dripstone::hazard::on_player_damage(d, payload),
            HANDLER_MOB_DAMAGE => dripstone::hazard::on_mob_damage(d, payload),
            HANDLER_PLACED => dripstone::behavior::on_placed(d, payload),
            _ => Outcome::Continue,
        }
    }

    fn client_frame(&mut self, frame: &ClientFrameData) {
        self.spores.frame(frame);
    }
}

register_mod!(Exploration);
