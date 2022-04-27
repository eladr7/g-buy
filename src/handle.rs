use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    msg::{HandleAnswer, HandleMsg, ResponseStatus, StaticItemData},
    state::{get_category_prefixes, save_new_item, update_item_quantity},
    viewing_key::ViewingKey,
};
use cosmwasm_std::{
    to_binary, Api, Env, Extern, HandleResponse, HumanAddr, Querier, StdResult, Storage,
    Uint128,
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
        HandleMsg::AddItem(static_item_data) => add_new_item(deps, env, static_item_data),
        HandleMsg::SetViewingKey { key, .. } => try_set_key(deps, env, key),
        // Elad::update:: if the goal is reached - transfer the money and remove the item!
        _ => {
            println!("Not impl yet");
            Ok(HandleResponse {
                messages: vec![],
                log: vec![],
                data: Some(to_binary(&HandleAnswer::AddItem {
                    status: ResponseStatus::Success,
                })?),
            })
        }
        // HandleMsg::UpdateItem((n1, n2)) => sub(deps, env, n1, n2),
        // HandleMsg::RemoveItem((n1, n2)) => mul(deps, env, n1, n2),
        
    }
}

fn add_new_item<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    static_item_data: StaticItemData,
) -> StdResult<HandleResponse> {
    // Elad: Can add authentication with the viewing key
    // let sender_address = env.message.sender;
    // let sender_canonical_address = deps.api.canonical_address(&sender_address)?;

    let (static_prefix, dynamic_prefix) =
        get_category_prefixes(static_item_data.category.as_bytes())?;

    save_new_item(&mut deps.storage, static_prefix, &static_item_data)?;

    let key = sha_256(base64::encode(static_item_data.url.clone()).as_bytes());
    update_item_quantity(&mut deps.storage, &key, dynamic_prefix, 0)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::AddItem {
            status: ResponseStatus::Success,
        })?),
    })
}

pub fn try_set_key<S: Storage, A: Api, Q: Querier>(
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
    use crate::msg::{GetItems, InitMsg, QueryMsg};
    use crate::query::query;
    use crate::viewing_key::ViewingKey;

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{coins, from_binary, InitResponse};

    fn init_helper() -> (
        StdResult<InitResponse>,
        Extern<MockStorage, MockApi, MockQuerier>,
    ) {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env("instantiator", &coins(1000, "token"));

        let init_msg = InitMsg {
            msg: String::from("initialized"),
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

        // Perform an Add operation
        let env = mock_env("bob", &coins(2, "token"));
        let suka = StaticItemData {
            name: String::from("suka"),
            category: String::from("laptops"),
            url: String::from("www.blat.com"),
            img_url: String::from("www.image-blat.com"),
            seller_address: String::from("sellerAddress"),
            seller_email: String::from("seller@email.com"),
            price: Uint128(1000),
            wanted_price: Uint128(900),
            group_size_goal: 10,
        };
        let msg = HandleMsg::AddItem(suka);
        let _res = handle(&mut deps, env, msg).unwrap();

        // Query the user's transactions history using their viewing key
        let fetched_data = query_category_items(&mut deps)?;
        assert_eq!(fetched_data.items[0].static_data.price, Uint128(1000));
        assert_eq!(fetched_data.items[0].dynamic_data.current_group_size, 0);

        // // Now try to hack into bob's account using the wrong key - and fail
        query_history_wrong_vk(deps);
        Ok(())
    }
}
