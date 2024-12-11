use ic_ledger_types::{AccountIdentifier, Memo, Subaccount, Tokens, TransferArgs, DEFAULT_FEE};

use crate::txn_handler::TransactionType;

pub fn transfer(
    from_subaccount: [u8; 32],
    to: AccountIdentifier,
    amount: u64,
    paid_by_sender: bool,
) -> TransactionType {
    let amount = Tokens::from_e8s(if paid_by_sender {
        amount
    } else {
        amount - DEFAULT_FEE.e8s()
    });

    let from_subaccount = Subaccount(from_subaccount);

    let arg = TransferArgs {
        memo: Memo(0),
        amount,
        fee: DEFAULT_FEE,
        from_subaccount: Some(from_subaccount),
        to,
        created_at_time: None,
    };

    todo!()
}
