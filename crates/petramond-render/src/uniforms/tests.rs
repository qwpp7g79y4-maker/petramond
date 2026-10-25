use super::*;

fn wgsl_size(ty: &str) -> usize {
    match ty {
        "mat4x4<f32>" => 64,
        "vec4<f32>" | "vec4<u32>" | "vec4<i32>" => 16,
        other => panic!("unhandled frame uniform type `{other}`"),
    }
}

#[test]
fn the_frame_table_describes_every_uniform_byte() {
    let mut offset = 0usize;
    for (name, ty, at) in UNIFORM_FIELDS {
        assert_eq!(
            at, offset,
            "`{name}` sits at byte {at} in `Uniforms` but the table reaches it at {offset} \
             (a field missing from UNIFORM_FIELDS?)"
        );
        offset += wgsl_size(ty);
    }
    assert_eq!(
        offset,
        std::mem::size_of::<Uniforms>(),
        "UNIFORM_FIELDS stops short of the end of `Uniforms`"
    );
}
