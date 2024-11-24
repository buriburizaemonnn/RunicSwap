use std::collections::HashSet;

use candid::{CandidType, Decode, Encode, Principal};
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use serde::Deserialize;

use super::{
    memory::{Memory, MemoryIds},
    read_memory_manager,
};

#[derive(CandidType, Deserialize, Default)]
pub struct UserDetails {
    pub lp_positions: HashSet<u128>,
    pub icp_balance: u64,
}

impl Storable for UserDetails {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub type UserMap = StableBTreeMap<Principal, UserDetails, Memory>;

pub fn init_user_mapping() -> UserMap {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::UserMapping.into());
        UserMap::init(memory)
    })
}
