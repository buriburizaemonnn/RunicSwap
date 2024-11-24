use ic_cdk::api::management_canister::bitcoin::BitcoinNetwork;
use icrc_ledger_types::icrc1::account::Account;

use crate::state::read_config;

use super::utils::{account_to_derivation_path, derive_public_key, ripemd160, sha256};

pub fn account_to_p2pkh_address(account: &Account) -> String {
    read_config(|config| {
        let prefix = match config.bitcoin_network() {
            BitcoinNetwork::Mainnet => 0x00,
            BitcoinNetwork::Testnet | BitcoinNetwork::Regtest => 0x6f,
        };
        let ecdsa_key = config.ecdsa_key();
        let path = account_to_derivation_path(account);
        let derived_public_key = derive_public_key(ecdsa_key, &path).public_key;
        let ripemd_pk = ripemd160(&sha256(&derived_public_key));
        let mut raw_address = vec![prefix];
        raw_address.extend(ripemd_pk);
        let checksum = &sha256(&sha256(&raw_address.clone()))[..4];
        raw_address.extend(checksum);
        bs58::encode(raw_address).into_string()
    })
}
