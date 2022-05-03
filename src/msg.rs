use cosmwasm_std::{Api, Extern, HumanAddr, Querier, StdError, StdResult, Storage, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    // User supplied entropy string for pseudorandom number generator seed
// pub prng_seed: String,
// /// The entropy for creating the viewing key to be used by all users of the application
// pub entropy: String,
// pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserContactData {
    pub email: String,
    pub delivery_address: String,
} // regular store

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserItemDetails {
    pub account_address: HumanAddr,
    pub contact_data: UserContactData, // Elad: Option<>
    pub quantity: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StaticItemData {
    pub name: String,
    pub category: String, // Elad: Might be redundant.
    pub url: String,
    pub img_url: String,
    pub seller_address: String,
    pub seller_email: String,
    pub price: Uint128,
    pub wanted_price: Uint128,
    pub group_size_goal: u32,
} // Elad: authenticate category

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateItemData {
    /// url is the unique identifier of the product (could be also the creator address)
    pub category: String,
    pub url: String,
    pub user_details: UserItemDetails, //Elad: verify quantity is not 0 for a new user or seprate: AddUser/UpdateUser
                                       // Elad: refund the user if the new ammount is smaller than the old
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RemoveItemData {
    pub category: String,
    pub url: String,
    pub verification_key: String,
}
impl RemoveItemData {
    pub fn authenticate_delete<S: Storage, A: Api, Q: Querier>(
        &self,
        deps: &Extern<S, A, Q>,
        address: &HumanAddr,
    ) -> StdResult<()> {
        // Todo: Authenticate against the address of the seller! not with the user who tries to delete
        let vk = ViewingKey(self.verification_key.clone());

        let canonical_addr = deps.api.canonical_address(address)?;

        let expected_key = ViewingKey::read_viewing_key(&deps.storage, &canonical_addr);

        if expected_key.is_none() {
            // Checking the key will take significant time. We don't want to exit immediately if it isn't set
            // in a way which will allow to time the command and determine if a viewing key doesn't exist
            vk.check_viewing_key(&[0u8; VIEWING_KEY_SIZE]);
            Err(StdError::generic_err("Authentication failed"))
        } else if vk.check_viewing_key(expected_key.unwrap().as_slice()) {
            Ok(())
        } else {
            Err(StdError::generic_err("Authentication failed"))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ItemData {
    pub static_data: StaticItemData,
    pub current_group_size: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserProductQuantity {
    pub url: String,
    pub quantity: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    AddItem(StaticItemData),
    UpdateItem(UpdateItemData),
    RemoveItem(RemoveItemData),
    SetViewingKey { key: String },
}

/// Responses from handle functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    AddItem { status: ResponseStatus },
    UpdateItem { status: ResponseStatus },
    RemoveItem { status: ResponseStatus },
    SetViewingKey { status: ResponseStatus },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Get all the items of a category
    GetItems {
        category: String,
        address: HumanAddr,
        key: String,
    },
}

impl QueryMsg {
    pub fn authenticate<S: Storage, A: Api, Q: Querier>(
        &self,
        deps: &Extern<S, A, Q>,
    ) -> StdResult<HumanAddr> {
        let (address, key) = match self {
            QueryMsg::GetItems { address, key, .. } => (address.clone(), ViewingKey(key.clone())),
        };

        let canonical_addr = deps.api.canonical_address(&address)?;

        let expected_key = ViewingKey::read_viewing_key(&deps.storage, &canonical_addr);

        if expected_key.is_none() {
            // Checking the key will take significant time. We don't want to exit immediately if it isn't set
            // in a way which will allow to time the command and determine if a viewing key doesn't exist
            key.check_viewing_key(&[0u8; VIEWING_KEY_SIZE]);
            Err(StdError::generic_err("Wrong viewing key"))
        } else if key.check_viewing_key(expected_key.unwrap().as_slice()) {
            Ok(address)
        } else {
            Err(StdError::generic_err("Wrong viewing key"))
        }
    }
}

/// Responses from query functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    GetItems(GetItems),
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GetItems {
    /// The list of items of the category (provided in QueryMsg::GetItems::category)
    pub items: Vec<ItemData>,
    /// The list of items the user participates in, if any
    pub user_items: Vec<UserProductQuantity>,
    /// The contact data the user added when applied to an item
    pub contact_data: Option<UserContactData>,
    pub status: ResponseStatus,
}
