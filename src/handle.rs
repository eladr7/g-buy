use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    msg::{HandleAnswer, HandleMsg, RemoveItemData, ResponseStatus, StaticItemData},
    state::{
        get_all_participating_users_addresses, get_category_prefixes,
        remove_all_category_item_users_details, remove_category_item, remove_current_group_size,
        remove_user_item_quantity, save_new_item, update_current_group_size,
    },
    update_logic::update_user_for_item,
    viewing_key::ViewingKey,
};
use cosmwasm_std::{
    to_binary, Api, Env, Extern, HandleResponse, Querier, StdError, StdResult, Storage,
};
use secret_toolkit::crypto::sha_256;

#[derive(Serialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleResp {
    Success,
    ViewingKey(String),
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::AddItem(static_item_data) => add_new_item(deps, static_item_data),
        HandleMsg::UpdateItem(update_item_data) => {
            update_user_for_item(deps, env, update_item_data)
        }
        HandleMsg::RemoveItem(remove_item_data) => remove_item(deps, env, remove_item_data),
        HandleMsg::SetViewingKey { key, .. } => set_viewing_key(deps, env, key),
    }
}

fn add_new_item<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    static_item_data: StaticItemData,
) -> StdResult<HandleResponse> {
    let (static_prefix, dynamic_prefix, _dynamic_prefix_users) =
        get_category_prefixes(static_item_data.category.as_bytes())?;

    save_new_item(&mut deps.storage, static_prefix, &static_item_data)?;

    let key = sha_256(base64::encode(static_item_data.url.clone()).as_bytes());
    update_current_group_size(&mut deps.storage, &key, dynamic_prefix, 0)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::AddItem {
            status: ResponseStatus::Success,
        })?),
    })
}

fn remove_item<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    remove_item_data: RemoveItemData,
) -> StdResult<HandleResponse> {
    remove_item_data.authenticate_delete(deps, &env.message.sender)?;

    remove_item_authenticated(&remove_item_data.category, &remove_item_data.url, deps)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::RemoveItem {
            status: ResponseStatus::Success,
        })?),
    })
}

pub fn remove_item_authenticated<S: Storage, A: Api, Q: Querier>(
    category: &str,
    url: &str,
    deps: &mut Extern<S, A, Q>,
) -> Result<(), StdError> {
    let (static_prefix, dynamic_prefix, dynamic_prefix_users) =
        get_category_prefixes(category.as_bytes())?;
    let url_key = sha_256(base64::encode(url).as_bytes());

    remove_category_item(&mut deps.storage, static_prefix, url)?;
    remove_current_group_size(&mut deps.storage, dynamic_prefix, &url_key)?;

    let users_addresses =
        get_all_participating_users_addresses(&deps.storage, dynamic_prefix_users, &url_key)?;

    remove_all_category_item_users_details(&mut deps.storage, dynamic_prefix_users, &url_key)?;

    for user_address in users_addresses.iter() {
        // Remove the user's quantity object for this item (Find it by its URL)
        remove_user_item_quantity(
            &mut deps.storage,
            dynamic_prefix,
            deps.api.canonical_address(user_address)?.as_slice(),
            url,
        )?;
    }

    Ok(())
}

pub fn set_viewing_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    key: String,
) -> StdResult<HandleResponse> {
    let vk = ViewingKey(key);

    let message_sender = deps.api.canonical_address(&env.message.sender)?;
    ViewingKey::write_viewing_key(&mut deps.storage, &message_sender, &vk);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::SetViewingKey {
            status: ResponseStatus::Success,
        })?),
    })
}

#[cfg(test)]
mod tests {
    use crate::contract::init;
    use crate::msg::{
        GetItems, InitMsg, QueryMsg, UpdateItemData, UserContactData, UserItemDetails,
    };
    use crate::query::query;
    use crate::viewing_key::ViewingKey;

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{coins, from_binary, HumanAddr, InitResponse, Uint128};

    fn init_helper() -> (
        StdResult<InitResponse>,
        Extern<MockStorage, MockApi, MockQuerier>,
    ) {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env("instantiator", &coins(1000, "token"));

        let init_msg = InitMsg {
            // msg: String::from("initialized"),
        };

        (init(&mut deps, env, init_msg), deps)
    }

