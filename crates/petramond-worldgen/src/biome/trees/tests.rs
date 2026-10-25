use super::*;
use crate::feature::stub_species;

fn table(weights: &[u32]) -> SpeciesTable {
    let weighted: Vec<_> = weights.iter().map(|&w| (w, stub_species())).collect();
    SpeciesTable::new(&weighted).unwrap()
}

#[test]
fn weighted_species_split_one_roll_by_cumulative_weight() {
    let species = table(&[11, 30, 59]);
    let mut counts = [0usize; 3];
    let mut rng = FeatureRng::from_state(77);
    for _ in 0..20_000 {
        let picked = species.pick(&mut rng);
        let i = (0..3)
            .find(|&i| std::ptr::eq(species.entries[i].1, picked))
            .expect("a table entry");
        counts[i] += 1;
    }
    let shares = counts.map(|c| c as f32 / 20_000.0);
    for (share, weight) in shares.iter().zip([0.11, 0.30, 0.59]) {
        assert!((share - weight).abs() < 0.02, "shares {shares:?}");
    }
}
