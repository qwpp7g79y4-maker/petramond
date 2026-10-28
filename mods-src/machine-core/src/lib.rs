//! Shared machine plumbing for container-slot mod machines: the generic placed-machine driver
//! ([`Machine`] over a [`MachineSpec`]) and the session-stable registry caches (item info,
//! per-class recipe results) machine steps read.
//!
//! A machine KIND is a [`MachineSpec`] impl: its block/variant/state keys plus one `step`. Its
//! rows list the kind's block behavior, and the engine wakes a machine through it: its container
//! changing, a player opening or closing its GUI, a block update, or the scheduled tick the driver
//! asks for after every step that changed something. A step that changes nothing leaves the
//! machine at rest until one of those wakes it, so an idle machine costs nothing. The kitchen's
//! oven and miller and the forge's forging furnace and anvil are all specs over this driver, and
//! every fuelled one keeps its fire in a [`Burner`].
//!
//! # Cost shape
//!
//! The engine hands the driver all of one kind's hooks for a tick in one call, and the driver
//! answers them with a fixed number of host crossings, not one per machine: anchors resolve,
//! read, step and write back through the batched calls (`cell_groups`, `get_blocks`,
//! `container_get_many`, `section_kv_get_many` / `_set_many`, `set_model_parts_many`,
//! `set_block_draws`, `schedule_ticks`). Every one of those is PAGED at the host's batch cap, so
//! more busy machines than the cap keep working instead of erroring the call, which disables the
//! mod.

use std::collections::{HashMap, HashSet};

use mod_sdk::*;

mod burner;
pub use burner::Burner;

pub trait MachineSpec: Default {
    const BLOCK_KEY: &'static str;
    const VARIANT_KEYS: &'static [&'static str];
    const STATE_KEY: &'static str;
    const AUX_KEYS: &'static [&'static str] = &[];

    fn init(&mut self) {}

    /// One machine's step. `slots` is the anchor's container (empty while none exists); the
    /// driver writes back the slots the step changed. `state` is this cell's
    /// [`STATE_KEY`](Self::STATE_KEY) blob, written back only if the step changed it (empty =
    /// nothing stored).
    ///
    /// Presentation goes into `out` rather than out through a host call, so the whole kind
    /// submits in one crossing.
    fn step(
        &mut self,
        ctx: &StepCtx<'_>,
        caches: &mut Caches,
        slots: &mut Vec<Option<ItemStackData>>,
        state: &mut Vec<u8>,
        out: &mut Presentation,
    );

    /// The machine at `anchor` was broken or replaced. `data` is the cell data it held, read
    /// before the engine cleared it.
    fn removed(&mut self, _anchor: [i32; 3], _data: &[(String, Vec<u8>)]) {}
}

pub struct StepCtx<'a> {
    pub pos: [i32; 3],
    pub current: BlockId,
    pub block: BlockId,
    variants: &'a [Option<BlockId>],
    pub aux: &'a [Vec<u8>],
    pub viewers: &'a [PlayerId],
}

impl StepCtx<'_> {
    pub fn gui_open(&self) -> bool {
        !self.viewers.is_empty()
    }

    pub fn publish(&self, key: &str, value: GuiValue) {
        gui_state_set(ContainerAddress::Block(self.pos), key, value);
    }

    pub fn variant(&self, i: usize) -> Option<BlockId> {
        self.variants.get(i).copied().flatten()
    }

    pub fn variant_or_base(&self, i: usize) -> BlockId {
        self.variant(i).unwrap_or(self.block)
    }
}

pub struct Machine<S: MachineSpec> {
    block: Option<BlockId>,
    variants: Vec<Option<BlockId>>,
    spec: S,
}

impl<S: MachineSpec> Default for Machine<S> {
    fn default() -> Self {
        Machine {
            block: None,
            variants: Vec::new(),
            spec: S::default(),
        }
    }
}

impl<S: MachineSpec> Machine<S> {
    pub fn init(&mut self) -> bool {
        self.block = resolve_block_logged(S::BLOCK_KEY);
        if self.block.is_none() {
            return false;
        }
        self.variants = S::VARIANT_KEYS
            .iter()
            .map(|key| resolve_block_logged(key))
            .collect();
        self.spec.init();
        true
    }

    fn owns(&self, block: BlockId) -> bool {
        Some(block) == self.block || self.variants.contains(&Some(block))
    }

    pub fn is_present(&self, pos: [i32; 3]) -> bool {
        get_block(pos).is_some_and(|b| self.owns(b))
    }

    pub fn spec(&self) -> &S {
        &self.spec
    }