    fn invoke_set_viewing_key(
        deps: &mut Extern<cosmwasm_std::MemoryStorage, MockApi, MockQuerier>,
    ) -> ViewingKey {
        let vk: &str = "wefhjyr";
        let msg = HandleMsg::SetViewingKey {
            key: String::from(vk),
        };
        let handle_result = handle(deps, mock_env("bob", &[]), msg);
        assert!(
            handle_result.is_ok(),
            "handle() failed: {}",
            handle_result.err().unwrap()
        );

        // Get the viewing key of the reply to HandleMsg::CreateViewingKey
        let answer: HandleAnswer = from_binary(&handle_result.unwrap().data.unwrap()).unwrap();
        match answer {
            HandleAnswer::SetViewingKey { status } => match status {
                ResponseStatus::Failure => panic!("Failed to set the viewing key"),
                _ => {}
            },
            _ => panic!("NOPE"),
        };
        ViewingKey(String::from(vk))
    }

    fn query_history_wrong_vk(deps: Extern<cosmwasm_std::MemoryStorage, MockApi, MockQuerier>) {
        let wrong_vk_query_response = query(
            &deps,
            QueryMsg::GetItems {
                category: String::from("laptops"),
                address: HumanAddr("bob".to_string()),
                key: "wrong_vk".to_string(),
            },
        );
        let error = match wrong_vk_query_response {
            Ok(_response) => "This line should not be reached!".to_string(),
            Err(_err) => "Wrong viewing key for this address or viewing key not set".to_string(),
        };
        assert_eq!(
            error,
            "Wrong viewing key for this address or viewing key not set".to_string()
        );
    }

    fn query_category_items(
        deps: &mut Extern<cosmwasm_std::MemoryStorage, MockApi, MockQuerier>,
    ) -> StdResult<GetItems> {
        let vk = invoke_set_viewing_key(deps);
        let query_response = query(
            &*deps,
            QueryMsg::GetItems {
                address: HumanAddr("bob".to_string()),
                key: vk.0,
                category: String::from("laptops"),
            },
        )
        .unwrap();
        let category_items_data: GetItems = from_binary(&query_response)?;
        Ok(category_items_data)
    }

    fn add_new_item_for_test(
        deps: &mut Extern<cosmwasm_std::MemoryStorage, MockApi, MockQuerier>,
        env: Env,
    ) {
        let new_item_data = StaticItemData {
            name: String::from("Cool item"),
            category: String::from("laptops"),
            url: String::from("www.item.com"),
            img_url: String::from("www.image-item.com"),
            seller_address: String::from("sellerAddress"),
            seller_email: String::from("seller@email.com"),
            price: Uint128(1000),
            wanted_price: Uint128(900),
            group_size_goal: 10,
        };
        let msg = HandleMsg::AddItem(new_item_data);
        let _res = handle(deps, env, msg).unwrap();
    }

    fn remove_item_for_test(
        deps: &mut Extern<cosmwasm_std::MemoryStorage, MockApi, MockQuerier>,
        env: Env,
    ) {
        let remove_msg = RemoveItemData {
            category: String::from("laptops"),
            url: String::from("www.item.com"),
            verification_key: String::from("wefhjyr"),
        };
        let msg = HandleMsg::RemoveItem(remove_msg);
        let _res = handle(deps, env, msg).unwrap();
    }

    fn create_update_msg(quantity: u32) -> UpdateItemData {
        let user_details = UserItemDetails {
            account_address: HumanAddr(String::from("bob")),
            contact_data: UserContactData {
                delivery_address: String::from("user delivery address"),
                email: String::from("user@email.com"),
            },
            quantity: quantity,
        };
        let update_item_data = UpdateItemData {
            category: String::from("laptops"),
            url: String::from("www.item.com"),
            user_details,
        };
        update_item_data
    }

    fn assert_fetched_data_after_update(
        fetched_data: GetItems,
        expected_len: usize,
        expected_quantity: u32,
        expected_group_size: u32,
    ) {
        assert_eq!(fetched_data.user_items.len(), expected_len);
        assert_eq!(fetched_data.user_items[0].url, String::from("www.item.com"));
        assert_eq!(fetched_data.user_items[0].quantity, expected_quantity);
        assert_eq!(
            fetched_data.contact_data.unwrap().email,
            String::from("user@email.com")
        );
        assert_eq!(fetched_data.items[0].static_data.price, Uint128(1000));
        assert_eq!(
            fetched_data.items[0].current_group_size,
            expected_group_size
        );
        assert_eq!(fetched_data.status, ResponseStatus::Success);
    }

