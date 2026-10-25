use super::*;

const FIELD: ReefField = ReefField {
    lattice: 15,
    per_mille: 820,
    radius: (3, 7),
};

/// Nine colours around the wheel with every form, as distinct ids.
fn palette() -> Vec<Coral> {
    let forms = [
        Form::Block,
        Form::Fan,
        Form::Brain,
        Form::Table,
        Form::Plate,
        Form::Tube,
        Form::Whip,
    ];
    (0..9u16)
        .map(|c| {
            let mut coral = Coral {
                hue: f32::from(c) * 40.0,
                ..Coral::default()
            };
            for (f, form) in forms.into_iter().enumerate() {
                coral.set(form, Some(BlockId(100 + c * 10 + f as u16)));
            }
            coral
        })
        .collect()
}

fn columns(seed: u32, room: i32, span: i32) -> Vec<([i32; 2], Reef)> {
    columns_in(seed, room, span, &palette())
}

fn columns_in(seed: u32, room: i32, span: i32, palette: &[Coral]) -> Vec<([i32; 2], Reef)> {
    let mut out = Vec::new();
    for wz in -span..span {
        for wx in -span..span {
            if let Some(reef) = plan(seed, wx, wz, room, &FIELD, palette) {
                out.push(([wx, wz], reef));
            }
        }
    }
    out
}

/// Every colour a column or fringe cell shows.
fn colours(reef: Reef) -> Vec<usize> {
    match reef {
        Reef::Head(c) => {
            let crown = match c.crown {
                Some(Crown::Fan(x) | Crown::Table(x) | Crown::Tube(x) | Crown::Whip(x)) => Some(x),
                _ => None,
            };
            [c.species.colour(), crown].into_iter().flatten().collect()
        }
        Reef::Fringe(Fringe::Fan(x) | Fringe::Brain(x) | Fringe::Tube(x)) => vec![x],
        Reef::Fringe(_) => Vec::new(),
    }
}

#[test]
fn heads_keep_inside_the_water_and_lanterns_shelter_under_caps() {
    for seed in [1, 9, 77] {
        for room in [2, 4, 7, 12] {
            for (at, reef) in columns(seed, room, 40) {
                let Reef::Head(c) = reef else { continue };
                assert!(
                    1 <= c.base && c.base <= c.top && c.height() <= room,
                    "{at:?} in room {room}: {c:?}"
                );
                assert!(c.top < room, "{at:?} reaches the waterline: {c:?}");
                assert!(
                    !c.lantern || c.base >= 3,
                    "{at:?} lantern with no room: {c:?}"
                );
            }
        }
    }
}

/// Every cell of a head is joined, through columns whose filled spans overlap, to a column
/// standing on the floor: a cap never floats.
#[test]
fn every_cap_hangs_from_a_stem() {
    let mut undercut = 0;
    for seed in [1, 9, 77, 1234] {
        let heads: std::collections::HashMap<[i32; 2], Column> = columns(seed, 14, 60)
            .into_iter()
            .filter_map(|(at, reef)| match reef {
                Reef::Head(c) => Some((at, c)),
                Reef::Fringe(_) => None,
            })
            .collect();
        for (&at, &start) in &heads {
            if start.base == 1 {
                continue;
            }
            undercut += 1;
            let mut seen = std::collections::HashSet::from([at]);
            let mut queue = vec![(at, start)];
            let mut grounded = false;
            while let Some(([x, z], c)) = queue.pop() {
                if c.base == 1 {
                    grounded = true;
                    break;
                }
                for n in [[x + 1, z], [x - 1, z], [x, z + 1], [x, z - 1]] {
                    let Some(&nc) = heads.get(&n) else { continue };
                    if nc.base <= c.top && c.base <= nc.top && seen.insert(n) {
                        queue.push((n, nc));
                    }
                }
            }
            assert!(grounded, "seed {seed}: the cap at {at:?} floats: {start:?}");
        }
    }
    assert!(undercut > 0, "some heads are undercut");
}

/// A head's colour comes in one theme: at most three colours, the dominant one's neighbours on
/// the wheel or, now and then, one across it.
#[test]
fn a_head_keeps_to_one_colour_theme() {
    let palette = palette();
    let n = palette.len();
    let mut contrasts = 0;
    for seed in [1, 9, 77] {
        let mut themes: std::collections::HashMap<[i32; 2], std::collections::BTreeSet<usize>> =
            Default::default();
        for (at, reef) in columns(seed, 14, 60) {
            let (head, _) = nearest(seed, at[0], at[1], &FIELD, &palette).expect("a head");
            themes.entry(head.center).or_default().extend(colours(reef));
        }
        assert!(!themes.is_empty());
        for (center, colours) in themes {
            assert!(
                colours.len() <= 3,
                "seed {seed}: {center:?} shows {colours:?}"
            );
            let near = |a: usize, b: usize| {
                let d = (a + n - b) % n;
                d <= 2 || n - d <= 2
            };
            let fits = colours.is_empty()
                || colours.iter().any(|&d| {
                    colours
                        .iter()
                        .all(|&c| near(c, d) || (c + n - d) % n == n / 2)
                });
            assert!(fits, "seed {seed}: head at {center:?} scatters {colours:?}");
            contrasts += usize::from(colours.iter().any(|&a| {
                colours.iter().any(|&b| {
                    let d = (a + n - b) % n;
                    d > 2 && n - d > 2
                })
            }));
        }
    }
    assert!(contrasts > 0, "some heads carry a contrasting accent");
}

/// A colour grows only the forms it has rows for; the rest of the palette fills in.
#[test]
fn a_colour_grows_only_the_forms_it_has() {
    let mut sparse = palette();
    for (i, coral) in sparse.iter_mut().enumerate() {
        coral.set(Form::Table, None);
        coral.set(Form::Whip, (i % 2 == 0).then_some(BlockId(9)));
        coral.set(Form::Brain, (i % 3 == 0).then_some(BlockId(8)));
    }
    let blocks = ReefBlocks {
        rock: BlockId(1),
        staghorn: BlockId(2),
        anemone: BlockId(3),
        lantern: BlockId(4),
    };
    let mut grown = 0;
    for seed in [1, 9, 77] {
        for (at, reef) in columns_in(seed, 14, 60, &sparse) {
            match reef {
                Reef::Head(c) => {
                    assert!(
                        !matches!(c.crown, Some(Crown::Table(_))),
                        "{at:?} grows a table nobody has: {c:?}"
                    );
                    if let Species::Brain(x) | Species::Whips(x) = c.species {
                        let has = match c.species {
                            Species::Brain(_) => sparse[x].has(Form::Brain),
                            _ => sparse[x].has(Form::Whip),
                        };
                        assert!(has, "{at:?}: {c:?}");
                    }
                    for dy in 1..=c.height() {
                        grown += usize::from(head_cell(c, dy, &blocks, &sparse).is_some());
                    }
                }
                Reef::Fringe(f) => {
                    assert!(fringe_cell(f, &blocks, &sparse).is_some(), "{at:?}: {f:?}");
                }
            }
        }
    }
    assert!(grown > 0);
}
