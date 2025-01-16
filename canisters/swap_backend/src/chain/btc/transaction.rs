pub mod combined;
pub mod runestone;

use bitcoin::{
    absolute::LockTime, transaction::Version, Address, Amount, OutPoint, ScriptBuf, Sequence,
    Transaction, TxIn, TxOut, Witness,
};
use ic_cdk::api::management_canister::bitcoin::Utxo;
use icrc_ledger_types::icrc1::account::Account;

use crate::{state::write_utxo_manager, txn_handler::TransactionType};

use super::{signer::mock_signature, utils::slice_to_txid, DUST_THRESHOLD};

pub struct BtcTransferArgs {
    pub sender: Address,
    pub receiver: Address,
    pub sender_account: Account,
    pub amount: u64,
    pub paid_by_sender: bool,
    pub fee_per_vbytes: u64,
}

pub fn transfer(
    BtcTransferArgs {
        sender,
        receiver,
        sender_account,
        amount,
        paid_by_sender,
        fee_per_vbytes,
    }: BtcTransferArgs,
) -> Result<TransactionType, u64> {
    let mut total_fee = 0;
    loop {
        let (txn, utxos) =
            build_transaction_with_fee(&sender, &receiver, amount, paid_by_sender, total_fee)?;
        let signed_txn = mock_signature(&txn);
        let txn_vsize = signed_txn.vsize() as u64;
        if (txn_vsize * fee_per_vbytes) / 1000 == total_fee {
            return Ok(TransactionType::Bitcoin {
                utxos,
                txn: Box::new(txn),
                sender: Box::new(sender),
                sender_account,
            });
        } else {
            write_utxo_manager(|manager| {
                manager.record_btc_utxos(sender.to_string().as_str(), utxos)
            });
            total_fee = (txn_vsize * fee_per_vbytes) / 1000;
        }
    }
}

fn build_transaction_with_fee(
    sender: &Address,
    receiver: &Address,
    amount: u64,
    paid_by_sender: bool,
    fee: u64,
) -> Result<(Transaction, Vec<Utxo>), u64> {
    let (mut input, mut output) = (vec![], vec![]);
    let required_amount = if paid_by_sender { amount + fee } else { amount };
    let (utxos, total_spent) = write_utxo_manager(|manager| {
        let addr = sender.to_string();
        let mut utxos = vec![];
        let mut total_spent = 0;
        while let Some(utxo) = manager.get_bitcoin_utxo(&addr) {
            total_spent += utxo.value;
            utxos.push(utxo);
            if total_spent > required_amount {
                break;
            }
        }
        if total_spent < required_amount {
            manager.record_btc_utxos(&addr, utxos);
            return Err(required_amount);
        }
        Ok((utxos, total_spent))
    })?;

    utxos.iter().for_each(|utxo| {
        let txin = TxIn {
            previous_output: OutPoint {
                txid: slice_to_txid(&utxo.outpoint.txid),
                vout: utxo.outpoint.vout,
            },
            sequence: Sequence::MAX,
            script_sig: ScriptBuf::new(),
            witness: Witness::new(),
        };
        input.push(txin);
    });

    let receiver_amount = if paid_by_sender { amount } else { amount - fee };

    output.push(TxOut {
        value: Amount::from_sat(receiver_amount),
        script_pubkey: receiver.script_pubkey(),
    });

    let remaining = total_spent - required_amount;

    if remaining >= DUST_THRESHOLD {
        output.push(TxOut {
            value: Amount::from_sat(remaining),
            script_pubkey: sender.script_pubkey(),
        });
    }

    let txn = Transaction {
        input,
        output,
        version: Version(2),
        lock_time: LockTime::ZERO,
    };
    Ok((txn, utxos))
}