    #[test]
    fn test_init_sanity() {
        let (init_result, _deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );
    }

    #[test]
    fn test_set_viewing_key() {
        // Initialize the contract
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        // Compute the viewing key
        let key = invoke_set_viewing_key(&mut deps);

        // Get the viewing key from the storage
        let bob_canonical = deps
            .api
            .canonical_address(&HumanAddr("bob".to_string()))
            .unwrap();
        let saved_vk = ViewingKey::read_viewing_key(&deps.storage, &bob_canonical).unwrap();

        // Verify that the key in the storage is the same as the key from HandleAnswer::CreateViewingKey
        assert!(key.check_viewing_key(saved_vk.as_slice()));
    }

    #[test]
    fn test_add_new_item() -> StdResult<()> {
        // Initialize the contract
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        let env = mock_env("bob", &coins(2, "token"));
        add_new_item_for_test(&mut deps, env);

        // Query the user's transactions history using their viewing key
        let fetched_data = query_category_items(&mut deps)?;
        assert_eq!(fetched_data.items[0].static_data.price, Uint128(1000));
        assert_eq!(fetched_data.items[0].current_group_size, 0);

        // // Now try to hack into bob's account using the wrong key - and fail
        query_history_wrong_vk(deps);
        Ok(())
    }

    #[test]
    fn test_update_new_user_item_goal_not_reached() -> StdResult<()> {
        // Initialize the contract
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        let env = mock_env("bob", &coins(2, "token"));
        let env_clone = env.clone();
        add_new_item_for_test(&mut deps, env);

        let update_item_data = create_update_msg(1);
        let msg = HandleMsg::UpdateItem(update_item_data);
        let _res = handle(&mut deps, env_clone, msg).unwrap();

        let fetched_data = query_category_items(&mut deps)?;
        assert_fetched_data_after_update(fetched_data, 1, 1, 1);

        // // Now try to hack into bob's account using the wrong key - and fail
        query_history_wrong_vk(deps);
        Ok(())
    }

    #[test]
    fn test_update_new_user_item_goal_reached() -> StdResult<()> {
        // Initialize the contract
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        let env = mock_env("bob", &coins(2, "token"));
        let env_clone = env.clone();
        add_new_item_for_test(&mut deps, env);

        let update_item_data = create_update_msg(10);
        let msg = HandleMsg::UpdateItem(update_item_data);
        let _res = handle(&mut deps, env_clone, msg).unwrap();

        let fetched_data = query_category_items(&mut deps)?;
        assert_eq!(fetched_data.user_items.len(), 0);
        assert_eq!(fetched_data.contact_data, None);

        assert_eq!(fetched_data.items.len(), 0);
        assert_eq!(fetched_data.status, ResponseStatus::Success);

        // // Now try to hack into bob's account using the wrong key - and fail
        query_history_wrong_vk(deps);
        Ok(())
    }

    #[test]
    fn test_update_existing_item_goal_not_reached() -> StdResult<()> {
        // Initialize the contract
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        let env = mock_env("bob", &coins(2, "token"));
        let env_clone = env.clone();
        let env_clone2 = env.clone();
        add_new_item_for_test(&mut deps, env);

        let update_item_data = create_update_msg(1);
        let msg = HandleMsg::UpdateItem(update_item_data);
        let _res = handle(&mut deps, env_clone, msg).unwrap();

        let update_item_data2 = create_update_msg(5);
        let msg2 = HandleMsg::UpdateItem(update_item_data2);
        let _res = handle(&mut deps, env_clone2, msg2).unwrap();

        let fetched_data = query_category_items(&mut deps)?;
        assert_fetched_data_after_update(fetched_data, 1, 5, 5);

        // // Now try to hack into bob's account using the wrong key - and fail
        query_history_wrong_vk(deps);
        Ok(())
    }

