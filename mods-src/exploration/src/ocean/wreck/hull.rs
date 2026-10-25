//! A wreck's hull in the ship's own frame: `x` across the beam (the keel runs
//! down the middle column), `z` along the length from the transom (0) to the
//! stem, `y` up from the keel. Every cell is shell, hold, deck, rail or
//! outside the ship.

pub const BEAM: i32 = 7;
pub const MID: i32 = BEAM / 2;
/// The deck's height above the keel.
pub const DECK: i32 = 3;
/// Cells of the length, back from the stem, under the raised forecastle deck.
pub const FORECASTLE: i32 = 4;
/// The height of the strake that runs the wale, a darker band along each side.
pub const WALE: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Shell,
    /// The side planking at [`WALE`] height.
    Wale,
    Hold,
    Deck,
    Rail,
}

#[derive(Clone, Copy, Debug)]
pub struct Hull {
    pub len: i32,
}

impl Hull {
    /// Half the hull's width at deck height, `lz` cells from the transom: full
    /// amidships, a squared-off transom aft, drawn in to a point at the stem.
    pub fn half_width(&self, lz: i32) -> i32 {
        if !(0..self.len).contains(&lz) {
            return -1;
        }
        let aft = if lz == 0 { MID - 1 } else { MID };
        let fore = match self.len - 1 - lz {
            0 => 0,
            1 => 1,
            2 | 3 => 2,
            _ => MID,
        };
        aft.min(fore)
    }

    /// The hull's half width at height `y`: the bilge rounds in below the waterline.
    fn side_at(&self, lz: i32, y: i32) -> i32 {
        let hw = self.half_width(lz);
        if y <= 1 {
            (hw - 1).max(0)
        } else {
            hw
        }
    }

    /// The deck's height at `lz`: one higher under the forecastle.
    pub fn deck_at(&self, lz: i32) -> i32 {
        if lz >= self.len - FORECASTLE {
            DECK + 1
        } else {
            DECK
        }
    }

    pub fn part(&self, lx: i32, y: i32, lz: i32) -> Option<Part> {
        let hw = self.half_width(lz);
        let off = (lx - MID).abs();
        let deck = self.deck_at(lz);
        if hw < 0 || off > hw || !(0..=deck + 1).contains(&y) {
            return None;
        }
        let at_ends = lz == 0 || lz == self.len - 1;
        let break_wall = lz == self.len - FORECASTLE && y == DECK + 1;
        match y {
            0 => (off <= self.side_at(lz, 1)).then_some(Part::Shell),
            y if y == deck => Some(Part::Deck),
            y if y == deck + 1 => (off == hw || lz == 0).then_some(Part::Rail),
            y => {
                let side = self.side_at(lz, y);
                if off > side {
                    None
                } else if off == side && y == WALE {
                    Some(Part::Wale)
                } else if off == side || at_ends || break_wall {
                    Some(Part::Shell)
                } else {
                    Some(Part::Hold)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hold_is_closed_by_shell_below_and_to_every_side_and_by_deck_above() {
        for len in [15, 18, 21] {
            let hull = Hull { len };
            for lz in 0..len {
                for y in 0..=DECK + 1 {
                    for lx in 0..BEAM {
                        if hull.part(lx, y, lz) != Some(Part::Hold) {
                            continue;
                        }
                        for [dx, dy, dz] in [
                            [1, 0, 0],
                            [-1, 0, 0],
                            [0, -1, 0],
                            [0, 1, 0],
                            [0, 0, 1],
                            [0, 0, -1],
                        ] {
                            let n = hull.part(lx + dx, y + dy, lz + dz);
                            assert!(
                                matches!(
                                    n,
                                    Some(Part::Hold | Part::Shell | Part::Wale | Part::Deck)
                                ),
                                "len {len}: hold at {:?} opens onto {n:?} toward {:?}",
                                [lx, y, lz],
                                [dx, dy, dz]
                            );
                        }
                    }
                }
            }
        }
    }
}
