//! Blocks a player empties by hand: a row carrying `exploration:take_out`, `{"item": "<item>",
//! "into": "<block>", "sound": "<sound>"}` (`sound` optional), hands `item` to whoever uses it
//! and turns into `into` in place, facing kept. An open clam gives up its pearl and shuts.

use mod_sdk::*;
use serde::Deserialize;

use crate::keys;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTakeOut {
    item: String,
    into: String,
    #[serde(default)]
    sound: Option<String>,
}

struct Row {
    item: String,
    into: BlockId,
    sound: Option<String>,
}

pub struct TakingOut {
    rows: FxHashMap<BlockId, Row>,
}

impl TakingOut {
    pub fn resolve() -> Option<TakingOut> {
        let rows: FxHashMap<BlockId, Row> = blocks_with_data_as::<RawTakeOut>(keys::TAKE_OUT)
            .into_iter()
            .filter_map(|(block, raw)| {
                resolve_item_logged(&raw.item)?;
                let row = Row {
                    into: resolve_block_logged(&raw.into)?,
                    item: raw.item,
                    sound: raw.sound,
                };
                Some((block, row))
            })
            .collect();
        (!rows.is_empty()).then_some(TakingOut { rows })
    }

    /// The server takes the item out; the client claims the same click without writing.
    /// Sneaking with a block in hand places against the block instead.
    pub fn on_interact(&self, world: SideWorld, pos: [i32; 3]) -> Outcome {
        let Some(row) = world.block(pos).and_then(|b| self.rows.get(&b)) else {
            return Outcome::Continue;
        };
        let actor = player_state();
        if actor.sneak && held_item_places_block(actor.held) {
            return Outcome::Continue;
        }
        if world == SideWorld::Server {
            if !swap_block(pos, row.into) {
                return Outcome::Continue;
            }
            give_item(&row.item, 1);
            if let Some(sound) = &row.sound {
                emit_sound(sound, Some(block_center(pos)));
            }
        }
        Outcome::Cancel
    }
}
