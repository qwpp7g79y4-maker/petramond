use super::super::*;

fn shader_bit_reads(src: &str) -> Vec<(&'static str, u32, u32)> {
    let hex = |s: &str| -> Option<u32> {
        let end = s.find('u')?;
        u32::from_str_radix(&s[..end], 16).ok()
    };
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = src[at..].find("packed") {
        let start = at + i;
        at = start + "packed".len();
        let word = if src[at..].starts_with('2') {
            at += 1;
            "packed2"
        } else {
            "packed"
        };
        let tail = src[at..].trim_start();
        if let Some(rest) = tail.strip_prefix(">> ") {
            let Some(u) = rest.find("u) & 0x") else {
                continue;
            };
            let (Ok(shift), Some(mask)) = (rest[..u].parse::<u32>(), hex(&rest[u + 7..])) else {
                continue;
            };
            out.push((word, shift, mask));
        } else if let Some(rest) = tail.strip_prefix("& 0x") {
            if let Some(mask) = hex(rest) {
                out.push((word, 0, mask));
            }
        }
    }
    out
}

#[test]
fn the_generated_transition_decode_reads_the_lanes_apply_writes() {
    let text = super::super::transition::wgsl();
    let reads = shader_bit_reads(&text);
    for want in [
        ("packed", 0, TILE_MASK),
        ("packed", OVERLAY_FLAG.trailing_zeros(), 0x1),
        ("packed2", CELL_UV_U_SHIFT, 0x3FF),
        ("packed2", DYED_FLAG2.trailing_zeros(), 0x1FF),
        ("packed", UV_MODE_SHIFT, 0x7),
        ("packed", SHADE_SHIFT, 0x3),
    ] {
        assert!(reads.contains(&want), "transition decode misses {want:?}");
    }
    assert!(
        text.contains("packed2 >> 28u"),
        "high nibble starts at bit 28"
    );
    assert!(
        text.contains("(packed >> 31u) << 12u"),
        "packed bit 31 is slot bit 12"
    );
}
