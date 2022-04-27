use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    // User supplied entropy string for pseudorandom number generator seed
    // pub prng_seed: String,
    // /// The entropy for creating the viewing key to be used by all users of the application
    // pub entropy: String,
    msg: String,
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
pub struct UserQuantity {
    pub quantity: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserItemDetails {
    pub account_address: HumanAddr,
    pub contact_data: UserContactData, // Elad: Option<>
    pub quantity: UserQuantity,
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DynamicItemData {
    pub current_group_size: u32, // Elad: Change to current_num_units
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ItemData {
    static_data: StaticItemData,
    dynamic_data: DynamicItemData,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserProductData {
    pub url: String,
    pub quantity: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    AddItem {
        name: String,
        category: String,
        url: String,
        img_url: String,
        seller_address: HumanAddr,
        seller_email: String,
        price: Uint128,
        wanted_price: Uint128,
        group_size_goal: u32,
    },
    UpdateItem {
        /// url is the unique identifier of the product (could be also the creator address)
        category: String,
        url: String,
        user_details: UserItemDetails, //Elad: verify quantity is not 0 for a new user or seprate: AddUser/UpdateUser
                                       // Elad: refund the user if the new ammount is smaller than the old
    },
    RemoveItem {
        category: String,
        url: String,
        verification_key: String,
    },
    SetViewingKey {
        key: String,
    },
}

/// Responses from handle functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    AddItem { status: String },
    UpdateItem { status: String },
    RemoveItem { status: String },
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
/// Responses from query functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    GetItems {
        items: Vec<ItemData>,
        user_items: Vec<UserProductData>,
        contact_data: UserContactData,
        status: String,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CountResponse {
    pub count: i32,
}
