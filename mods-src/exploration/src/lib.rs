#![warn(clippy::too_many_lines)]

use mod_sdk::*;

mod cascade;
mod cavern;
mod content;
mod dripstone;
mod fishing;
mod fluids;
mod keys;
mod ocean;
mod probe;
mod shroom;
mod spores;
mod worn_emitters;

use content::Content;
use dripstone::Dripstone;

const GEN_CAVERN: u32 = 1;
const GEN_DRIPSTONE: u32 = 2;
const GEN_OCEAN_FLORA: u32 = 3;
const GEN_ICEBERG: u32 = 4;
const GEN_SEA_CAVE: u32 = 5;
const GEN_WRECK: u32 = 7;
const GEN_RUIN: u32 = 8;
const GEN_LIGHTHOUSE: u32 = 9;
const GEN_SHOAL: u32 = 10;
const HOOK_DRIPSTONE: u32 = 1;
const HOOK_KELP: u32 = 2;
const HOOK_RIPEN: u32 = 3;
const TICK_SEA_LIFE: u32 = 1;
const TICK_FISHING: u32 = 2;
const TICK_WORN_EMITTERS: u32 = 3;
const HANDLER_PROJECTILE: u32 = 1;
const HANDLER_PLAYER_DAMAGE: u32 = 2;
const HANDLER_MOB_DAMAGE: u32 = 3;
const HANDLER_PLACED: u32 = 4;
const HANDLER_USE: u32 = 5;
const HANDLER_INTERACT: u32 = 6;

pub(crate) use keys::MUSHROOM_CAVERN as BIOME_KEY;

pub const BIOME_TOP_Y: i32 = 96;

#[derive(Default)]
struct Exploration {
    content: Option<Content>,
    dripstone: Option<Dripstone>,
    ocean: Option<ocean::Ocean>,
    icebergs: Option<ocean::iceberg::Icebergs>,
    sea_caves: Option<ocean::sea_cave::SeaCaves>,
    wrecks: Option<ocean::wreck::Wrecks>,
    shoals: Option<ocean::shoal::Shoals>,
    ruins: Option<ocean::ruin::Ruins>,
    lighthouses: Option<ocean::lighthouse::Lighthouses>,
    sea_life: Option<ocean::sea_life::SeaLife>,
    ripening: Option<ocean::ripen::Ripening>,
    taking_out: Option<ocean::take_out::TakingOut>,
    fishing: Option<fishing::Fishing>,
    worn_emitters: Option<worn_emitters::WornEmitters>,
    spores: spores::Spores,
}

impl Exploration {
    /// The site-placed sea structures, each registered when its content resolves.
    fn init_sea_structures(&mut self) {
        if let Some(icebergs) = ocean::iceberg::Icebergs::new(ocean::iceberg::frozen_seas()) {
            register_worldgen_feature(
                WorldgenStage::Trees,
                GEN_ICEBERG,
                ocean::iceberg::GEN_FILTER,
            );
            self.icebergs = Some(icebergs);
        }
        if let Some(wrecks) = ocean::wreck::Wrecks::new() {
            register_worldgen_feature(WorldgenStage::Trees, GEN_WRECK, ocean::wreck::GEN_FILTER);
            self.wrecks = Some(wrecks);
        }
        if let Some(shoals) = ocean::shoal::Look::resolve()
            .and_then(|look| ocean::shoal::Shoals::new(ocean::shoal::warm_seas(), look))
        {
            register_worldgen_feature(WorldgenStage::Trees, GEN_SHOAL, ocean::shoal::GEN_FILTER);
            self.shoals = Some(shoals);
        }
        if let Some(ruins) = ocean::ruin::Ruins::new() {
            register_worldgen_feature(WorldgenStage::Trees, GEN_RUIN, ocean::ruin::GEN_FILTER);
            self.ruins = Some(ruins);
        }
        if let Some(lights) = ocean::lighthouse::Lighthouses::new() {
            register_worldgen_feature(
                WorldgenStage::Trees,
                GEN_LIGHTHOUSE,
                ocean::lighthouse::GEN_FILTER,
            );
            self.lighthouses = Some(lights);
        }
        if let Some(caves) = ocean::sea_cave::SeaCaves::new() {
            register_worldgen_feature(
                WorldgenStage::Trees,
                GEN_SEA_CAVE,
                ocean::sea_cave::GEN_FILTER,
            );
            self.sea_caves = Some(caves);
        }
    }
}

