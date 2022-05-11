use crate::{
    handle::remove_item_authenticated,
    msg::{HandleAnswer, ResponseStatus, UpdateItemData, UserProductQuantity},
    state::{
        get_category_item_by_url, get_category_item_group_size, get_category_prefixes,
        get_category_user_items_quantities_by_url, remove_category_item_user_details,
        remove_user_item_quantity, save_category_element_user,
        save_category_element_user_item_details, update_category_item_user_details,
        update_current_group_size, update_user_item_quantity,
    },
};
use cosmwasm_std::{
    to_binary, Api, BankMsg, Coin, CosmosMsg, Env, Extern, HandleResponse, HumanAddr, Querier,
    StdError, StdResult, Storage, Uint128,
};
use secret_toolkit::crypto::sha_256;

pub fn update_user_for_item<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    update_item_data: UpdateItemData,
) -> StdResult<HandleResponse> {
    let (new_quantity, item_data, current_group_size, old_quantity_obj) =
        get_update_data(deps, &env, &update_item_data)?;

    if old_quantity_obj == None {
        return update_item_new_user(
            current_group_size,
            &item_data,
            &env,
            &update_item_data,
            deps,
        );
    }

    let old_quantity = old_quantity_obj.unwrap().quantity;

    // old_quantity is positive

    if new_quantity == 0 {
        // old_quantity > 0, new_quantity == 0

        remove_user_from_item(
            deps,
            current_group_size,
            old_quantity,
            &env,
            &update_item_data,
            &item_data,
        )?;
    }

    // old_quantity >0 , new_quantity > 0

    update_item_in_stores(
        deps,
        &env,
        current_group_size,
        old_quantity,
        &update_item_data,
    )?;

    if new_quantity < old_quantity {
        // refund the user: (the client side should charge the comission for that)
        return refund_user(old_quantity, &item_data, &env, &update_item_data);
    }

    // If the group size goal was reached, pay the seller and remove the item
    if new_quantity > old_quantity
        && current_group_size + new_quantity - old_quantity >= item_data.group_size_goal
    {
        return pay_seller(
            current_group_size,
            old_quantity,
            item_data,
            env,
            update_item_data,
            deps,
        );
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::UpdateItem {
            status: ResponseStatus::Success,
        })?),
    })
}

fn get_update_data<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    update_item_data: &UpdateItemData,
) -> Result<
    (
        u32,
        crate::msg::StaticItemData,
        u32,
        Option<UserProductQuantity>,
    ),
    StdError,
> {
    let sender_canonical_address = deps.api.canonical_address(&env.message.sender)?;
    let new_quantity = update_item_data.user_details.quantity;
    let (static_prefix, dynamic_prefix, _dynamic_prefix_users) =
        get_category_prefixes(update_item_data.category.as_bytes())?;
    let url_key = sha_256(base64::encode(update_item_data.url.clone()).as_bytes());
    let item_data =
        match get_category_item_by_url(&deps.storage, static_prefix, &update_item_data.url)? {
            Some(v) => v,
            None => {
                return Err(StdError::generic_err(
                    "Item data wasn't found. It should exist in this context",
                ))
            }
        };
    let current_group_size =
        match get_category_item_group_size(&deps.storage, dynamic_prefix, &url_key)? {
            Some(current_group_size) => current_group_size,
            None => return Err(StdError::generic_err("This item does not exist anymore")),
        };
    let old_quantity_obj = get_category_user_items_quantities_by_url(
        &deps.storage,
        dynamic_prefix,
        sender_canonical_address.as_slice(),
        &update_item_data.url,
    )?;
    Ok((
        new_quantity,
        item_data,
        current_group_size,
        old_quantity_obj,
    ))
}

fn pay_seller<S: Storage, A: Api, Q: Querier>(
    current_group_size: u32,
    old_quantity: u32,
    item_data: crate::msg::StaticItemData,
    env: Env,
    update_item_data: UpdateItemData,
    deps: &mut Extern<S, A, Q>,
) -> StdResult<HandleResponse> {
    let new_quantity = update_item_data.user_details.quantity;

    let seller_payment =
        (current_group_size + new_quantity - old_quantity) as u128 * item_data.wanted_price.u128();
    let transfer_funds_msg = transfer_funds(
        &env.contract.address,
        &HumanAddr(item_data.seller_address),
        seller_payment,
    )?;
    remove_item_authenticated(&update_item_data.category, &update_item_data.url, deps)?;
    Ok(transfer_funds_msg)
}

fn refund_user(
    old_quantity: u32,
    item_data: &crate::msg::StaticItemData,
    env: &Env,
    update_item_data: &UpdateItemData,
) -> StdResult<HandleResponse> {
    let new_quantity = update_item_data.user_details.quantity;

    let refund_amount = (old_quantity - new_quantity) as u128 * item_data.wanted_price.u128();
    let transfer_funds_msg = transfer_funds(
        &env.contract.address,
        &update_item_data.user_details.account_address,
        refund_amount,
    )?;
    Ok(transfer_funds_msg)
}

