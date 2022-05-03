use secret_toolkit::{
    serialization::{Bincode2, Serde},
    storage::{AppendStore, AppendStoreMut},
};
use serde::Serialize;

use crate::msg::{StaticItemData, UpdateItemData, UserItemDetails, UserProductQuantity};

use cosmwasm_std::{HumanAddr, ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};
const PREFIX_LAPTOPS_STATIC: &[u8] = b"laptops-static";
const PREFIX_LAPTOPS_DYNAMIC: &[u8] = b"laptops-dynamic";
const PREFIX_LAPTOPS_DYNAMIC_USERS: &[u8] = b"laptops-dynamic-users";

const PREFIX_KEYBOARDS_STATIC: &[u8] = b"keyboards-static";
const PREFIX_KEYBOARDS_DYNAMIC: &[u8] = b"keyboards-dynamic";
const PREFIX_KEYBOARDS_DYNAMIC_USERS: &[u8] = b"keyboards-dynamic-users";

const PREFIX_MOUSES_STATIC: &[u8] = b"mouses-static";
const PREFIX_MOUSES_DYNAMIC: &[u8] = b"mouses-dynamic";
const PREFIX_MOUSES_DYNAMIC_USERS: &[u8] = b"mouses-dynamic-users";

const PREFIX_MOUSE_PADS_STATIC: &[u8] = b"mouse-pads-static";
const PREFIX_MOUSE_PADS_DYNAMIC: &[u8] = b"mouse-pads-dynamic";
const PREFIX_MOUSE_PADS_DYNAMIC_USERS: &[u8] = b"mouse-pads-dynamic-users";

const LAPTOPS: &[u8] = b"laptops";
const KEYBOARDS: &[u8] = b"keyboards";
const MOUSES: &[u8] = b"mouses";
const MOUSE_PADS: &[u8] = b"mouse-pads";

pub fn get_category_prefixes(category: &[u8]) -> StdResult<(&[u8], &[u8], &[u8])> {
    match category {
        LAPTOPS => Ok((
            PREFIX_LAPTOPS_STATIC,
            PREFIX_LAPTOPS_DYNAMIC,
            PREFIX_LAPTOPS_DYNAMIC_USERS,
        )),
        KEYBOARDS => Ok((
            PREFIX_KEYBOARDS_STATIC,
            PREFIX_KEYBOARDS_DYNAMIC,
            PREFIX_KEYBOARDS_DYNAMIC_USERS,
        )),
        MOUSES => Ok((
            PREFIX_MOUSES_STATIC,
            PREFIX_MOUSES_DYNAMIC,
            PREFIX_MOUSES_DYNAMIC_USERS,
        )),
        MOUSE_PADS => Ok((
            PREFIX_MOUSE_PADS_STATIC,
            PREFIX_MOUSE_PADS_DYNAMIC,
            PREFIX_MOUSE_PADS_DYNAMIC_USERS,
        )),
        _ => Err(StdError::generic_err("No such category!")),
    }
}

// [CATEGORY_STATIC] ==> static item data
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
    prefix_static: &[u8],
) -> StdResult<Vec<StaticItemData>> {
    let store = ReadonlyPrefixedStorage::multilevel(&[prefix_static], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<StaticItemData, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok(vec![]);
    };

    let items_static_data: StdResult<Vec<StaticItemData>> = store.iter().collect();
    items_static_data
    // Ok(items_static_data?)
}

// remove_category_items(&mut deps.storage, &static_prefix, &update_item_data.url)?;
pub fn remove_category_item<S: Storage>(
    storage: &mut S,
    prefix_static: &[u8], // key is the hash of the seller address with the item url
    url: &str,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_static], storage);
    let mut storage = AppendStoreMut::<StaticItemData, _, _>::attach_or_create(&mut storage)?;

    match storage.len() {
        0 => Ok(()),
        len => {
            let mut c: u32 = 0;
            for item_data in storage.iter() {
                let unwrapped = item_data?;
                if unwrapped.url == url {
                    break;
                }
                c += 1;
            }
            if c == len {
                return Err(StdError::generic_err("Item to remove wasn't found"));
            };

            if c == len - 1 {
                storage.pop()?;
                return Ok(());
            }

            let last = storage.get_at(len - 1)?;
            storage.set_at(c, &last)?;
            storage.pop()?;

            Ok(())
        }
    }
}

pub fn get_category_item_by_url<S: ReadonlyStorage>(
    storage: &S,
    prefix_static: &[u8],
    url: &str,
) -> StdResult<Option<StaticItemData>> {
    let store = ReadonlyPrefixedStorage::multilevel(&[prefix_static], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<StaticItemData, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok(None);
    };

    for category_item in store.iter() {
        let unwrapped = category_item?;
        if unwrapped.url == url {
            return Ok(Some(unwrapped));
        }
    }

    Ok(None)
}

