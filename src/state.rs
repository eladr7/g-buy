use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{UserItemDetails, UserProductData};

use cosmwasm_std::{CanonicalAddr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
pub static CONFIG_KEY: &[u8] = b"config";

pub struct ItemUsers {
    pub users_data: Vec<UserItemDetails>,
}
// [CATEGORY_STATIC, url] ==> static item data
// [CATEGORY_DYNAMIC, url] ==> dynamic item data
// [CATEGORY_USERS_DATA, url] ==> UserItemDetails /// multilevel mut store

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserProducts {
    pub products: Vec<UserProductData>,
} // mutStore
  // [CATEGORY_USER_PRODUCTS, userAddress] ==> UserProducts

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub count: i32,
    pub owner: CanonicalAddr,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}