    pub fn spec_mut(&mut self) -> &mut S {
        &mut self.spec
    }

    /// Steps the machine at `anchor` next tick: something it works on changed outside its
    /// container (a GUI button, a fitting bought).
    pub fn wake(&self, anchor: [i32; 3]) {
        schedule_tick(anchor, 1);
    }

    /// Hands a `BlockBroken` of one of this kind's blocks to [`MachineSpec::removed`].
    pub fn on_broken(
        &mut self,
        pos: [i32; 3],
        block: BlockId,
        group: Option<&CellGroupData>,
        data: &[(String, Vec<u8>)],
    ) {
        if self.owns(block) {
            self.spec.removed(group.map_or(pos, |g| g.anchor), data);
        }
    }

    /// Steps every machine one of `hooks` names, once each, then asks a tick for each one whose
    /// step changed something.
    pub fn on_hooks(&mut self, caches: &mut Caches, hooks: &[(BlockHookKind, [i32; 3])]) {
        let Some(block) = self.block else {
            return;
        };
        let cells: Vec<[i32; 3]> = hooks
            .iter()
            .filter(|(kind, _)| *kind != BlockHookKind::RandomTick)
            .map(|(_, pos)| *pos)
            .collect();
        if cells.is_empty() {
            return;
        }
        let groups = paged(cells.clone(), cell_groups);
        let mut seen = HashSet::new();
        let anchors: Vec<[i32; 3]> = cells
            .iter()
            .zip(groups)
            .map(|(pos, group)| group.map_or(*pos, |g| g.anchor))
            .filter(|anchor| seen.insert(*anchor))
            .collect();
        let live: Vec<([i32; 3], BlockId)> = anchors
            .iter()
            .zip(paged(anchors.clone(), get_blocks))
            .filter_map(|(pos, b)| b.filter(|b| self.owns(*b)).map(|b| (*pos, b)))
            .collect();
        if live.is_empty() {
            return;
        }
        let positions: Vec<[i32; 3]> = live.iter().map(|(p, _)| *p).collect();
        let containers = paged(positions.clone(), |page| {
            container_get_many(page.into_iter().map(ContainerAddress::from).collect())
        });
        let states = paged(positions.clone(), |page| {
            section_kv_get_many(S::STATE_KEY, page)
        });
        let auxiliary: Vec<_> = S::AUX_KEYS
            .iter()
            .map(|key| paged(positions.clone(), |page| section_kv_get_many(key, page)))
            .collect();
        let viewers = viewers_by_anchor();
        let mut out = Presentation::default();
        let mut swaps = Vec::new();
        let mut busy = Vec::new();
        for (i, (((pos, current), slots), state)) in
            live.into_iter().zip(containers).zip(states).enumerate()
        {
            let aux: Vec<_> = auxiliary
                .iter()
                .map(|values| values[i].clone().unwrap_or_default())
                .collect();
            let mut slots = slots.unwrap_or_default();
            let before_slots = slots.clone();
            let mut state = state.unwrap_or_default();
            let before_state = state.clone();
            let ctx = StepCtx {
                pos,
                current,
                block,
                variants: &self.variants,
                viewers: viewers.get(&pos).map_or(&[], Vec::as_slice),
                aux: &aux,
            };
            let rows = out.rows.len();
            self.spec
                .step(&ctx, caches, &mut slots, &mut state, &mut out);
            let mut changed = false;
            for (at, want) in out.rows.drain(rows..) {
                if want != current {
                    swaps.push((at, want));
                    changed = true;
                }
            }
            let writes = changed_slots(&before_slots, &slots);
            if !writes.is_empty() {
                container_set(pos.into(), writes);
                changed = true;
            }
            if state != before_state {
                out.state.push((pos, (!state.is_empty()).then_some(state)));
                changed = true;
            }
            if changed {
                busy.push((pos, 1));
            }
        }
        out.flush(S::STATE_KEY);
        for (pos, row) in swaps {
            swap_block(pos, row);
        }
        paged(busy, |page| {
            schedule_ticks(page);
            Vec::<()>::new()
        });
    }
}

fn viewers_by_anchor() -> HashMap<[i32; 3], Vec<PlayerId>> {
    let mut by_anchor: HashMap<[i32; 3], Vec<PlayerId>> = HashMap::new();
    for viewer in gui_viewers() {
        if let Some(ContainerAddress::Block(anchor)) = viewer.anchor {
            by_anchor.entry(anchor).or_default().push(viewer.player_id);
        }
    }
    by_anchor
}