// Elad: can be done with regular storage: the key will be sha256(url + categoryPrefixDynamic)
// [CATEGORY_DYNAMIC, url] ==> dynamic item data
pub fn update_current_group_size<S: Storage>(
    storage: &mut S,
    key: &[u8],
    prefix_dynamic: &[u8], // key is the hash of the seller address with the item url
    value: u32,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic, key], storage);
    let mut storage = AppendStoreMut::attach_or_create(&mut storage)?;
    if !storage.is_empty() {
        storage.pop()?;
    }
    storage.push(&value)
}
pub fn get_category_item_group_size<S: ReadonlyStorage>(
    storage: &S,
    prefix_dynamic: &[u8],
    key: &[u8],
) -> StdResult<Option<u32>> {
    let store = ReadonlyPrefixedStorage::multilevel(&[prefix_dynamic, key], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<u32, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok(None);
    };

    let item_dynamic_data: StdResult<Vec<u32>> = store.iter().collect();

    let unwraped = item_dynamic_data?;
    if !unwraped.is_empty() {
        return Ok(Some(unwraped[0]));
    }

    Ok(None)
}

pub fn remove_current_group_size<S: Storage>(
    storage: &mut S,
    prefix_dynamic: &[u8],
    key: &[u8],
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic, key], storage);
    let mut storage = AppendStoreMut::<u32, _, _>::attach_or_create(&mut storage)?;

    // This store will always have one item for a product
    if (storage.len() > 0) {
        storage.pop()?;
    }

    Ok(())
}

// for_address.as_slice()
// Used both for url => Vec<UserItemDetails>
//     and  userAddr => Vec<UserProductQuantity>
// [CATEGORY_USERS_DATA, url/userAddress] ==> UserItemDetails
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

pub fn save_category_element_user<S: Storage>(
    storage: &mut S,
    key: &[u8],
    prefix_dynamic: &[u8], // key is the hash of the seller address with the item url
    value: &UserProductQuantity,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic, key], storage);
    let mut storage = AppendStoreMut::attach_or_create(&mut storage)?;
    storage.push(value)
}

pub fn save_category_element_user_item_details<S: Storage>(
    storage: &mut S,
    key: &[u8],
    prefix_dynamic_users: &[u8], // key is the hash of the seller address with the item url
    value: &UserItemDetails,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic_users, key], storage);
    let mut storage = AppendStoreMut::attach_or_create(&mut storage)?;
    storage.push(value)
}

// remove_user_item_quantity(&mut deps.storage, &prefix_dynamic, &key, &update_item_data.url)?;
pub fn remove_user_item_quantity<S: Storage>(
    storage: &mut S,
    prefix_dynamic: &[u8],
    key: &[u8],
    url: &str,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic, key], storage);
    let mut storage = AppendStoreMut::<UserProductQuantity, _, _>::attach_or_create(&mut storage)?;

    match storage.len() {
        0 => Ok(()),
        len => {
            let mut c: u32 = 0;
            for user_item_quantity in storage.iter() {
                let unwrapped = user_item_quantity?;
                if unwrapped.url == url {
                    break;
                }
                c += 1;
            }
            if c == len {
                return Err(StdError::generic_err("Item to remove wasn't found"));
            };

            if c == len - 1 {
                storage.pop()?;
                return Ok(());
            }

            let last = storage.get_at(len - 1)?;
            storage.set_at(c, &last)?;
            storage.pop()?;

            Ok(())
        }
    }
}

pub fn update_user_item_quantity<S: Storage>(
    storage: &mut S,
    prefix_dynamic: &[u8],
    key: &[u8],
    update_item_data: &UpdateItemData,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic, key], storage);
    let mut storage = AppendStoreMut::<UserProductQuantity, _, _>::attach_or_create(&mut storage)?;

    match storage.len() {
        0 => Ok(()),
        len => {
            let mut c: u32 = 0;
            for user_item_quantity in storage.iter() {
                let unwrapped = user_item_quantity?;
                if unwrapped.url == update_item_data.url {
                    break;
                }
                c += 1;
            }
            if c == len {
                return Err(StdError::generic_err("Item to update wasn't found"));
            };

            // if c == len - 1 {
            //     storage.pop()?;
            //     return Ok(());
            // }

            // let last = storage.get_at(len - 1)?;
            let updated_user_item_quantity = UserProductQuantity {
                quantity: update_item_data.user_details.quantity,
                url: update_item_data.url.clone(),
            };
            storage.set_at(c, &updated_user_item_quantity)?;
            // storage.pop()?;

            Ok(())
        }
    }
}

// [CATEGORY_USERS_DATA, userAddress] ==> Vec<UserProductQuantity>
pub fn get_ctegory_user_items_quantities<S: ReadonlyStorage>(
    storage: &S,
    prefix_dynamic: &[u8],
    key: &[u8],
) -> StdResult<Vec<UserProductQuantity>> {
    let store = ReadonlyPrefixedStorage::multilevel(&[prefix_dynamic, key], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<UserProductQuantity, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok(vec![]);
    };

    let user_category_items: StdResult<Vec<UserProductQuantity>> = store.iter().collect();
    user_category_items
    // Ok(user_category_items?)
}

