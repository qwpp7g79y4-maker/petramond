//! What every structure built on the sea floor shares: a cell it leaves open
//! below the surface is water, never air, and a block short of a full cube
//! below it holds water; a cell under the floor stays as the sea floor left it
//! (wrecks and ruins lie half buried); and the water column over each of its
//! columns is cleared to the surface, so nothing the seabed grew before it
//! stands inside it.

use std::collections::BTreeMap;

use mod_sdk::build::{Form, Heights, Material, Plan};

use crate::keys;

pub struct Sunken<'a> {
    pub plan: Plan,
    pub sea: i32,
    pub floor: &'a Heights,
    /// The highest cell written in each column.
    tops: BTreeMap<[i32; 2], i32>,
}

impl<'a> Sunken<'a> {
    pub fn new(sea: i32, floor: &'a Heights) -> Self {
        Sunken {
            plan: Plan::new(),
            sea,
            floor,
            tops: BTreeMap::new(),
        }
    }

    pub fn put(&mut self, c: [i32; 3], m: &Material) {
        let top = self.tops.entry([c[0], c[2]]).or_insert(c[1]);
        *top = (*top).max(c[1]);
        self.plan.set(c, m);
        // Below the sea a slab, fence or chest stands in water. The floor read here is the one
        // before the sea wears it, so even a block it seems to bury may end up open.
        if c[1] < self.sea && !matches!(m.form, Form::Block | Form::Log) {
            self.plan.hold_fluid(c, keys::WATER);
        }
    }

    /// Open water at `c`, unless it is under the sea floor or above the sea.
    pub fn flood(&mut self, c: [i32; 3]) {
        if self.floor.get(c[0], c[2]).is_some_and(|f| c[1] > f) && c[1] < self.sea {
            self.put(c, &Material::named(keys::WATER));
        }
    }

    /// The top of the sea floor at a column, if read.
    pub fn ground(&self, x: i32, z: i32) -> Option<i32> {
        self.floor.get(x, z)
    }

    /// The cells the structure spans, inclusive.
    pub fn bounds(&self) -> ([i32; 3], [i32; 3]) {
        let mut min = [i32::MAX; 3];
        let mut max = [i32::MIN; 3];
        for (c, _) in self.plan.cells() {
            for a in 0..3 {
                min[a] = min[a].min(c[a]);
                max[a] = max[a].max(c[a]);
            }
        }
        (min, max)
    }

    /// The finished plan, with the water above every column it stands in cleared to the surface.
    pub fn finish(mut self) -> Plan {
        let water = Material::named(keys::WATER);
        for ([x, z], top) in std::mem::take(&mut self.tops) {
            for y in top + 1..self.sea {
                if !self.plan.occupied([x, y, z]) {
                    self.plan.set([x, y, z], &water);
                }
            }
        }
        self.plan
    }
}

/// Cell data naming the loot table a generated chest is stocked from when first opened.
pub fn loot(plan: &mut Plan, at: [i32; 3], table: &str) {
    plan.data(
        at,
        "petramond:loot",
        format!("{{\"table\":\"{table}\"}}").into_bytes(),
    );
}
