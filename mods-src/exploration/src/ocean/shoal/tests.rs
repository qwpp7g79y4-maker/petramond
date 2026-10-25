use super::*;

const SEA: i32 = 63;
const WARM: u8 = 50;

struct Flat {
    floor: i32,
    biome: u8,
}

impl Terrain for Flat {
    fn heights(&self, min: [i32; 2], max: [i32; 2]) -> Option<Heights> {
        Some(Heights::from_fn(min, max, |_, _| self.floor))
    }

    fn biomes(&self, columns: Vec<[i32; 2]>) -> Option<Vec<u8>> {
        Some(vec![self.biome; columns.len()])
    }
}

fn look() -> Look {
    let named = |k: &str| Material::named(k);
    let palette = (0..5)
        .map(|c| {
            let mut coral = Coral::default();
            coral.hue = c as f32 * 72.0;
            for (i, &form) in reef::FORM_LIST.iter().enumerate() {
                coral.set(form, Some(named(&format!("test:coral_{c}_{i}"))));
            }
            coral
        })
        .collect();
    Look {
        rock: named("test:rock"),
        crust: named("test:crust"),
        lantern: named("test:lantern"),
        anemone: named("test:anemone"),
        staghorn: named("test:staghorn"),
        seagrass: named("test:seagrass"),
        palette,
    }
}

/// One site's plan, as its cells.
type Built = (Site, Vec<([i32; 3], Material)>);

fn plans(depth: i32, count: usize) -> Vec<Built> {
    let terrain = Flat {
        floor: SEA - 1 - depth,
        biome: WARM,
    };
    let look = look();
    (0..60)
        .flat_map(|z| (0..60).map(move |x| [x, z]))
        .filter_map(|cell| GRID.site(3, cell))
        .filter_map(
            |site| match derive(3, SEA, site, &[(WARM, 1.0)], &look, &terrain) {
                Derived::Plan(plan) => Some((site, plan.cells().map(|(c, m)| (c, *m)).collect())),
                _ => None,
            },
        )
        .take(count)
        .collect()
}

/// Rock stands on the sea floor and keeps under water: every piece of it is joined to the floor
/// and nothing a shoal writes reaches the waterline.
#[test]
fn every_rock_stands_on_the_floor_under_water() {
    let depth = 16;
    let floor = SEA - 1 - depth;
    let found = plans(depth, 30);
    assert!(found.len() >= 10, "warm rock gardens grow formations");
    for (site, cells) in found {
        let rock: BTreeSet<[i32; 3]> = cells
            .iter()
            .filter(|(_, m)| m.block.as_str() == "test:rock")
            .map(|(c, _)| *c)
            .collect();
        let water = Material::named(keys::WATER);
        for (c, m) in &cells {
            if *m != water {
                assert!(c[1] < SEA - 1, "{site:?}: {c:?} reaches the waterline");
            }
            let (dx, dz) = (c[0] - site.center[0], c[2] - site.center[1]);
            assert!(
                dx.abs() <= REACH && dz.abs() <= REACH,
                "{c:?} escapes {site:?}"
            );
        }
        let solid: BTreeSet<[i32; 3]> = cells
            .iter()
            .filter(|(_, m)| {
                let n = m.block.as_str();
                n == "test:rock" || n == "test:crust" || n.ends_with("_0") || n.ends_with("_2")
            })
            .map(|(c, _)| *c)
            .collect();
        let mut reached = BTreeSet::new();
        let mut queue: Vec<[i32; 3]> = solid
            .iter()
            .copied()
            .filter(|c| c[1] <= floor + 1)
            .collect();
        reached.extend(queue.iter().copied());
        while let Some([x, y, z]) = queue.pop() {
            for n in [
                [x + 1, y, z],
                [x - 1, y, z],
                [x, y + 1, z],
                [x, y - 1, z],
                [x, y, z + 1],
                [x, y, z - 1],
            ] {
                if solid.contains(&n) && reached.insert(n) {
                    queue.push(n);
                }
            }
        }
        // Rubble round the foot lies on the floor, so it is reached too.
        assert_eq!(reached.len(), solid.len(), "{site:?}: rock floats");
        assert!(!rock.is_empty());
    }
}

