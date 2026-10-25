//! Worn equipment: body slots (head, chest, legs, feet) that hold one item each,
//! and only an item whose row says it is worn there. The click, shift and drag
//! rules live here so the server's menu and the client's prediction run the
//! same code.

use super::Inventory;
use crate::item::ItemStack;

crate::wire_enum::wire_enum! {
    #[derive(Hash)]
    pub enum WearSlot: u8 {
        Head = 0,
        Chest = 1,
        Legs = 2,
        Feet = 3,
    }
    default Head
}

pub const WEAR_SLOTS: usize = 4;

impl WearSlot {
    pub const ALL: [WearSlot; WEAR_SLOTS] = [
        WearSlot::Head,
        WearSlot::Chest,
        WearSlot::Legs,
        WearSlot::Feet,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn key(self) -> &'static str {
        match self {
            WearSlot::Head => "head",
            WearSlot::Chest => "chest",
            WearSlot::Legs => "legs",
            WearSlot::Feet => "feet",
        }
    }

    pub fn from_key(key: &str) -> Option<WearSlot> {
        WearSlot::ALL.into_iter().find(|s| s.key() == key)
    }

    /// Whether `stack` may be worn here.
    pub fn admits(self, stack: &ItemStack) -> bool {
        stack.item.wear().is_some_and(|w| w.slot == self)
    }
}

impl Inventory {
    #[inline]
    pub fn worn(&self, slot: WearSlot) -> Option<&ItemStack> {
        self.worn.get(slot.index()).and_then(Option::as_ref)
    }

    pub fn worn_items(&self) -> impl Iterator<Item = &ItemStack> {
        self.worn.iter().flatten()
    }

    #[inline]
    pub fn raw_worn(&self) -> &[Option<ItemStack>; WEAR_SLOTS] {
        &self.worn
    }

    pub fn set_worn(&mut self, slot: WearSlot, stack: Option<ItemStack>) {
        if let Some(cell) = self.worn.get_mut(slot.index()) {
            *cell = stack;
            self.bump_revision();
        }
    }

    pub fn take_worn(&mut self, slot: WearSlot) -> Option<ItemStack> {
        let taken = self.worn.get_mut(slot.index())?.take();
        if taken.is_some() {
            self.bump_revision();
        }
        taken
    }

    /// A click on a worn slot: put on one of the cursor's items if that slot takes it, swap a
    /// lone cursor item with what is worn, or take what is worn onto an empty cursor.
    pub fn click_worn(&mut self, slot: WearSlot) {
        let i = slot.index();
        match (self.cursor.as_mut(), self.worn[i].is_some()) {
            (None, true) => {
                self.cursor = self.worn[i].take();
                self.bump_revision();
            }
            (Some(held), false) if slot.admits(held) => {
                self.worn[i] = Some(held.restack(1));
                held.count -= 1;
                if held.count == 0 {
                    self.cursor = None;
                }
                self.bump_revision();
            }
            (Some(held), true) if held.count == 1 && slot.admits(held) => {
                std::mem::swap(&mut self.cursor, &mut self.worn[i]);
                self.bump_revision();
            }
            _ => {}
        }
    }

    /// Shift-click on a worn slot: back into the inventory if it fits.
    pub fn shift_unwear(&mut self, slot: WearSlot) {
        let Some(stack) = self.take_worn(slot) else {
            return;
        };
        if let Some(rest) = self.add(stack) {
            self.worn[slot.index()] = Some(rest);
        }
    }

    /// Puts one of inventory slot `i`'s items on, if it is worn somewhere still free.
    pub(super) fn wear_from_slot(&mut self, i: usize) -> bool {
        let Some(stack) = self.slots.get(i).copied().flatten() else {
            return false;
        };
        let Some(slot) = stack.item.wear().map(|w| w.slot) else {
            return false;
        };
        if self.worn(slot).is_some() {
            return false;
        }
        self.worn[slot.index()] = Some(stack.restack(1));
        self.slots[i] = (stack.count > 1).then(|| stack.restack(stack.count - 1));
        self.bump_revision();
        true
    }

    /// How many of `held` a drag may put in a worn slot.
    pub fn worn_capacity(&self, slot: WearSlot, held: &ItemStack) -> u8 {
        u8::from(self.worn(slot).is_none() && slot.admits(held))
    }

    /// A drag placing `wanted` (at most one) of the cursor into a worn slot.
    pub fn place_cursor_in_worn(&mut self, slot: WearSlot, wanted: u8) {
        if wanted == 0 || self.worn(slot).is_some() {
            return;
        }
        let Some(held) = self.cursor.as_mut().filter(|h| slot.admits(h)) else {
            return;
        };
        self.worn[slot.index()] = Some(held.restack(1));
        held.count -= 1;
        if held.count == 0 {
            self.cursor = None;
        }
        self.bump_revision();
    }
}

#[cfg(test)]
mod tests;
