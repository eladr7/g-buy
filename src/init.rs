use cosmwasm_std::{Api, Env, Extern, InitResponse, Querier, StdResult, Storage};

use crate::msg::InitMsg;

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    // Elad: may add a maximum number of products
    Ok(InitResponse::default())
}
