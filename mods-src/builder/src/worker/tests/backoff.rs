use crate::worker::backoff::Tries;

#[test]
fn counting_never_overflows() {
    let mut tries = Tries::default();
    for _ in 0..300 {
        tries.count([0, 0, 0]);
    }
    assert_eq!(tries.count([0, 0, 0]), u8::MAX);
    assert!(!tries.within([0, 0, 0], 3));
}