impl Mod for Exploration {
    fn init(&mut self) {
        self.taking_out = ocean::take_out::TakingOut::resolve();
        if self.taking_out.is_some() {
            register_event_handler(EventKind::InteractAttempt, 0, HANDLER_INTERACT);
        }
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
                self.ripening = ocean::ripen::Ripening::resolve();
                if self.ripening.is_some() {
                    register_block_behavior(keys::RIPENS, HOOK_RIPEN);
                }
                self.sea_life = ocean::sea_life::SeaLife::resolve(ocean.water);
                if self.sea_life.is_some() {
                    register_tick_system(Stage::Mobs, AttachSide::Before, 0, TICK_SEA_LIFE);
                }
                self.ocean = Some(ocean);
            }
            None => log("exploration: ocean content failed to resolve; seafloor bare"),
        }
        self.worn_emitters = worn_emitters::WornEmitters::resolve();
        if self.worn_emitters.is_some() {
            register_tick_system(Stage::Mobs, AttachSide::After, 0, TICK_WORN_EMITTERS);
        }
        self.fishing = resolve_block_logged(keys::WATER).and_then(fishing::Fishing::resolve);
        if self.fishing.is_some() {
            register_event_handler(EventKind::UseUnclaimed, 0, HANDLER_USE);
            register_tick_system(Stage::ItemPhysics, AttachSide::Before, 0, TICK_FISHING);
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
            GEN_WRECK => self
                .wrecks
                .as_mut()
                .map_or_else(GenOutput::default, |w| w.generate(ctx)),
            GEN_SHOAL => self
                .shoals
                .as_mut()
                .map_or_else(GenOutput::default, |s| s.generate(ctx)),
            GEN_RUIN => self
                .ruins
                .as_mut()
                .map_or_else(GenOutput::default, |r| r.generate(ctx)),
            GEN_LIGHTHOUSE => self
                .lighthouses
                .as_mut()
                .map_or_else(GenOutput::default, |l| l.generate(ctx)),
            GEN_SEA_CAVE => self
                .sea_caves
                .as_mut()
                .map_or_else(GenOutput::default, |c| c.generate(ctx)),
            GEN_ICEBERG => self
                .icebergs
                .as_mut()
                .map_or_else(GenOutput::default, |i| i.generate(ctx)),
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
            (HOOK_RIPEN, _, _) => {
                if let Some(ripening) = &self.ripening {
                    ripening.on_hook(kind, pos);
                }
            }
            _ => {}
        }
    }

    fn handle_event(&mut self, handler_id: u32, payload: &mut EventPayload) -> Outcome {
        if let (
            HANDLER_INTERACT,
            Some(taking_out),
            EventPayload::InteractAttempt {
                block: Some(pos), ..
            },
        ) = (handler_id, &self.taking_out, &*payload)
        {
            return taking_out.on_interact(SideWorld::current(), *pos);
        }
        if let (HANDLER_USE, Some(fishing), EventPayload::UseUnclaimed { player, .. }) =
            (handler_id, &mut self.fishing, &*payload)
        {
            return fishing.on_use(*player);
        }
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

    fn tick_system(&mut self, system_id: u32) {
        match system_id {
            TICK_SEA_LIFE => {
                if let Some(life) = &mut self.sea_life {
                    life.tick();
                }
            }
            TICK_FISHING => {
                if let Some(fishing) = &mut self.fishing {
                    fishing.tick();
                }
            }
            TICK_WORN_EMITTERS => {
                if let Some(worn) = &mut self.worn_emitters {
                    worn.tick();
                }
            }
            _ => {}
        }
    }

    fn client_frame(&mut self, frame: &ClientFrameData) {
        self.spores.frame(frame);
    }
}

register_mod!(Exploration);
