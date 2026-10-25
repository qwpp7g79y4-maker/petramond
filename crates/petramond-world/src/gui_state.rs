mod kind;

use crate::item::ItemStack;
use std::collections::BTreeMap;
use std::sync::Arc;

pub use kind::GuiKind;
pub use kind::{engine_kind_keys, intern_kind, intern_str, kind_key, resolve_kind};

pub const MAX_MENU_DRAG_SLOTS: usize =
    crate::container::MAX_CONTAINER_SLOTS + crate::inventory::TOTAL_SLOTS;

#[derive(Clone, Debug, PartialEq)]
pub enum GuiValue {
    F32(f32),
    I32(i32),
    Str(String),
    List(Vec<std::collections::BTreeMap<String, Self>>),
}

pub type GuiStateMap = BTreeMap<String, GuiValue>;

pub type WidgetId = &'static str;

pub fn empty_gui_state() -> Arc<GuiStateMap> {
    static EMPTY: std::sync::OnceLock<Arc<GuiStateMap>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(|| Arc::new(GuiStateMap::new())).clone()
}

pub fn gui_state_set(map: &mut Arc<GuiStateMap>, key: String, value: GuiValue) {
    Arc::make_mut(map).insert(key, value);
}

pub fn gui_state_clear(map: &mut Arc<GuiStateMap>) {
    if !map.is_empty() {
        *map = empty_gui_state();
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContainerView {
    pub slots: Vec<Option<ItemStack>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HealthView {
    pub current: i32,
    pub max: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MenuSlot {
    Inventory(usize),
    OffHand,
    Worn(crate::inventory::WearSlot),
    CraftResult,
    Container(usize),
    Widget(WidgetId),
}
