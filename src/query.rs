use secret_toolkit::crypto::sha_256;

use crate::{
    msg::{DynamicItemData, GetItems, ItemData, QueryMsg, ResponseStatus},
    state::{
        get_category_item_dynamic_data, get_category_item_user_contact_data, get_category_items,
        get_ctegory_user_items,
    },
};
use cosmwasm_std::{
    to_binary, Api, Binary, Extern, HumanAddr, Querier, QueryResult, StdResult, Storage,
};

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetItems { .. } => viewing_keys_queries(deps, msg),
    }
}

pub fn viewing_keys_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {
    msg.authenticate(deps)?;

    match msg {
        QueryMsg::GetItems {
            address, category, ..
        } => to_binary(&may_get_items(deps, &address, category)?),
    }
}

// Elad: move out the get prefix function to the calling function!
// Elad: Replace all String with the proper &str
pub fn may_get_items<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account: &HumanAddr,
    category: String,
) -> StdResult<GetItems> {
    let address = deps.api.canonical_address(account)?;

    let (items_static_data, status) = get_category_items(&deps.storage, &category)?;
    let mut items_data = Vec::new();
    for item_static_data in items_static_data.iter() {
        let key = sha_256(base64::encode(item_static_data.url.clone()).as_bytes());
        let (quantity, status) = get_category_item_dynamic_data(&deps.storage, &category, &key)?;
        items_data.push(ItemData {
            static_data: item_static_data.clone(),
            dynamic_data: {
                DynamicItemData {
                    current_group_size: quantity.current_group_size,
                }
            },
        })
    }

    let (user_items, status) =
        get_ctegory_user_items(&deps.storage, &category, &address.as_slice())?;

    for user_items_iter in user_items.iter() {
        let key = sha_256(base64::encode(user_items_iter.url.clone()).as_bytes());

        let (contact_data, status) =
            get_category_item_user_contact_data(&deps.storage, &category, &key, &account)?;

        let result = GetItems {
            items: items_data,
            user_items,
            contact_data: Some(contact_data),
            status: ResponseStatus::Success,
        };
        return Ok(result);
    }

    let result = GetItems {
        items: items_data,
        user_items,
        contact_data: None,
        status: ResponseStatus::Success,
    };
    return Ok(result);
}
