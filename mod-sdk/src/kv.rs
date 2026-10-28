use crate::__rt::host_fn;

host_fn! {
    pub fn section_kv_find(section: [i32; 3], key: &str) -> Option<Vec<[i32; 3]>>
        => SectionKvFind { section, key: key.into() } => FoundBlocks
}

host_fn! {
    pub fn world_kv_get(key: &str) -> Option<Vec<u8>> => WorldKvGet { key: key.into() } => Bytes
}

host_fn! {
    pub fn world_kv_set(key: &str, value: Vec<u8>) -> bool
        => WorldKvSet { key: key.into(), value: Some(value) } => Bool
}

host_fn! {
    pub fn world_kv_delete(key: &str) -> bool => WorldKvSet { key: key.into(), value: None } => Bool
}

host_fn! {
    pub fn section_kv_get_many(key: &str, positions: Vec<[i32; 3]>) -> Vec<Option<Vec<u8>>>
        => SectionKvGet { key: key.into(), positions } => BytesMany
}

host_fn! {
    pub fn section_kv_set_many(key: &str, writes: Vec<([i32; 3], Option<Vec<u8>>)>) -> Vec<bool>
        => SectionKvSet { key: key.into(), writes } => Bools
}

pub fn section_kv_get(pos: [i32; 3], key: &str) -> Option<Vec<u8>> {
    section_kv_get_many(key, vec![pos]).pop().flatten()
}

pub fn section_kv_set(pos: [i32; 3], key: &str, value: Vec<u8>) -> bool {
    section_kv_set_many(key, vec![(pos, Some(value))])
        .pop()
        .unwrap_or(false)
}

pub fn section_kv_delete(pos: [i32; 3], key: &str) -> bool {
    section_kv_set_many(key, vec![(pos, None)])
        .pop()
        .unwrap_or(false)
}
