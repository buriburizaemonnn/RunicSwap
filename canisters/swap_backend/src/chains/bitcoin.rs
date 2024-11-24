use bitcoin::{address::NetworkUnchecked, Address};
use ic_cdk::api::management_canister::bitcoin::{
    bitcoin_get_current_fee_percentiles, BitcoinNetwork, GetCurrentFeePercentilesRequest,
};

use crate::state::read_config;

pub mod address;
pub mod runestone;
pub mod transaction;
pub mod utils;

pub async fn get_fee_per_vbyte() -> u64 {
    let network = read_config(|config| config.bitcoin_network());
    // Get fee percentiles from previous transactions to estimate our own fee.
    let fee_percentiles =
        bitcoin_get_current_fee_percentiles(GetCurrentFeePercentilesRequest { network })
            .await
            .unwrap()
            .0;

    if fee_percentiles.is_empty() {
        // There are no fee percentiles. This case can only happen on a regtest
        // network where there are no non-coinbase transactions. In this case,
        // we use a default of 2000 millisatoshis/byte (i.e. 2 satoshi/byte)
        2000
    } else {
        // Choose the 50th percentile for sending fees.
        fee_percentiles[50]
    }
}

pub fn validate_address(addr: &str) -> Address {
    read_config(|config| {
        let bitcoin_network = match config.bitcoin_network() {
            BitcoinNetwork::Mainnet => bitcoin::Network::Bitcoin,
            BitcoinNetwork::Testnet => bitcoin::Network::Testnet,
            BitcoinNetwork::Regtest => bitcoin::Network::Regtest,
        };
        let addr: Address<NetworkUnchecked> = match addr.parse() {
            Err(_e) => ic_cdk::trap("Failed to parse address"),
            Ok(addr) => addr,
        };
        if !addr.is_valid_for_network(bitcoin_network) {
            ic_cdk::trap("Address invalid for selected bitcoin network")
        }
        addr.require_network(bitcoin_network).unwrap()
    })
}
