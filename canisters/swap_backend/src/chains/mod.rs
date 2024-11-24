use candid::Principal;
use ic_ledger_types::{AccountIdentifier, Subaccount};
use icrc_ledger_types::icrc1::account::Account;
use tiny_keccak::{Hasher, Sha3};

use crate::DepositAddresses;

pub mod bitcoin;
pub mod ic;

pub fn principal_to_subaccount(principal: &Principal) -> [u8; 32] {
    let mut hash = [0; 32];
    let mut hasher = Sha3::v256();
    hasher.update(principal.as_slice());
    hasher.finalize(&mut hash);
    hash
}

pub fn generate_deposit_addresses(principal: &Principal) -> DepositAddresses {
    let subaccount = principal_to_subaccount(principal);
    let canister_id = ic_cdk::id();
    let account = Account {
        owner: canister_id,
        subaccount: Some(subaccount),
    };
    let account_identifier = AccountIdentifier::new(&canister_id, &Subaccount(subaccount));
    let bitcoin = bitcoin::address::account_to_p2pkh_address(&account);
    DepositAddresses {
        account_string: account.to_string(),
        account,
        account_identifier_string: account_identifier.to_hex(),
        account_identifier,
        bitcoin,
    }
}

pub fn generate_deposit_addresses_with_subaccount(subaccount: [u8; 32]) -> DepositAddresses {
    let canister_id = ic_cdk::id();
    let account = Account {
        owner: canister_id,
        subaccount: Some(subaccount),
    };
    let account_identifier = AccountIdentifier::new(&canister_id, &Subaccount(subaccount));
    let bitcoin = bitcoin::address::account_to_p2pkh_address(&account);
    DepositAddresses {
        account_string: account.to_string(),
        account,
        account_identifier_string: account_identifier.to_hex(),
        account_identifier,
        bitcoin,
    }
}

pub fn generate_ic_subaccount(principal: &Principal) -> ic_ledger_types::Subaccount {
    ic_ledger_types::Subaccount(principal_to_subaccount(principal))
}

pub fn generate_subaccount_for_pool(poolid: u128, created_at: u64) -> [u8; 32] {
    let mut hash = [0; 32];
    let mut hasher = Sha3::v256();
    hasher.update(&poolid.to_be_bytes());
    hasher.update(&created_at.to_le_bytes());
    hasher.update(ic_cdk::id().as_slice());
    hasher.finalize(&mut hash);
    hash
}
