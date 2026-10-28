use crate::legality::prelude::*;

host_domain! {
    KvCall {
        WorldKvGet {
            key: String,
        } => legal(SERVER, Sim, Read),
        /// Writes `key`; `None` deletes it. → [`HostRet::Bool`](crate::HostRet::Bool): a write
        /// answers `true`, a delete whether the key was there.
        WorldKvSet {
            key: String,
            value: Option<Vec<u8>>,
        } => legal(SERVER, Sim, Write),
        /// `key` at each cell. → [`HostRet::BytesMany`](crate::HostRet::BytesMany).
        SectionKvGet {
            key: String,
            positions: Vec<[i32; 3]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// Writes `key` at each cell; `None` deletes it there. Per entry: `false` = unloaded, a
        /// full cell (a new key past `CELL_KV_MAX_KEYS`) or deleting an absent key.
        /// → [`HostRet::Bools`](crate::HostRet::Bools).
        SectionKvSet {
            key: String,
            writes: Vec<([i32; 3], Option<Vec<u8>>)>,
        } => legal(SERVER, Sim, Write),
        SectionKvFind {
            section: [i32; 3],
            key: String,
        } => legal(SERVER, Sim, Read),
    }
}