// Elad: unify this function with get_category_item_by_url()
// [CATEGORY_USERS_DATA, userAddress] ==> Vec<UserProductQuantity>
pub fn get_category_user_items_quantities_by_url<S: ReadonlyStorage>(
    storage: &S,
    prefix_dynamic: &[u8],
    key: &[u8],
    url: &str,
) -> StdResult<Option<UserProductQuantity>> {
    let store = ReadonlyPrefixedStorage::multilevel(&[prefix_dynamic, key], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<UserProductQuantity, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok(None);
    };

    for user_product_data in store.iter() {
        let unwrapped = user_product_data?;
        if unwrapped.url == url {
            return Ok(Some(unwrapped));
        }
    }

    Ok(None)
}

pub fn get_category_item_user_details<S: ReadonlyStorage>(
    storage: &S,
    prefix_dynamic_users: &[u8],
    key: &[u8],
    user_address: &HumanAddr,
) -> StdResult<Option<UserItemDetails>> {
    // Elad: separate between [dynamic, url] of quantity and useritemdetails
    let store = ReadonlyPrefixedStorage::multilevel(&[prefix_dynamic_users, key], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<UserItemDetails, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok(None);
    };

    for user_item_details in store.iter() {
        let unwrapped = user_item_details?;
        if unwrapped.account_address == *user_address {
            return Ok(Some(unwrapped));
        }
    }
    Ok(None)
}

pub fn get_all_participating_users_addresses<S: ReadonlyStorage>(
    storage: &S,
    prefix_dynamic_users: &[u8],
    key: &[u8],
) -> StdResult<Vec<HumanAddr>> {
    let store = ReadonlyPrefixedStorage::multilevel(&[prefix_dynamic_users, key], storage);

    // Try to access the storage of transfers for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = AppendStore::<UserItemDetails, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok(vec![]);
    };

    let mut users_addresses = Vec::new();
    for user_item_details in store.iter() {
        let unwrapped = user_item_details?;
        users_addresses.push(unwrapped.account_address)
    }
    Ok(users_addresses)
}

// remove_category_item_user_details(&mut deps.storage, &prefix_dynamic_users, &key, &env.message.sender)?;
pub fn remove_category_item_user_details<S: Storage>(
    storage: &mut S,
    prefix_dynamic_users: &[u8],
    key: &[u8],
    user_address: &HumanAddr,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic_users, key], storage);
    let mut storage = AppendStoreMut::<UserItemDetails, _, _>::attach_or_create(&mut storage)?;

    match storage.len() {
        0 => Ok(()),
        len => {
            let mut c: u32 = 0;
            for user_item_quantity in storage.iter() {
                let unwrapped = user_item_quantity?;
                if unwrapped.account_address == *user_address {
                    break;
                }
                c += 1;
            }
            if c == len {
                return Err(StdError::generic_err("Item to remove wasn't found"));
            };

            if c == len - 1 {
                storage.pop()?;
                return Ok(());
            }

            let last = storage.get_at(len - 1)?;
            storage.set_at(c, &last)?;
            storage.pop()?;

            Ok(())
        }
    }
}

pub fn remove_all_category_item_users_details<S: Storage>(
    storage: &mut S,
    prefix_dynamic_users: &[u8],
    key: &[u8],
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic_users, key], storage);
    // storage.remove(key); // Elad: Check
    let mut storage = AppendStoreMut::<UserItemDetails, _, _>::attach_or_create(&mut storage)?;

    match storage.len() {
        0 => Ok(()),
        len => {
            for _x in 0..len {
                storage.pop()?;
            }
            Ok(())
        }
    }
}

pub fn update_category_item_user_details<S: Storage>(
    storage: &mut S,
    prefix_dynamic_users: &[u8],
    key: &[u8],
    update_item_data: &UpdateItemData,
) -> StdResult<()> {
    let mut storage = PrefixedStorage::multilevel(&[prefix_dynamic_users, key], storage);
    let mut storage = AppendStoreMut::<UserItemDetails, _, _>::attach_or_create(&mut storage)?;

    match storage.len() {
        0 => Ok(()),
        len => {
            let mut c: u32 = 0;
            for user_item_quantity in storage.iter() {
                let unwrapped = user_item_quantity?;
                if unwrapped.account_address == update_item_data.user_details.account_address {
                    break;
                }
                c += 1;
            }
            if c == len {
                return Err(StdError::generic_err("Item to update wasn't found"));
            };

            // if c == len - 1 {
            //     storage.pop()?;
            //     return Ok(());
            // }

            // let last = storage.get_at(len - 1)?;
            storage.set_at(c, &update_item_data.user_details)?;
            // storage.pop()?;

            Ok(())
        }
    }
}
