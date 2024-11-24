use candid::Nat;
use icrc_ledger_types::icrc1::{
    account::Account,
    transfer::{TransferArg, TransferError},
};

use crate::{
    state::read_config, transaction_handler::TransactionType, types::SubmittedTransactionIdType,
};

pub fn transfer(
    from_subaccount: icrc_ledger_types::icrc1::account::Subaccount,
    to: Account,
    amount: u128,
    fee_paid_by_sender: bool,
) -> TransactionType {
    let amount = if fee_paid_by_sender {
        Nat::from(amount + 10_000)
    } else {
        Nat::from(amount - 10_000)
    };
    let txn = TransferArg {
        from_subaccount: Some(from_subaccount),
        to,
        fee: Some(Nat::from(10_000u128)),
        created_at_time: None,
        memo: None,
        amount,
    };
    TransactionType::Icrc1(txn)
}

pub async fn submit_icrc1_txn(txn: TransferArg) -> Option<SubmittedTransactionIdType> {
    let ckbtc = read_config(|config| config.ckbtc());
    let result: (Result<Nat, TransferError>,) = ic_cdk::call(ckbtc, "icrc1_transfer", (txn,))
        .await
        .expect("failed to call ckbtc canister");
    match result.0 {
        Err(err) => {
            ic_cdk::println!("Failed to transfer token: {:?}", err);
            None
        }
        Ok(txid) => Some(SubmittedTransactionIdType::Icrc1 { txid }),
    }
}
