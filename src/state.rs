use secret_toolkit::{
    serialization::{Bincode2, Serde},
    storage::{AppendStore, AppendStoreMut},
};
use serde::Serialize;

use crate::msg::{
    DynamicItemData, ResponseStatus, StaticItemData, UserContactData, UserItemDetails,
    UserProductData,
};

use cosmwasm_std::{HumanAddr, ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};
const PREFIX_LAPTOPS_STATIC: &[u8] = b"laptops-static";
const PREFIX_LAPTOPS_DYNAMIC: &[u8] = b"laptops-dynamic";

const PREFIX_KEYBOARDS_STATIC: &[u8] = b"keyboards-static";
const PREFIX_KEYBOARDS_DYNAMIC: &[u8] = b"keyboards-dynamic";

const PREFIX_MOUSES_STATIC: &[u8] = b"mouses-static";
const PREFIX_MOUSES_DYNAMIC: &[u8] = b"mouses-dynamic";

const PREFIX_MOUSE_PADS_STATIC: &[u8] = b"mouse-pads-static";
const PREFIX_MOUSE_PADS_DYNAMIC: &[u8] = b"mouse-pads-dynamic";

const LAPTOPS: &[u8] = b"laptops";
const KEYBOARDS: &[u8] = b"keyboards";
const MOUSES: &[u8] = b"mouses";
const MOUSE_PADS: &[u8] = b"mouse-pads";

pub fn get_category_prefixes(category: &[u8]) -> StdResult<(&[u8], &[u8])> {
    match category {
        LAPTOPS => Ok((PREFIX_LAPTOPS_STATIC, PREFIX_LAPTOPS_DYNAMIC)),
        KEYBOARDS => Ok((PREFIX_KEYBOARDS_STATIC, PREFIX_KEYBOARDS_DYNAMIC)),
        MOUSES => Ok((PREFIX_MOUSES_STATIC, PREFIX_MOUSES_DYNAMIC)),
        MOUSE_PADS => Ok((PREFIX_MOUSE_PADS_STATIC, PREFIX_MOUSE_PADS_DYNAMIC)),
        _ => Err(StdError::generic_err("No such category!")),
    }
}
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct ItemUsers {
//     pub users_data: Vec<UserItemDetails>,
// }
// [CATEGORY_STATIC, url] ==> static item data
// [CATEGORY_DYNAMIC, url] ==> dynamic item data
// [CATEGORY_USERS_DATA, url] ==> UserItemDetails /// multilevel mut store

pub fn save_new_item<S: Storage>(
    storage: &mut S,
    prefix_static: &[u8], // key is the hash of the seller address with the item url
    value: &StaticItemData,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_static], storage);
    let mut storage = AppendStoreMut::attach_or_create(&mut storage)?;
    storage.push(value)
}

pub fn get_category_items<S: ReadonlyStorage>(
    storage: &S,
    category: &str,
) -> StdResult<(Vec<StaticItemData>, ResponseStatus)> {
    let (static_prefix, dynamic_prefix) = get_category_prefixes(category.as_bytes())?;

    let store = ReadonlyPrefixedStorage::multilevel(&[static_prefix], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<StaticItemData, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok((vec![], ResponseStatus::Success));
    };

    let items_static_data: StdResult<Vec<StaticItemData>> = store
        .iter()
        .map(|itemData| itemData.and_then(|x| Ok(x)))
        .collect();
    Ok((items_static_data.unwrap(), ResponseStatus::Success))
}

// Elad: can be done with regular storage: the key will be sha256(url + categoryPrefixDynamic)
pub fn update_item_quantity<S: Storage>(
    storage: &mut S,
    key: &[u8],
    prefix_dynamic: &[u8], // key is the hash of the seller address with the item url
    value: u32,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic, key], storage);
    let mut storage = AppendStoreMut::attach_or_create(&mut storage)?;
    storage.push(&value)
}

pub fn get_category_item_dynamic_data<S: ReadonlyStorage>(
    storage: &S,
    category: &str,
    key: &[u8],
) -> StdResult<(DynamicItemData, ResponseStatus)> {
    let (static_prefix, dynamic_prefix) = get_category_prefixes(category.as_bytes())?;

    let store = ReadonlyPrefixedStorage::multilevel(&[dynamic_prefix, key], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<DynamicItemData, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Err(StdError::generic_err("Found no quantity count!"));
    };

    let item_dynamic_data: StdResult<Vec<DynamicItemData>> = store
        .iter()
        .map(|dynamic_data| dynamic_data.and_then(|x| Ok(x)))
        .collect();

    let unwraped = item_dynamic_data.unwrap();
    if unwraped.len() > 0 {
        return Ok((unwraped[0].clone(), ResponseStatus::Success));
    }

    return Err(StdError::generic_err("Found no quantity count!"));
}

// for_address.as_slice()
// Used both for url => Vec<UserItemDetails>
//     and  userAddr => Vec<UserProductData>
pub fn save_category_element<T: Serialize, S: Storage>(
    storage: &mut S,
    key: &[u8],
    prefix_dynamic: &[u8], // key is the hash of the seller address with the item url
    value: &T,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic, key], storage);
    let mut storage = AppendStoreMut::attach_or_create(&mut storage)?;
    storage.push(&Bincode2::serialize(value)?)
}

pub fn get_ctegory_user_items<S: ReadonlyStorage>(
    storage: &S,
    category: &str,
    key: &[u8],
) -> StdResult<(Vec<UserProductData>, ResponseStatus)> {
    let (static_prefix, dynamic_prefix) = get_category_prefixes(category.as_bytes())?;

    let store = ReadonlyPrefixedStorage::multilevel(&[dynamic_prefix, key], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<UserProductData, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok((vec![], ResponseStatus::Success));
    };

    let user_category_items: StdResult<Vec<UserProductData>> = store
        .iter()
        .map(|itemData| itemData.and_then(|x| Ok(x)))
        .collect();
    Ok((user_category_items.unwrap(), ResponseStatus::Success))
}

pub fn get_category_item_user_contact_data<S: ReadonlyStorage>(
    storage: &S,
    category: &str,
    key: &[u8],
    user_address: &HumanAddr,
) -> StdResult<(UserContactData, ResponseStatus)> {
    let (static_prefix, dynamic_prefix) = get_category_prefixes(category.as_bytes())?;

    let store = ReadonlyPrefixedStorage::multilevel(&[dynamic_prefix, key], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<UserItemDetails, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Err(StdError::generic_err("Found no result!"));
    };

    for user_item_details in store.iter() {
        let unwrapped = user_item_details.unwrap();
        if unwrapped.account_address == *user_address {
            return Ok((unwrapped.contact_data.clone(), ResponseStatus::Success));
        }
    }
    Err(StdError::generic_err("Found no result!"))
}
