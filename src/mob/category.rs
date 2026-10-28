//! Spawn categories (`spawn_categories.json`): a population species count toward, its cap, and
//! the spawner that tops it up around players. Engine rows use bare names (`passive`); a pack adds
//! its own under its namespace. A species with no category spawns only when something places it
//! and counts toward no cap.

use serde::Deserialize;

use petramond_world::registry::Catalog;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpawnCategory(pub u8);

#[derive(Clone, Debug)]
pub struct CategoryDef {
    pub name: &'static str,
    /// Live members at most, over the category's own spawn reach (a `scan` cap scales with the
    /// chunks in reach).
    pub cap: u32,
    /// How far from every player a member may stray before it despawns, unless its row says.
    pub despawn_radius: Option<f32>,
    /// Chunks the world first loads get a population of these species.
    pub populates: bool,
    pub spawner: Option<Spawner>,
}

/// How a category tops up the world around players.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Spawner {
    /// Every `every` ticks, per player: a group of one species at a column in the ring
    /// `near..far` blocks away, the species from those living in the column's biome.
    Ring { every: u64, near: f32, far: f32 },
    /// Every tick: a column in a loaded chunk around a player, scanned top-down for an open cell
    /// on a full floor `near..far` blocks from the nearest player that some species admits.
    Scan { near: f32, far: f32 },
}

const ENGINE_CATEGORIES: &[&str] = &["passive"];

#[derive(Deserialize)]
struct RawCategories {
    spawn_categories: Vec<RawCategory>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCategory {
    category: String,
    cap: u32,
    #[serde(default)]
    despawn_radius: Option<f32>,
    #[serde(default)]
    populates: bool,
    #[serde(default)]
    spawner: Option<RawSpawner>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum RawSpawner {
    Ring { every: u64, near: f32, far: f32 },
    Scan { near: f32, far: f32 },
}

pub(crate) static CATEGORIES: petramond_world::content::Slot<Catalog<CategoryDef>> =
    petramond_world::content::Slot::new("spawn_categories.json", &[], |reg| {
        petramond_world::registry::read_catalog(
            reg.packs(),
            "spawn_categories.json",
            "spawn category",
            parse,
        )
    });

impl SpawnCategory {
    #[inline]
    pub fn def(self) -> &'static CategoryDef {
        &CATEGORIES.current().rows()[self.0 as usize]
    }

    pub fn by_name(name: &str) -> Option<SpawnCategory> {
        CATEGORIES
            .current()
            .id(name)
            .map(|id| SpawnCategory(id as u8))
    }

    pub fn all() -> impl Iterator<Item = SpawnCategory> {
        (0..CATEGORIES.current().rows().len()).map(|i| SpawnCategory(i as u8))
    }
}

impl<'de> Deserialize<'de> for SpawnCategory {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        SpawnCategory::by_name(&name)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown spawn category '{name}'")))
    }
}

fn parse(texts: &[&str]) -> Result<Catalog<CategoryDef>, String> {
    petramond_world::registry::load_catalog(
        texts,
        |text| serde_json::from_str::<RawCategories>(text).map(|f| f.spawn_categories),
        |r| &r.category,
        ENGINE_CATEGORIES,
        "spawn category",
        |r, id, names| {
            let name = names.name(id).expect("id resolved from this table");
            let positive = |v: f32| v.is_finite() && v > 0.0;
            if r.despawn_radius.is_some_and(|d| !positive(d)) {
                return Err(format!(
                    "spawn category '{name}': despawn_radius must be positive"
                ));
            }
            let spawner = match r.spawner {
                None => None,
                Some(RawSpawner::Ring { every, near, far }) => {
                    if every == 0 || !(near >= 0.0 && positive(far) && near < far) {
                        return Err(format!(
                            "spawn category '{name}': ring needs every > 0 and 0 <= near < far"
                        ));
                    }
                    Some(Spawner::Ring { every, near, far })
                }
                Some(RawSpawner::Scan { near, far }) => {
                    if !(near >= 0.0 && positive(far) && near < far) {
                        return Err(format!(
                            "spawn category '{name}': scan needs 0 <= near < far"
                        ));
                    }
                    Some(Spawner::Scan { near, far })
                }
            };
            Ok(CategoryDef {
                name,
                cap: r.cap,
                despawn_radius: r.despawn_radius,
                populates: r.populates,
                spawner,
            })
        },
    )
}