    #[test]
    fn test_update_existing_item_goal_reached() -> StdResult<()> {
        // Initialize the contract
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        let env = mock_env("bob", &coins(2, "token"));
        let env_clone = env.clone();
        let env_clone2 = env.clone();
        add_new_item_for_test(&mut deps, env);

        let update_item_data = create_update_msg(1);
        let msg = HandleMsg::UpdateItem(update_item_data);
        let _res = handle(&mut deps, env_clone, msg).unwrap();

        let update_item_data2 = create_update_msg(10);
        let msg2 = HandleMsg::UpdateItem(update_item_data2);
        let _res = handle(&mut deps, env_clone2, msg2).unwrap();

        let fetched_data = query_category_items(&mut deps)?;
        assert_eq!(fetched_data.user_items.len(), 0);
        assert_eq!(fetched_data.contact_data, None);

        assert_eq!(fetched_data.items.len(), 0);
        assert_eq!(fetched_data.status, ResponseStatus::Success);

        // Now try to hack into bob's account using the wrong key - and fail
        query_history_wrong_vk(deps);
        Ok(())
    }

    #[test]
    fn test_update_existing_item_reduce_quantity_partially() -> StdResult<()> {
        // Initialize the contract
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        let env = mock_env("bob", &coins(2, "token"));
        let env_clone = env.clone();
        let env_clone2 = env.clone();
        add_new_item_for_test(&mut deps, env);

        let update_item_data = create_update_msg(5);
        let msg = HandleMsg::UpdateItem(update_item_data);
        let _res = handle(&mut deps, env_clone, msg).unwrap();

        let update_item_data2 = create_update_msg(2);
        let msg2 = HandleMsg::UpdateItem(update_item_data2);
        let _res = handle(&mut deps, env_clone2, msg2).unwrap();

        let fetched_data = query_category_items(&mut deps)?;
        assert_fetched_data_after_update(fetched_data, 1, 2, 2);

        // // Now try to hack into bob's account using the wrong key - and fail
        query_history_wrong_vk(deps);
        Ok(())
    }

    #[test]
    fn test_update_existing_item_reduce_quantity_completely() -> StdResult<()> {
        // Initialize the contract
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        let env = mock_env("bob", &coins(2, "token"));
        let env_clone = env.clone();
        let env_clone2 = env.clone();
        add_new_item_for_test(&mut deps, env);

        let update_item_data = create_update_msg(5);
        let msg = HandleMsg::UpdateItem(update_item_data);
        let _res = handle(&mut deps, env_clone, msg).unwrap();

        let update_item_data2 = create_update_msg(0);
        let msg2 = HandleMsg::UpdateItem(update_item_data2);
        let _res = handle(&mut deps, env_clone2, msg2).unwrap();

        let fetched_data = query_category_items(&mut deps)?;
        assert_eq!(fetched_data.user_items.len(), 0);
        assert_eq!(fetched_data.contact_data, None);

        assert_eq!(fetched_data.items.len(), 1);
        assert_eq!(fetched_data.status, ResponseStatus::Success);

        // // Now try to hack into bob's account using the wrong key - and fail
        query_history_wrong_vk(deps);
        Ok(())
    }

    #[test]
    fn test_remove_item() -> StdResult<()> {
        // Initialize the contract
        let (init_result, mut deps) = init_helper();
        assert!(
            init_result.is_ok(),
            "Init failed: {}",
            init_result.err().unwrap()
        );

        let env = mock_env("bob", &coins(2, "token"));
        let env2 = env.clone();
        add_new_item_for_test(&mut deps, env);

        // Query the user's transactions history using their viewing key
        let fetched_data = query_category_items(&mut deps)?;
        assert_eq!(fetched_data.items[0].static_data.price, Uint128(1000));
        assert_eq!(fetched_data.items[0].current_group_size, 0);

        remove_item_for_test(&mut deps, env2);
        let fetched_data2 = query_category_items(&mut deps)?;
        assert_eq!(fetched_data2.user_items.len(), 0);
        assert_eq!(fetched_data2.contact_data, None);

        assert_eq!(fetched_data2.items.len(), 0);
        assert_eq!(fetched_data2.status, ResponseStatus::Success);

        // // Now try to hack into bob's account using the wrong key - and fail
        query_history_wrong_vk(deps);
        Ok(())
    }
}
