use std::collections::HashMap;

use crate::keys;
use crate::schema::{read_rows, MetalSpec, MouldSpec};

const MELT_TICKS_DEFAULT: u32 = 240;
const MOLTEN_DEFAULT: [u8; 3] = [168, 96, 54];
const SOLID_DEFAULT: [u8; 3] = [96, 62, 44];

#[derive(Clone)]
pub struct Metal {
    pub melt_ticks: u32,
    pub molten: [u8; 3],
    pub solid: [u8; 3],
    pub name: Option<String>,
    pub melts_to: Option<String>,
}

impl Default for Metal {
    fn default() -> Self {
        Metal {
            melt_ticks: MELT_TICKS_DEFAULT,
            molten: MOLTEN_DEFAULT,
            solid: SOLID_DEFAULT,
            name: None,
            melts_to: None,
        }
    }
}

#[derive(Default)]
pub struct Casting {
    moulds: HashMap<String, String>,
    metals: HashMap<String, Metal>,
}

struct RawMetal {
    melt_ticks: Option<u32>,
    molten: Option<[u8; 3]>,
    solid: Option<[u8; 3]>,
    name: Option<String>,
    melts_to: Option<String>,
}

fn resolve_metals(raw: HashMap<String, RawMetal>) -> HashMap<String, Metal> {
    let canonical = |name: &String, m: &RawMetal| -> Option<&RawMetal> {
        m.melts_to
            .as_ref()
            .filter(|to| *to != name)
            .and_then(|to| raw.get(to))
    };
    raw.iter()
        .map(|(name, m)| {
            let from = canonical(name, m);
            (
                name.clone(),
                Metal {
                    melt_ticks: m
                        .melt_ticks
                        .or_else(|| from?.melt_ticks)
                        .unwrap_or(MELT_TICKS_DEFAULT),
                    molten: m.molten.or_else(|| from?.molten).unwrap_or(MOLTEN_DEFAULT),
                    solid: m.solid.or_else(|| from?.solid).unwrap_or(SOLID_DEFAULT),
                    name: m.name.clone().or_else(|| from?.name.clone()),
                    melts_to: m.melts_to.clone(),
                },
            )
        })
        .collect()
}

impl Casting {
    pub fn resolve() -> Casting {
        let moulds: std::collections::HashMap<_, _> = read_rows::<MouldSpec>(keys::MOULD_DATA)
            .into_iter()
            .map(|(item, mould)| (item, mould.class))
            .collect();
        let metals = resolve_metals(
            read_rows::<MetalSpec>(keys::METAL_DATA)
                .into_iter()
                .map(|(item, spec)| (item, RawMetal::from(spec)))
                .collect(),
        );
        Casting { moulds, metals }
    }

    pub fn mould_class(&self, item: &str) -> Option<&str> {
        self.moulds.get(item).map(String::as_str)
    }

    pub fn metal(&self, item: &str) -> Metal {
        self.metals.get(item).cloned().unwrap_or_default()
    }

    pub fn is_metal(&self, item: &str) -> bool {
        self.metals.contains_key(item)
    }

    pub fn molten_form(&self, item: &str) -> String {
        self.metals
            .get(item)
            .and_then(|m| m.melts_to.clone())
            .unwrap_or_else(|| item.to_owned())
    }
}

#[cfg(test)]
impl Casting {
    pub fn for_test(moulds: &[(&str, &str)], metals: &[(&str, Option<&str>)]) -> Casting {
        Casting {
            moulds: moulds
                .iter()
                .map(|(i, c)| ((*i).to_owned(), (*c).to_owned()))
                .collect(),
            metals: metals
                .iter()
                .map(|(i, to)| {
                    (
                        (*i).to_owned(),
                        Metal {
                            melts_to: to.map(str::to_owned),
                            ..Metal::default()
                        },
                    )
                })
                .collect(),
        }
    }
}

impl From<MetalSpec> for RawMetal {
    fn from(spec: MetalSpec) -> RawMetal {
        RawMetal {
            melt_ticks: spec.melt_ticks.map(|t| t.max(1.0) as u32),
            molten: spec.molten,
            solid: spec.solid,
            name: spec.name,
            melts_to: spec.melts_to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_derived_form_inherits_its_metals_numbers() {
        let raw = |melts_to: Option<&str>, ticks: Option<u32>| RawMetal {
            melt_ticks: ticks,
            molten: ticks.map(|_| [1, 2, 3]),
            solid: ticks.map(|_| [4, 5, 6]),
            name: ticks.map(|_| "Iron".to_owned()),
            melts_to: melts_to.map(str::to_owned),
        };
        let metals = resolve_metals(HashMap::from([
            ("petramond:raw_iron".to_owned(), raw(None, Some(300))),
            (
                "forge:iron_plate".to_owned(),
                raw(Some("petramond:raw_iron"), None),
            ),
            ("other:mystery".to_owned(), raw(Some("other:absent"), None)),
        ]));
        let plate = &metals["forge:iron_plate"];
        assert_eq!(plate.melt_ticks, 300, "the plate melts at iron's rate");
        assert_eq!((plate.molten, plate.solid), ([1, 2, 3], [4, 5, 6]));
        assert_eq!(
            plate.name.as_deref(),
            Some("Iron"),
            "a plate melted down is iron — the crucible names the metal, not the form"
        );
        assert_eq!(metals["other:mystery"].melt_ticks, MELT_TICKS_DEFAULT);
        assert_eq!(metals["other:mystery"].name, None);
    }
}