#[derive(Default)]
pub struct Presentation {
    draws: Vec<([i32; 3], Vec<DrawPrim>)>,
    parts: Vec<([i32; 3], u32, Option<[u8; 3]>)>,
    rows: Vec<([i32; 3], BlockId)>,
    state: Vec<([i32; 3], Option<Vec<u8>>)>,
}

impl Presentation {
    pub fn parts(&mut self, pos: [i32; 3], mask: u32, tint: Option<[u8; 3]>) {
        self.parts.push((pos, mask, tint));
    }

    pub fn draw(&mut self, pos: [i32; 3], prims: Vec<DrawPrim>) {
        self.draws.push((pos, prims));
    }

    /// The row the machine at `pos` should stand as (lit, full); swapped in when it differs.
    pub fn row(&mut self, pos: [i32; 3], block: BlockId) {
        self.rows.push((pos, block));
    }

    fn flush(self, state_key: &str) {
        let Presentation {
            draws,
            parts,
            rows: _,
            state,
        } = self;
        paged(state, |page| section_kv_set_many(state_key, page));
        paged(parts, set_model_parts_many);
        paged(draws, set_block_draws);
    }
}

/// An item row's `petramond:fuel`: how long one burns.
#[derive(serde::Deserialize)]
struct Fuel {
    burn_ticks: u32,
}

#[derive(Default)]
pub struct Caches {
    fuel_ticks: Option<HashMap<String, u32>>,
    max_stack: HashMap<String, u8>,
    recipes: HashMap<(String, String), Option<ItemStackData>>,
}

impl Caches {
    pub fn fuel_ticks_for(&mut self, item: &str) -> u32 {
        let fuel = self.fuel_ticks.get_or_insert_with(|| {
            let rows = items_with_data_as::<Fuel>("petramond:fuel");
            let names = item_names(rows.iter().map(|(id, _)| *id).collect());
            rows.into_iter()
                .zip(names)
                .filter_map(|((_, fuel), name)| Some((name?, fuel.burn_ticks)))
                .collect()
        });
        fuel.get(item).copied().unwrap_or(0)
    }

    pub fn max_stack_for(&mut self, item: &str) -> u8 {
        if let Some(&m) = self.max_stack.get(item) {
            return m;
        }
        let m = item_info(item).map(|i| i.max_stack).unwrap_or(64);
        self.max_stack.insert(item.to_owned(), m);
        m
    }

    pub fn recipe_for(&mut self, class: &str, item: &str) -> Option<ItemStackData> {
        let key = (class.to_owned(), item.to_owned());
        if let Some(cached) = self.recipes.get(&key) {
            return cached.clone();
        }
        let result = recipe_result(class, item);
        self.recipes.insert(key, result.clone());
        result
    }
}

pub fn output_accepts(
    caches: &mut Caches,
    output: &Option<ItemStackData>,
    result: &ItemStackData,
) -> bool {
    match output {
        None => true,
        Some(o) => {
            o.item == result.item
                && o.data == result.data
                && caches.max_stack_for(&o.item).saturating_sub(o.count) >= result.count
        }
    }
}

pub fn merge_output(output: &mut Option<ItemStackData>, result: &ItemStackData) {
    *output = Some(match output.take() {
        None => result.clone(),
        Some(o) => ItemStackData {
            count: o.count + result.count,
            ..o
        },
    });
}

pub fn consume_one(slot: &mut Option<ItemStackData>) {
    if let Some(s) = slot.take() {
        *slot = (s.count > 1).then(|| ItemStackData {
            count: s.count - 1,
            ..s
        });
    }
}

pub fn changed_slots(
    before: &[Option<ItemStackData>],
    after: &[Option<ItemStackData>],
) -> Vec<(u32, Option<ItemStackData>)> {
    (0..before.len().max(after.len()))
        .filter(|&i| slot_at(before, i) != slot_at(after, i))
        .map(|i| (i as u32, slot_at(after, i).clone()))
        .collect()
}

