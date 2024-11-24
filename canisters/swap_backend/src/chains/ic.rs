use ic_ledger_types::{AccountIdentifier, Memo, Subaccount, Tokens, TransferArgs};

use crate::transaction_handler::TransactionType;

pub mod icrc1;

pub fn transfer(
    from_subaccount: Subaccount,
    to: AccountIdentifier,
    amount: u64,
    fee_paid_by_sender: bool,
) -> TransactionType {
    let amount = if fee_paid_by_sender {
        amount + ic_ledger_types::DEFAULT_FEE.e8s()
    } else {
        amount - ic_ledger_types::DEFAULT_FEE.e8s()
    };
    TransactionType::Icp(TransferArgs {
        memo: Memo(0),
        amount: Tokens::from_e8s(amount),
        fee: ic_ledger_types::DEFAULT_FEE,
        from_subaccount: Some(from_subaccount),
        to,
        created_at_time: None,
    })
}
