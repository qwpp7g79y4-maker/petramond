//! Forge: metal is CAST, not assembled.
//!
//! Three pieces, only one has real code.
//!
//! clay.rs - worldgen feature, clay in riverbeds, banks, savanna patches. Data plus one
//! positional field. Only host call: batched biome probe for bank detection.
//!
//! ore.rs - petramond ore, socket-carving gem source, rare veins below cave floor. Purely
//! positional, no host calls.
//!
//! Pottery table - no code. A block row that opens `forge:pottery_table` plus recipe rows
//! naming that station gives you a full crafting bench, engine handles the rest.
//!
//! Recipe `petramond:unlock_on` rows decide when each station shows up.
//!
//! furnace.rs - the forging furnace, a machine you run by hand. Mould in, lever to pour,
//! cast comes out of the basin as an item.
//!
//! Data decides the cast, not code. Mould in the basin carries a `forge:mould` row naming
//! a recipe class; furnace looks up (class, input metal) in the machine-recipe table. No
//! mould means `forge:cast_plate`. Whole metal x mould matrix lives in recipes.json, so
//! another pack adds a mould just by shipping item and processing rows. No `cast_iron_axe`
//! anywhere.

mod anvil;
mod augments;
mod clay;
mod content;
mod furnace;
mod gold;
mod keys;
mod liquid;
mod ore;
mod schema;

use machine_core::Caches;
use mod_sdk::*;

use furnace::ForgingFurnace;

const FURNACE_HOOK: u32 = 1;
const ANVIL_HOOK: u32 = 2;
const ON_BLOCK_BROKEN: u32 = 1;
const ON_BLOCK_BREAK: u32 = 4;
const ON_MOB_DAMAGED: u32 = 5;
const ON_PLAYER_DAMAGE: u32 = 6;
const GEN_CLAY: u32 = 1;
const GEN_ORE: u32 = 2;

#[derive(Default)]
struct Forge {
    furnace: ForgingFurnace,
    anvil: anvil::Anvil,
    caches: Caches,
    clay: clay::Deposits,
    ore: ore::Ore,
    gold: gold::Gold,
}

impl Mod for Forge {
    fn init(&mut self) {
        self.clay.init();
        register_worldgen_feature(WorldgenStage::Underground, GEN_CLAY, clay::GEN_FILTER);
        self.ore.init();
        register_worldgen_feature(WorldgenStage::Underground, GEN_ORE, ore::GEN_FILTER);

        if runtime_side() != RuntimeSide::Server {
            return;
        }
        self.gold = gold::Gold::resolve();

        let furnace_ok = self.furnace.init();
        if !furnace_ok {
            log("forge: the forging furnace rows are missing; casting stays idle");
        }
        let anvil_ok = self.anvil.init();
        if !anvil_ok {
            log("forge: the anvil rows are missing; augments stay idle");
        }
        if !self.gold.is_empty() || anvil_ok {
            register_event_handler(EventKind::BlockBreakPre, 0, ON_BLOCK_BREAK);
        }
        if anvil_ok {
            register_event_handler(EventKind::MobDamaged, 0, ON_MOB_DAMAGED);
            // Last, so it only sees a player hit nothing refused.
            register_event_handler(EventKind::PlayerDamagePre, i32::MAX, ON_PLAYER_DAMAGE);
        }
        if !furnace_ok && !anvil_ok {
            return;
        }
        if furnace_ok {
            register_block_behavior(keys::FURNACE_BEHAVIOR, FURNACE_HOOK);
        }
        if anvil_ok {
            register_block_behavior(keys::ANVIL_BEHAVIOR, ANVIL_HOOK);
        }
        register_event_handler(EventKind::BlockBroken, 0, ON_BLOCK_BROKEN);
    }

    fn handle_event(&mut self, handler_id: u32, payload: &mut EventPayload) -> Outcome {
        match (handler_id, &mut *payload) {
            (
                ON_BLOCK_BREAK,
                EventPayload::BlockBreakPre {
                    block,
                    harvested,
                    actor: EntityRef::Player(player),
                    drops,
                    ..
                },
            ) => {
                let proc = self.gold.on_block_break(*block, *harvested, *player, drops);
                self.anvil
                    .spec()
                    .wear_held(*player, anvil::WearOn::Break, proc.as_deref());
            }
            (
                ON_MOB_DAMAGED,
                EventPayload::MobDamaged {
                    mob_id,
                    source: DamageSource::PlayerAttack { id },
                    killed,
                    ..
                },
            ) => {
                let spec = self.anvil.spec();
                if !*killed {
                    spec.inflict_held(*id, EntityRef::Entity(*mob_id));
                }
                spec.wear_held(*id, anvil::WearOn::Hit, None);
            }
            (
                ON_PLAYER_DAMAGE,
                EventPayload::PlayerDamagePre {
                    player: victim,
                    amount,
                    source: DamageSource::PlayerAttack { id },
                    ..
                },
            ) if *amount > 0 => {
                let spec = self.anvil.spec();
                spec.inflict_held(*id, EntityRef::Player(*victim));
                spec.wear_held(*id, anvil::WearOn::Hit, None);
            }
            (
                ON_BLOCK_BROKEN,
                EventPayload::BlockBroken {
                    pos,
                    block,
                    group,
                    data,
                    ..
                },
            ) => {
                self.furnace.on_broken(*pos, *block, group.as_ref(), data);
                self.anvil.on_broken(*pos, *block, group.as_ref(), data);
            }
            _ => {}
        }
        Outcome::Continue
    }

    fn gui_click(
        &mut self,
        player: PlayerId,
        kind_key: &str,
        widget_id: &str,
        at: Option<ContainerAddress>,
    ) {
        let Some(ContainerAddress::Block(pos)) = at else {
            return;
        };
        if kind_key == keys::FURNACE_GUI && widget_id == keys::WIDGET_LEVER {
            self.furnace.spec().pull_lever(pos, &mut self.caches);
            self.furnace.wake(pos);
        }
        if kind_key == keys::FURNACE_GUI && widget_id == keys::WIDGET_FITTINGS {
            gui_open(player, keys::FITTINGS_GUI, Some(pos));
        }
        if kind_key == keys::FITTINGS_GUI {
            if widget_id == keys::WIDGET_BACK {
                gui_open(player, keys::FURNACE_GUI, Some(pos));
            }
            if let Some(index) = keys::fittings::TABLE
                .iter()
                .position(|fitting| fitting.widget == widget_id)
            {
                if self.furnace.is_present(pos) {
                    self.furnace.spec().fittings.buy(player, pos, index);
                    self.furnace.wake(pos);
                }
            }
        }
        if kind_key == keys::ANVIL_GUI && widget_id == keys::WIDGET_AUGMENT {
            self.anvil.spec_mut().request_apply(pos);
            self.anvil.wake(pos);
        }
    }

    fn block_hooks(&mut self, callback_id: u32, hooks: &[(BlockHookKind, [i32; 3])]) {
        match callback_id {
            FURNACE_HOOK => self.furnace.on_hooks(&mut self.caches, hooks),
            ANVIL_HOOK => self.anvil.on_hooks(&mut self.caches, hooks),
            _ => {}
        }
    }

    fn gen_feature(&mut self, feature_id: u32, ctx: &GenCtx) -> GenOutput {
        (match feature_id {
            GEN_CLAY => self.clay.generate(ctx),
            GEN_ORE => self.ore.generate(ctx),
            _ => Vec::new(),
        })
        .into()
    }
}

register_mod!(Forge);