fn slot_at(slots: &[Option<ItemStackData>], i: usize) -> &Option<ItemStackData> {
    const EMPTY: &Option<ItemStackData> = &None;
    slots.get(i).unwrap_or(EMPTY)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str, count: u8) -> ItemStackData {
        ItemStackData {
            item: item.into(),
            count,
            data: Vec::new(),
        }
    }

    #[test]
    fn slot_writeback_diffs_and_tolerates_a_length_change() {
        let before = vec![Some(stack("a", 1)), None, Some(stack("c", 2))];
        let after = vec![Some(stack("a", 1)), Some(stack("b", 1))];
        let changed = changed_slots(&before, &after);
        assert_eq!(
            changed.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![1, 2],
            "the moved slot AND the one `after` dropped off its end"
        );
        assert_eq!(changed[0].1.as_ref().map(|s| s.item.as_str()), Some("b"));
        assert!(changed[1].1.is_none(), "the dropped tail is CLEARED");

        assert_eq!(changed_slots(&before, &before[..1]), vec![(2, None)]);
        let grown = vec![
            Some(stack("a", 1)),
            None,
            Some(stack("c", 2)),
            Some(stack("d", 1)),
        ];
        assert_eq!(
            changed_slots(&before, &grown)
                .iter()
                .map(|(i, _)| *i)
                .collect::<Vec<_>>(),
            vec![3]
        );
    }

    const MACHINE: BlockId = BlockId(7);
    const ANCHOR: [i32; 3] = [0, 64, 0];
    const FOOTPRINT: [i32; 3] = [1, 64, 0];

    /// Counts its state byte down to zero, one per step.
    #[derive(Default)]
    struct Countdown {
        steps: usize,
    }

    impl MachineSpec for Countdown {
        const BLOCK_KEY: &'static str = "test:machine";
        const VARIANT_KEYS: &'static [&'static str] = &[];
        const STATE_KEY: &'static str = "test:left";

        fn step(
            &mut self,
            _ctx: &StepCtx<'_>,
            _caches: &mut Caches,
            _slots: &mut Vec<Option<ItemStackData>>,
            state: &mut Vec<u8>,
            _out: &mut Presentation,
        ) {
            self.steps += 1;
            if let Some(left) = state.first_mut().filter(|left| **left > 0) {
                *left -= 1;
            }
        }
    }

    fn host() -> testing::MockHost {
        let host = testing::MockHost::new();
        host.set_block(ANCHOR, MACHINE);
        host.set_block(FOOTPRINT, MACHINE);
        host.set_cell_kv(ANCHOR, Countdown::STATE_KEY, &[2]);
        host.on(|call| match call {
            HostCall::Block(BlockCall::CellGroup { positions }) => Some(HostRet::CellGroups(
                positions
                    .iter()
                    .map(|_| {
                        Some(CellGroupData {
                            anchor: ANCHOR,
                            facing: None,
                            cells: vec![ANCHOR, FOOTPRINT],
                        })
                    })
                    .collect(),
            )),
            HostCall::Gui(GuiCall::GuiViewers) => Some(HostRet::GuiViewers(Vec::new())),
            HostCall::Block(
                BlockCall::ScheduleTicks { .. }
                | BlockCall::SetModelParts { .. }
                | BlockCall::SetBlockDraws { .. },
            ) => Some(HostRet::Unit),
            _ => None,
        });
        host
    }

    fn scheduled(host: &testing::MockHost) -> Vec<([i32; 3], u64)> {
        host.calls()
            .into_iter()
            .filter_map(|call| match call {
                HostCall::Block(BlockCall::ScheduleTicks { ticks }) => Some(ticks),
                _ => None,
            })
            .flatten()
            .collect()
    }

    #[test]
    fn a_machine_steps_once_a_tick_and_rests_once_a_step_changes_nothing() {
        let host = host();
        let _guard = host.install();
        let mut machine = Machine::<Countdown> {
            block: Some(MACHINE),
            ..Machine::default()
        };
        let mut caches = Caches::default();

        machine.on_hooks(
            &mut caches,
            &[
                (BlockHookKind::ContainerChanged, ANCHOR),
                (BlockHookKind::NeighborUpdate, FOOTPRINT),
                (BlockHookKind::RandomTick, ANCHOR),
            ],
        );
        assert_eq!(machine.spec.steps, 1, "every hook names the same machine");
        assert_eq!(
            scheduled(&host),
            vec![(ANCHOR, 1)],
            "it changed, so it asks again"
        );

        for left in [0u8, 0] {
            machine.on_hooks(&mut caches, &[(BlockHookKind::ScheduledTick, ANCHOR)]);
            assert_eq!(host.cell_kv(ANCHOR, Countdown::STATE_KEY), Some(vec![left]));
        }
        assert_eq!(machine.spec.steps, 3);
        assert_eq!(
            scheduled(&host),
            vec![(ANCHOR, 1), (ANCHOR, 1)],
            "the step that changed nothing asked for no tick"
        );

        host.set_block(ANCHOR, BlockId(1));
        machine.on_hooks(&mut caches, &[(BlockHookKind::ScheduledTick, ANCHOR)]);
        assert_eq!(
            machine.spec.steps, 3,
            "a cell that is no longer the machine never steps"
        );
    }
}
