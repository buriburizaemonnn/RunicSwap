pub mod btc;

pub mod ic;

use btc::account_to_p2pkh_address;
use candid::{CandidType, Principal};
use ic_ledger_types::{AccountIdentifier, Subaccount};
use icrc_ledger_types::icrc1::account::Account;
use tiny_keccak::{Hasher, Sha3};

#[derive(CandidType)]
pub struct Addresses {
    pub icrc1: Account,
    pub icrc1_string: String,
    pub account_identifier: AccountIdentifier,
    pub account_identifier_string: String,
    pub bitcoin: String,
}

pub fn principal_to_subaccount(principal: &Principal) -> [u8; 32] {
    let mut hash = [0u8; 32];
    let mut hasher = Sha3::v256();
    hasher.update(principal.as_slice());
    hasher.finalize(&mut hash);
    hash
}

pub fn generate_subaccount_for_pool(pool_id: u128, created_at: u64) -> [u8; 32] {
    let mut hash = [0u8; 32];
    let mut hasher = Sha3::v256();
    hasher.update(&pool_id.to_be_bytes());
    hasher.update(ic_cdk::id().as_slice());
    hasher.update(&created_at.to_be_bytes());
    hasher.finalize(&mut hash);
    hash
}

impl From<&Principal> for Addresses {
    fn from(principal: &Principal) -> Self {
        let subaccount = principal_to_subaccount(principal);
        Self::from(subaccount)
    }
}

impl From<[u8; 32]> for Addresses {
    fn from(subaccount: [u8; 32]) -> Self {
        let id = ic_cdk::id();
        let account_identifier = AccountIdentifier::new(&id, &Subaccount(subaccount));
        let account = Account {
            owner: id,
            subaccount: Some(subaccount),
        };
        let bitcoin = account_to_p2pkh_address(&account);
        Self {
            icrc1_string: account.to_string(),
            icrc1: account,
            account_identifier_string: account_identifier.to_string(),
            account_identifier,
            bitcoin,
        }
    }
}
