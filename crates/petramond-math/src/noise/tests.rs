use super::*;

#[track_caller]
fn assert_bits(got: f64, want: u64, what: &str) {
    assert_eq!(
        got.to_bits(),
        want,
        "{what}: got {got:?} ({:#018x}), want {:?} ({want:#018x})",
        got.to_bits(),
        f64::from_bits(want)
    );
}

const SIMPLEX: [([f64; 3], u64, u64); 10] = [
    ([0.0, 0.0, 0.0], 0x0000000000000000, 0x0000000000000000),
    ([1.25, -3.5, 8.75], 0x3fd32e80b4a8415c, 0xbfcf324b5ea9531f),
    ([-0.001, 0.001, 0.0], 0x3f71eb7bb98ba7dd, 0x3ca096ad19f29697),
    (
        [100.125, -20.25, 450.5],
        0xbfc9888124251fc5,
        0x3fccbb024011ccbc,
    ),
    (
        [-255.5, -256.25, -257.75],
        0xbfc300c3a9a5ad45,
        0xbfe0bffffffffd9e,
    ),
    ([0.5, 0.5, 0.5], 0xbfd3a873cb3210be, 0x0000000000000000),
    ([13.0, 13.0, -7.0], 0xbfd9a385d3986824, 0xbce3897423a91562),
    (
        [-1234567.875, 42.0625, 9876543.5],
        0x3fdedb62fc3b3a09,
        0xbfde7c3a243da99a,
    ),
    (
        [30000000.0, -15000000.0, -27500000.0],
        0xbfdfeeb5409e5303,
        0xbfbb9de7c970271f,
    ),
    (
        [
            0.3333333333333333,
            -std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::E,
        ],
        0xbfc41b6a31e266cf,
        0xbfc9e44488a4bdbc,
    ),
];

#[test]
fn simplex_outputs_stay_fixed() {
    for (p, two, three) in SIMPLEX {
        assert_bits(simplex2(p[0], p[2]), two, &format!("simplex2 at {p:?}"));
        assert_bits(simplex3(p), three, &format!("simplex3 at {p:?}"));
    }
}

#[test]
fn cellular2_outputs_stay_fixed() {
    type CellularCase = (u32, [f64; 2], f64, [u64; 2], u64, i32);
    let cases: [CellularCase; 6] = [
        (
            0x0,
            [0.0, 0.0],
            1.0,
            [0xbfbe3582d64c7b00, 0xbfdaee1a1e5d7f40],
            0x3fdbf811bd7ceaf5,
            0,
        ),
        (
            0x1,
            [0.49, -0.51],
            1.0,
            [0x3fe5ef4ca0bf2000, 0xbfe64a971802bda0],
            0x3fd14b856bf834a6,
            412623973,
        ),
        (
            0x3039,
            [17.25, -3.75],
            0.5,
            [0x4030f25ac7ae3063, 0xc010d8fe29a7d710],
            0x3fe1aec72e67d92b,
            1234777172,
        ),
        (
            0xdeadbeef,
            [-1000.5, 2000.25],
            1.0,
            [0xc08f422bd643b750, 0x409f3ea141804031],
            0x3fe452a9577f59c7,
            1189863739,
        ),
        (
            0x7,
            [-123456.125, 987654.75],
            0.0,
            [0xc0fe240000000000, 0x412e240e00000000],
            0x3fd1e3779b97f4a8,
            1824994550,
        ),
        (
            0x2a,
            [3.5, 3.5],
            0.75,
            [0x400dd2a5c02c35e6, 0x400e89e8ec0b8512],
            0x3fd900b69bb8b533,
            166437962,
        ),
    ];
    for (seed, point, jitter, centre, distance, hash) in cases {
        let at = format!("seed {seed:#x} at {point:?}, jitter {jitter}");
        let cell = cellular2(seed, point, jitter);
        assert_bits(cell.center[0], centre[0], &at);
        assert_bits(cell.center[1], centre[1], &at);
        assert_bits(cell.distance, distance, &at);
        assert_eq!(cell.hash, hash, "{at}");
    }
}
