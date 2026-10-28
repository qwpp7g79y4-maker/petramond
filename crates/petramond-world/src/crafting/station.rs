//! Crafting stations (`stations.json`): which GUI a station opens, the screen that lays it out, and
//! the stations whose recipes it also offers. A recipe names the station it needs; a station
//! offers its own recipes and every station it includes, transitively.

use serde::Deserialize;

use crate::gui_state::GuiKind;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CraftingStation(GuiKind);

#[derive(Clone, Debug)]
pub struct StationDef {
    pub kind: GuiKind,
    /// The document that lays the station out: its own kind's, or a shared one.
    pub screen: GuiKind,
    /// Every station whose recipes this one offers, itself included.
    pub offers: Vec<GuiKind>,
}

#[allow(non_upper_case_globals)]
impl CraftingStation {
    /// The player's own grid.
    pub const Inventory: CraftingStation = CraftingStation(GuiKind::Inventory);

    pub fn from_key(key: &str) -> Option<Self> {
        let kind = crate::gui_state::resolve_kind(key)?;
        Self::of_kind(kind)
    }

    pub fn of_kind(kind: GuiKind) -> Option<Self> {
        stations()
            .iter()
            .any(|s| s.kind == kind)
            .then_some(Self(kind))
    }

    fn def(self) -> &'static StationDef {
        stations()
            .iter()
            .find(|s| s.kind == self.0)
            .expect("a CraftingStation is built from a loaded row")
    }

    pub fn gui_kind(self) -> GuiKind {
        self.0
    }

    pub fn screen(self) -> GuiKind {
        self.def().screen
    }

    pub fn key(self) -> &'static str {
        crate::gui_state::kind_key(self.0).unwrap_or("?")
    }

    /// Whether a recipe needing `required` can be made here.
    pub fn admits(self, required: Self) -> bool {
        self.def().offers.contains(&required.0)
    }
}

const ENGINE_STATIONS: &[&str] = &[
    "petramond:inventory",
    "petramond:crafting_table",
    "petramond:furniture_workbench",
    "petramond:chiseling_station",
];

#[derive(Deserialize)]
struct RawStations {
    stations: Vec<RawStation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStation {
    station: String,
    #[serde(default)]
    screen: Option<String>,
    #[serde(default)]
    includes: Vec<String>,
}

pub struct Stations {
    rows: Vec<StationDef>,
}

pub(crate) static STATIONS: crate::content::Slot<Stations> =
    crate::content::Slot::new(crate::content::stage::STATIONS, &[], |reg| {
        crate::registry::read_catalog(reg.packs(), "stations.json", "crafting station", parse)
    });

fn stations() -> &'static [StationDef] {
    &STATIONS.current().rows
}

fn parse(texts: &[&str]) -> Result<Stations, String> {
    let catalog = crate::registry::load_catalog(
        texts,
        |text| serde_json::from_str::<RawStations>(text).map(|f| f.stations),
        |r| &r.station,
        ENGINE_STATIONS,
        "crafting station",
        |r, id, names| {
            let name = names.name(id).expect("id resolved from this table");
            let kind = |key: &str| {
                crate::gui_state::intern_kind(key)
                    .ok_or_else(|| format!("crafting station '{name}': '{key}' is not a GUI kind"))
            };
            let includes = r
                .includes
                .iter()
                .map(|key| match names.id(key) {
                    Some(_) => kind(key),
                    None => Err(format!(
                        "crafting station '{name}' includes unknown station '{key}'"
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let own = kind(name)?;
            Ok((
                own,
                r.screen.as_deref().map(kind).transpose()?.unwrap_or(own),
                includes,
            ))
        },
    )?;
    let direct = catalog.rows();
    let rows = direct
        .iter()
        .map(|&(kind, screen, _)| {
            let mut offers = vec![kind];
            let mut i = 0;
            while i < offers.len() {
                let reached = direct.iter().find(|row| row.0 == offers[i]);
                for &next in reached.map_or(&[][..], |row| &row.2) {
                    if !offers.contains(&next) {
                        offers.push(next);
                    }
                }
                i += 1;
            }
            StationDef {
                kind,
                screen,
                offers,
            }
        })
        .collect();
    Ok(Stations { rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_station_offers_what_its_includes_reach_transitively() {
        let text = r#"{"stations": [
            {"station": "petramond:inventory"},
            {"station": "petramond:crafting_table", "includes": ["petramond:inventory"]},
            {"station": "petramond:furniture_workbench"},
            {"station": "petramond:chiseling_station"},
            {"station": "stationtest:bench", "includes": ["petramond:crafting_table"],
             "screen": "petramond:crafting_table"}
        ]}"#;
        let stations = parse(&[text]).expect("parses");
        let bench = stations.rows.last().unwrap();
        assert_eq!(bench.screen, GuiKind::CraftingTable);
        assert_eq!(
            bench.offers,
            vec![bench.kind, GuiKind::CraftingTable, GuiKind::Inventory]
        );
        let unknown = text.replace("\"petramond:crafting_table\"]", "\"stationtest:nowhere\"]");
        assert!(
            parse(&[&unknown]).is_err(),
            "an include must name a station"
        );
    }
}
