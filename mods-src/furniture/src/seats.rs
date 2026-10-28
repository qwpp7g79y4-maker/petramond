//! Sitting: pure mod policy over the engine's attach primitive (`attach` at a point, seated).
//! Block row data owns seat layout (points in the block's own footprint space, like model
//! geometry); `block_local_to_world` carries them into the world for the placed facing. Occupancy
//! is read straight from the engine roster (`attached`) — never mirrored mod state, so nothing
//! to desync or clean up. Engine owns the mechanism: one attachment per player, one body per
//! point, replication, seated body, release valves (sneak / death / spectator / leave).
//!
//! A click on furniture is always claimed — seated, or absorbed once every seat is taken.
//! The absorb is deliberate: occupancy is invisible to the initiating client's replica, so
//! pass-when-full would let the client ghost a block placement the server then refuses.
//! Client mirrors the same always-claim as a predictor. One exception: a sneak click while
//! holding a placeable block defers to the placement consumer (sneak-to-build against a
//! chair, same as farming's harvest sneak rule).
//!
//! Breaking furniture is the one release this mod owes the engine (a point isn't tied to any
//! block): `block_broken` carries the cells the piece occupied, and the players attached inside
//! them are released.

use mod_sdk::*;

use super::{keys, Furniture};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Piece {
    pub(super) seats: Vec<[f32; 3]>,
}

pub(super) fn load_pieces() -> Vec<ResolvedPiece> {
    blocks_with_data_as::<Piece>(keys::SEATS)
        .into_iter()
        .filter_map(|(block, piece)| {
            if piece.seats.is_empty() {
                log(&format!("furniture: seat row for {block:?} has no seats"));
                None
            } else {
                Some(ResolvedPiece { block, piece })
            }
        })
        .collect()
}

pub(super) struct ResolvedPiece {
    pub(super) block: BlockId,
    pub(super) piece: Piece,
}

impl Furniture {
    pub(super) fn piece_for(&self, block: BlockId) -> Option<&Piece> {
        self.pieces
            .iter()
            .find(|p| p.block == block)
            .map(|p| &p.piece)
    }

    pub(super) fn try_sit(&self, pos: [i32; 3], player: PlayerId, actor: &PlayerSnapshot) -> bool {
        let Some(piece) = self.seat_gate(pos, actor) else {
            return false;
        };
        let Some(facing) = cell_group(pos).and_then(|group| group.facing) else {
            return false;
        };
        let Some(anchors) = block_local_to_world(pos, piece.seats.clone()) else {
            return false;
        };
        let occupied: Vec<[f64; 3]> = players()
            .into_iter()
            .filter_map(|p| attached_point(&p.state))
            .collect();
        let yaw = facing_player_yaw(facing);
        let (cx, cz) = (pos[0] as f64 + 0.5, pos[2] as f64 + 0.5);
        let mut free: Vec<(f64, [f64; 3])> = anchors
            .into_iter()
            .filter(|anchor| !occupied.contains(anchor))
            .map(|anchor| ((anchor[0] - cx).powi(2) + (anchor[2] - cz).powi(2), anchor))
            .collect();
        free.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (_, anchor) in free {
            if attach(player, AttachTo::At { pos: anchor, yaw }, Posture::Seated) {
                break;
            }
        }
        true
    }

    /// The seat consumer's claim GATE, run by both instances over their own
    /// [`WorldView`]: furniture claims every click (seat or absorb), so the
    /// gate is the block id alone — no occupancy read, so the client's
    /// prediction is exact — apart from the sneak+placeable pass. Answers
    /// the clicked piece; a cell the instance cannot read never claims.
    pub(super) fn seat_gate(&self, pos: [i32; 3], actor: &PlayerSnapshot) -> Option<&Piece> {
        if actor.sneak && held_item_places_block(actor.held) {
            return None;
        }
        get_block(pos).and_then(|b| self.piece_for(b))
    }
}

/// The world point a player is attached at, if any.
fn attached_point(state: &PlayerSnapshot) -> Option<[f64; 3]> {
    match state.attached?.to {
        AttachTo::At { pos, .. } => Some(pos),
        AttachTo::Seat { .. } => None,
    }
}

/// Releases every player attached inside the cells a broken piece occupied.
pub(super) fn release_broken_piece_sitters(pos: [i32; 3], group: Option<&CellGroupData>) {
    let cells = group.map_or_else(|| vec![pos], |g| g.cells.clone());
    for entry in players() {
        let Some(point) = attached_point(&entry.state) else {
            continue;
        };
        let cell = point.map(|v| v.floor() as i32);
        if cells.contains(&cell) {
            detach(entry.id);
        }
    }
}

#[cfg(test)]
mod row_tests {
    use super::*;

    #[test]
    fn shipped_seat_rows_parse() {
        let rows = pack_rows_with_data(include_str!("../pack/blocks.json"), "blocks", keys::SEATS);
        for (block, raw) in rows {
            let piece: Piece = parse_row_data(&raw).unwrap_or_else(|e| panic!("{block}: {e}"));
            assert!(!piece.seats.is_empty(), "{block}");
            assert!(
                piece.seats.iter().flatten().all(|v| v.is_finite()),
                "{block}"
            );
        }
    }
}