/// Life grows on the rock: crowns stand on it, plates bracket out from a wall beside it, and
/// glowpods and anemones rest on the floor or the rock.
#[test]
fn life_grows_on_the_rock() {
    let depth = 16;
    let floor = SEA - 1 - depth;
    let mut plates = 0;
    for (site, cells) in plans(depth, 30) {
        let at: std::collections::BTreeMap<[i32; 3], &str> =
            cells.iter().map(|(c, m)| (*c, m.block.as_str())).collect();
        let solid = |c: [i32; 3]| {
            at.get(&c).is_some_and(|n| {
                *n == "test:rock" || *n == "test:crust" || n.ends_with("_0") || n.ends_with("_2")
            })
        };
        for (&[x, y, z], name) in &at {
            let stands = y == floor + 1 || solid([x, y - 1, z]);
            if name.ends_with("_4") {
                plates += 1;
                let beside = SIDES.iter().any(|[dx, dz]| solid([x + dx, y, z + dz]));
                let in_table = SIDES.iter().any(|[dx, dz]| {
                    at.get(&[x + dx, y, z + dz])
                        .is_some_and(|n| n.ends_with("_3") || n.ends_with("_4"))
                });
                assert!(
                    beside || in_table,
                    "{site:?}: a plate at {:?} hangs on nothing",
                    [x, y, z]
                );
            } else if *name == "test:lantern"
                || *name == "test:anemone"
                || *name == "test:staghorn"
                || *name == "test:seagrass"
                || name.ends_with("_1")
                || name.ends_with("_3")
                || name.ends_with("_5")
                || name.ends_with("_6")
            {
                assert!(
                    stands,
                    "{site:?}: {name} at {:?} stands on nothing",
                    [x, y, z]
                );
            }
        }
    }
    assert!(plates > 0, "walls carry plate corals");
}

#[test]
fn shallow_or_cold_seas_grow_none() {
    assert!(plans(MIN_ROOM + HEADROOM - 1, 1).is_empty(), "too shallow");
    let terrain = Flat {
        floor: SEA - 20,
        biome: WARM + 1,
    };
    let look = look();
    for x in 0..20 {
        if let Some(site) = GRID.site(3, [x, 0]) {
            assert!(matches!(
                derive(3, SEA, site, &[(WARM, 1.0)], &look, &terrain),
                Derived::Nothing
            ));
        }
    }
}

/// Every kind of formation stands, and arches and grottos leave water to swim through under or
/// inside their rock.
#[test]
fn every_kind_stands_and_some_can_be_swum_through() {
    let floor = Heights::from_fn([-40, -40], [40, 40], |_, _| 40);
    let mut seen = std::collections::BTreeMap::new();
    for i in 0..400u64 {
        let mut rng = GenRng::positional(7, i, 0, 0, 0);
        let f = Formation::roll(&mut rng, 7, 16);
        let cells = f.cells([0, 0], 40, 40 + 16, &floor);
        assert!(!cells.is_empty(), "{:?} stands nothing", f.kind());
        let swum = cells.iter().any(|&[x, y, z]| {
            y > 42 && (41..y).any(|yy| !cells.contains(&[x, yy, z])) && cells.contains(&[x, y, z])
        });
        let entry = seen.entry(format!("{:?}", f.kind())).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += usize::from(swum);
    }
    for kind in ["Arch", "Mushroom", "Spire", "Grotto", "Ridge"] {
        assert!(
            seen.get(kind).is_some_and(|(n, _)| *n > 0),
            "{kind} never rolls: {seen:?}"
        );
    }
    for kind in ["Arch", "Mushroom", "Grotto"] {
        let (n, swum) = seen[kind];
        assert!(
            swum * 2 > n,
            "{kind}: only {swum} of {n} leave water under rock"
        );
    }
}
