use std::sync::Mutex;

use crate::gui_state::GuiKind;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CraftingStation(GuiKind);

static REGISTERED_STATIONS: Mutex<Vec<GuiKind>> = Mutex::new(Vec::new());

#[allow(non_upper_case_globals)]
impl CraftingStation {
    pub const Inventory: CraftingStation = CraftingStation(GuiKind::Inventory);
    pub const CraftingTable: CraftingStation = CraftingStation(GuiKind::CraftingTable);
    pub const FurnitureWorkbench: CraftingStation = CraftingStation(GuiKind::FurnitureWorkbench);
    pub const ChiselingStation: CraftingStation = CraftingStation(GuiKind::ChiselingStation);

    pub const INVENTORY_KEY: &'static str = "petramond:inventory";
    pub const CRAFTING_TABLE_KEY: &'static str = "petramond:crafting_table";
    pub const FURNITURE_WORKBENCH_KEY: &'static str = "petramond:furniture_workbench";
    pub const CHISELING_STATION_KEY: &'static str = "petramond:chiseling_station";

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            Self::INVENTORY_KEY => Some(Self::Inventory),
            Self::CRAFTING_TABLE_KEY => Some(Self::CraftingTable),
            Self::FURNITURE_WORKBENCH_KEY => Some(Self::FurnitureWorkbench),
            Self::CHISELING_STATION_KEY => Some(Self::ChiselingStation),
            _ => {
                let kind = crate::gui_state::intern_kind(key)?;
                let mut stations = REGISTERED_STATIONS.lock().unwrap();
                if !stations.contains(&kind) {
                    stations.push(kind);
                }
                Some(Self(kind))
            }
        }
    }

    pub fn of_kind(kind: GuiKind) -> Option<Self> {
        match kind {
            GuiKind::Inventory => Some(Self::Inventory),
            GuiKind::CraftingTable => Some(Self::CraftingTable),
            GuiKind::FurnitureWorkbench => Some(Self::FurnitureWorkbench),
            GuiKind::ChiselingStation => Some(Self::ChiselingStation),
            _ => REGISTERED_STATIONS
                .lock()
                .unwrap()
                .contains(&kind)
                .then_some(Self(kind)),
        }
    }

    pub fn gui_kind(self) -> GuiKind {
        self.0
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Inventory => Self::INVENTORY_KEY,
            Self::CraftingTable => Self::CRAFTING_TABLE_KEY,
            Self::FurnitureWorkbench => Self::FURNITURE_WORKBENCH_KEY,
            Self::ChiselingStation => Self::CHISELING_STATION_KEY,
            Self(kind) => crate::gui_state::kind_key(kind).unwrap_or("?"),
        }
    }

    pub fn admits(self, required: Self) -> bool {
        self == required || (self == Self::CraftingTable && required == Self::Inventory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_stations_register_once_and_resolve_by_kind() {
        let a = CraftingStation::from_key("stationtest:bench").expect("namespaced key registers");
        let b = CraftingStation::from_key("stationtest:bench").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.key(), "stationtest:bench");
        assert_eq!(CraftingStation::of_kind(a.gui_kind()), Some(a));
        let plain = crate::gui_state::intern_kind("stationtest:not_a_bench").unwrap();
        assert_eq!(CraftingStation::of_kind(plain), None);
    }
}
