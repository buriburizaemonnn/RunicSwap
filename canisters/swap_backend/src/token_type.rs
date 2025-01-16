use candid::CandidType;
use serde::Deserialize;

use crate::{
    addresses::Addresses,
    chain::btc::{address_validation, transaction::combined::CombinedTransferArgs},
    updater::RuneId,
};

#[derive(CandidType, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenType {
    Bitcoin,
    Rune(RuneId),
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_combined_txn(
    token0: &TokenType,
    token1: &TokenType,
    rune_sender: &Addresses,
    rune_receiver: &str,
    bitcoin_sender: &Addresses,
    bitcoin_receiver: &str,
    fee_payer: &Addresses,
    amount0: u64,
    amount1: u64,
) -> Option<()> {
    let (rune, rune_amount, bitcoin_amount);
    match (token0, token1) {
        (TokenType::Bitcoin, TokenType::Rune(id)) => {
            rune = *id;
            rune_amount = amount1;
            bitcoin_amount = amount0;
        }
        (TokenType::Rune(id), TokenType::Bitcoin) => {
            rune = *id;
            rune_amount = amount0;
            bitcoin_amount = amount1;
        }
        _ => return None,
    }

    // TODO: check for balance

    // addresses
    let rune_sender_address = address_validation(&rune_sender.bitcoin).unwrap();
    let rune_receiver = address_validation(rune_receiver).unwrap();
    let bitcoin_sender_address = address_validation(&bitcoin_sender.bitcoin).unwrap();
    let bitcoin_receiver = address_validation(bitcoin_receiver).unwrap();
    let fee_payer_address = address_validation(&fee_payer.bitcoin).unwrap();

    todo!()
}
