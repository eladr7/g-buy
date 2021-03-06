use std::fmt;

use cosmwasm_std::ReadonlyStorage;
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Storage};

use crate::utils::{create_hashed_password, ct_slice_compare};

pub const VIEWING_KEY_SIZE: usize = 32;
// pub const VIEWING_KEY_PREFIX: &str = "api_key_";
pub const PREFIX_VIEWING_KEY: &[u8] = b"viewingkey";

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ViewingKey(pub String);

impl ViewingKey {
    pub fn check_viewing_key(&self, hashed_pw: &[u8]) -> bool {
        let mine_hashed = create_hashed_password(&self.0);

        ct_slice_compare(&mine_hashed, hashed_pw)
    }

    // pub fn new(env: &Env, seed: &[u8], entropy: &[u8]) -> Self {
    //     // 16 here represents the lengths in bytes of the block height and time.
    //     let entropy_len = 16 + env.message.sender.len() + entropy.len();
    //     let mut rng_entropy = Vec::with_capacity(entropy_len);
    //     rng_entropy.extend_from_slice(&env.block.height.to_be_bytes());
    //     rng_entropy.extend_from_slice(&env.block.time.to_be_bytes());
    //     rng_entropy.extend_from_slice(env.message.sender.0.as_bytes());
    //     rng_entropy.extend_from_slice(entropy);

    //     let mut rng = Prng::new(seed, &rng_entropy);

    //     let rand_slice = rng.rand_bytes();

    //     let key = sha_256(&rand_slice);

    //     Self(VIEWING_KEY_PREFIX.to_string() + &base64::encode(key))
    // }

    pub fn to_hashed(&self) -> [u8; VIEWING_KEY_SIZE] {
        create_hashed_password(&self.0)
    }

    // pub fn as_bytes(&self) -> &[u8] {
    //     self.0.as_bytes()
    // }

    pub fn write_viewing_key<S: Storage>(store: &mut S, owner: &CanonicalAddr, key: &ViewingKey) {
        let mut user_key_store = PrefixedStorage::new(PREFIX_VIEWING_KEY, store);
        user_key_store.set(owner.as_slice(), &key.to_hashed());
    }

    pub fn read_viewing_key<S: Storage>(store: &S, owner: &CanonicalAddr) -> Option<Vec<u8>> {
        let user_key_store = ReadonlyPrefixedStorage::new(PREFIX_VIEWING_KEY, store);
        user_key_store.get(owner.as_slice())
    }
}

impl fmt::Display for ViewingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
