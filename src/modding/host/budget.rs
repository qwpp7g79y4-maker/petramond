use mod_api::RuntimeSide;

pub(in crate::modding) const HOST_CALL_BASE_FUEL: u64 = 1_000;

pub(in crate::modding) const HOST_CALL_FUEL_PER_BYTE: u64 = 4;

pub(in crate::modding) fn host_call_fuel(
    side: RuntimeSide,
    request_len: usize,
    reply_len: usize,
) -> u64 {
    if side == RuntimeSide::Client {
        return HOST_CALL_BASE_FUEL;
    }
    let bytes = (request_len as u64).saturating_add(reply_len as u64);
    HOST_CALL_BASE_FUEL.saturating_add(bytes.saturating_mul(HOST_CALL_FUEL_PER_BYTE))
}
