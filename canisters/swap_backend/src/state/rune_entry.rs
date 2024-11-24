use candid::{CandidType, Decode, Encode};
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use serde::Deserialize;

use crate::types::RuneId;

use super::{
    memory::{Memory, MemoryIds},
    read_memory_manager,
};

#[derive(CandidType, Deserialize)]
pub struct RuneEntry {
    pub divisibility: u8,
}

impl Storable for RuneEntry {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub type RuneEntries = StableBTreeMap<RuneId, RuneEntry, Memory>;

pub fn init_rune_entries() -> RuneEntries {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::RuneEntries.into());
        RuneEntries::init(memory)
    })
}
