use secret_toolkit::crypto::sha_256;

use crate::{
    msg::{GetItems, ItemData, QueryMsg, ResponseStatus},
    state::{
        get_category_item_group_size, get_category_item_user_details, get_category_items,
        get_category_prefixes, get_ctegory_user_items_quantities,
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

    let (static_prefix, dynamic_prefix, dynamic_prefix_users) =
        get_category_prefixes(category.as_bytes())?;
    let items_static_data = get_category_items(&deps.storage, &static_prefix)?;

    let mut items_data = Vec::new();
    for item_static_data in items_static_data.iter() {
        let key = sha_256(base64::encode(item_static_data.url.clone()).as_bytes());
        let current_group_size =
            match get_category_item_group_size(&deps.storage, &dynamic_prefix, &key)? {
                Some(v) => v,
                None => 0,
            };
        items_data.push(ItemData {
            static_data: item_static_data.clone(),
            current_group_size,
        })
    }

    let user_items =
        get_ctegory_user_items_quantities(&deps.storage, &dynamic_prefix, &address.as_slice())?;

    for user_items_iter in user_items.iter() {
        let key = sha_256(base64::encode(user_items_iter.url.clone()).as_bytes());

        let contact_data = match get_category_item_user_details(
            &deps.storage,
            &dynamic_prefix_users,
            &key,
            &account,
        )? {
            Some(v) => Some(v.contact_data),
            None => None,
        };

        let result = GetItems {
            items: items_data,
            user_items,
            contact_data,
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
