use candid::{CandidType, Principal};
use ic_ledger_types::AccountIdentifier;
use icrc_ledger_types::icrc1::account::Account;
use serde::Deserialize;
use tiny_keccak::{Hasher, Sha3};

use crate::chain::btc::account_to_p2pkh_address;

#[derive(CandidType, Deserialize)]
pub struct Addresses {
    pub icrc1: Account,
    pub icrc1_string: String,
    pub account_identifier: AccountIdentifier,
    pub account_identifier_string: String,
    pub bitcoin: String,
}

impl From<[u8; 32]> for Addresses {
    fn from(subaccount: [u8; 32]) -> Self {
        let id = ic_cdk::id();
        let account_identifier =
            AccountIdentifier::new(&id, &ic_ledger_types::Subaccount(subaccount));
        let account = Account {
            owner: id,
            subaccount: Some(subaccount),
        };
        let bitcoin = account_to_p2pkh_address(&account);
        Addresses {
            icrc1: account,
            icrc1_string: account.to_string(),
            account_identifier,
            account_identifier_string: account_identifier.to_string(),
            bitcoin,
        }
    }
}

impl From<&Principal> for Addresses {
    fn from(value: &Principal) -> Self {
        let mut hash = [0u8; 32];
        let mut hasher = Sha3::v256();
        hasher.update(value.as_slice());
        hasher.finalize(&mut hash);
        Self::from(hash)
    }
}

pub fn gen_subaccount_for_pool(id: u128, time: u64) -> [u8; 32] {
    let mut hash = [0u8; 32];
    let mut hasher = Sha3::v256();
    hasher.update(&id.to_be_bytes());
    hasher.update(&time.to_be_bytes());
    hasher.update(ic_cdk::id().as_slice());
    hasher.finalize(&mut hash);
    hash
}
