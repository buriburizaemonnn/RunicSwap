use std::collections::HashSet;

use candid::{CandidType, Decode, Encode, Principal};
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use serde::Deserialize;

use crate::memory::{Memory, MemoryIds};

use super::read_memory_manager;

#[derive(CandidType, Deserialize, Default)]
pub struct UserDetail {
    pub slippage: u8,
    pub holdings: HashSet<u128>, // stores the list of pool id
}

impl Storable for UserDetail {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub type UserMapping = StableBTreeMap<Principal, UserDetail, Memory>;

pub fn init_user_mapping() -> UserMapping {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::User.into());
        UserMapping::init(memory)
    })
}