fn remove_user_from_item<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    current_group_size: u32,
    old_quantity: u32,
    env: &Env,
    update_item_data: &UpdateItemData,
    item_data: &crate::msg::StaticItemData,
) -> StdResult<HandleResponse> {
    let sender_canonical_address = deps.api.canonical_address(&env.message.sender)?;

    // Get the current product count of units: [dynamic_prefix, url]
    let (_static_prefix, dynamic_prefix, dynamic_prefix_users) =
        get_category_prefixes(update_item_data.category.as_bytes())?;
    let url_key = sha_256(base64::encode(update_item_data.url.clone()).as_bytes());

    update_current_group_size(
        &mut deps.storage,
        &url_key,
        dynamic_prefix,
        current_group_size - old_quantity,
    )?;
    remove_category_item_user_details(
        &mut deps.storage,
        dynamic_prefix_users,
        &url_key,
        &env.message.sender,
    )?;
    remove_user_item_quantity(
        &mut deps.storage,
        dynamic_prefix,
        sender_canonical_address.as_slice(),
        &update_item_data.url,
    )?;
    let refund_amount = (old_quantity as u128) * item_data.wanted_price.u128();
    let transfer_funds_msg = transfer_funds(
        &env.contract.address,
        &update_item_data.user_details.account_address,
        refund_amount,
    )?;
    Ok(transfer_funds_msg)
}

fn update_item_new_user<S: Storage, A: Api, Q: Querier>(
    current_group_size: u32,
    item_data: &crate::msg::StaticItemData,
    env: &Env,
    update_item_data: &UpdateItemData,
    deps: &mut Extern<S, A, Q>,
) -> StdResult<HandleResponse> {
    let sender_canonical_address = deps.api.canonical_address(&env.message.sender)?;

    // Get the current product count of units: [dynamic_prefix, url]
    let (_static_prefix, dynamic_prefix, dynamic_prefix_users) =
        get_category_prefixes(update_item_data.category.as_bytes())?;
    let url_key = sha_256(base64::encode(update_item_data.url.clone()).as_bytes());

    let new_quantity = update_item_data.user_details.quantity;
    if new_quantity == 0 {
        return Err(StdError::generic_err(
            "Cannot join a purchasing group with 0 quantity",
        ));
    }
    if new_quantity + current_group_size >= item_data.group_size_goal {
        let seller_payment =
            (current_group_size + new_quantity) as u128 * item_data.wanted_price.u128();
        let transfer_funds_msg = transfer_funds(
            &env.contract.address,
            &HumanAddr(item_data.seller_address.clone()),
            seller_payment,
        )?;

        remove_item_authenticated(&update_item_data.category, &update_item_data.url, deps)?;

        return Ok(transfer_funds_msg);
    }
    let url = update_item_data.url.clone();
    let user_product_quantity = UserProductQuantity {
        url,
        quantity: new_quantity,
    };
    save_category_element_user(
        &mut deps.storage,
        sender_canonical_address.as_slice(),
        dynamic_prefix,
        &user_product_quantity,
    )?;
    update_current_group_size(
        &mut deps.storage,
        &url_key,
        dynamic_prefix,
        current_group_size + new_quantity,
    )?;
    save_category_element_user_item_details(
        &mut deps.storage,
        &url_key,
        dynamic_prefix_users,
        &update_item_data.user_details,
    )?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::UpdateItem {
            status: ResponseStatus::Success,
        })?),
    })
}

fn update_item_in_stores<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    current_group_size: u32,
    old_quantity: u32,
    update_item_data: &UpdateItemData,
) -> Result<(), StdError> {
    let sender_canonical_address = deps.api.canonical_address(&env.message.sender)?;

    let new_quantity = update_item_data.user_details.quantity;

    // Get the current product count of units: [dynamic_prefix, url]
    let (_static_prefix, dynamic_prefix, dynamic_prefix_users) =
        get_category_prefixes(update_item_data.category.as_bytes())?;
    let url_key = sha_256(base64::encode(update_item_data.url.clone()).as_bytes());
    update_current_group_size(
        &mut deps.storage,
        &url_key,
        dynamic_prefix,
        current_group_size + new_quantity - old_quantity,
    )?;
    update_category_item_user_details(
        &mut deps.storage,
        dynamic_prefix_users,
        &url_key,
        update_item_data,
    )?;
    update_user_item_quantity(
        &mut deps.storage,
        dynamic_prefix,
        sender_canonical_address.as_slice(),
        update_item_data,
    )?;
    Ok(())
}

fn transfer_funds(
    from_address: &HumanAddr,
    to_address: &HumanAddr,
    amount: u128,
) -> StdResult<HandleResponse> {
    let from_address = from_address.clone();
    let to_address = to_address.clone();
    let msg = HandleResponse {
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            from_address,
            to_address,
            amount: vec![Coin {
                denom: "uscrt".into(),
                amount: Uint128(amount * 1000000),
            }],
        })],
        log: vec![],
        data: None,
    };

    Ok(msg)
}
