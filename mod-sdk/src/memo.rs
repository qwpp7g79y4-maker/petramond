use crate::__rt::host_fn;
use crate::MemoClaim;
mod blob;
pub use blob::{memo_blob_claim, memo_blob_put, MEMO_BLOB_MAX_BYTES};

host_fn! {
    pub fn memo_get_many(keys: Vec<Vec<u8>>) -> Vec<Option<Vec<u8>>>
        => MemoGet { keys } => BytesMany
}

pub fn memo_get(key: &[u8]) -> Option<Vec<u8>> {
    memo_get_many(vec![key.to_vec()]).pop().flatten()
}

host_fn! {
    /// [`memo_get`], but a miss also says who derives it. If you got the lease, derive the value
    /// and [`memo_put`] it. If you got Pending, somebody else has the lease; return
    /// [`GenOutput::deferred`](crate::GenOutput::deferred) and your section runs again later.
    /// A lease nobody fills expires and goes to the next claimant.
    pub fn memo_claim(key: &[u8]) -> MemoClaim => MemoClaim { key: key.to_vec() } => MemoClaim
}

host_fn! {
    pub fn memo_put(key: &[u8], value: Vec<u8>) -> bool => MemoPut { key: key.to_vec(), value } => Bool
}
