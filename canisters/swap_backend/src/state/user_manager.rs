use std::collections::HashSet;

use candid::CandidType;

#[derive(CandidType)]
pub struct UserHoldings {
    pub icp: u64, // icp balance
    pub holdings: HashSet<u128>,
}
