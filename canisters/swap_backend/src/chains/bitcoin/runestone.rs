use bitcoin::{
    absolute::LockTime, hashes::Hash, Address, OutPoint, ScriptBuf, Sequence, Transaction, TxIn,
    TxOut, Txid, Witness,
};
use ic_cdk::api::management_canister::bitcoin::Utxo;
use icrc_ledger_types::icrc1::account::Account;
use ordinals::{Edict, Runestone};

use crate::{
    state::{utxo_manager::RunicUtxo, write_utxo_state},
    transaction_handler::TransactionType,
    types::RuneId,
};

const DEFAULT_POSTAGE: u64 = 10_000;

/* pub fn calculate_transaction_fee(
    runeid: &RuneId,
    amount: u128,
    sender_addr: &str,
    receiver_addr: &str,
    sender_address: &Address,
    receiver_address: &Address,
    fee_paid_by_sender: bool,
    fee_per_vbyte: u64,
) -> u64 {
    // match build_transaction_with_fee(runeid, amount, sender_addr, receiver_addr, sender_address, receiver_address, fee_paid_by_sender, postage, fee)
    todo!()
} */

pub fn transfer(
    runeid: &RuneId,
    amount: u128,
    sender_addr: &str,
    receiver_addr: &str,
    sender_address: &Address,
    receiver_address: &Address,
    sender_account: &Account,
    receiver_account: &Account,
    fee_paid_by_sender: bool,
    fee_per_vbyte: u64,
    postage: Option<u64>,
) -> TransactionType {
    let mut total_fee = 0;
    loop {
        let (
            txn,
            runic_utxos_owned_by_sender,
            btc_utxos_owned_by_sender,
            btc_utxos_owned_by_receiver,
        ) = build_transaction_with_fee(
            runeid,
            amount,
            sender_addr,
            receiver_addr,
            sender_address,
            receiver_address,
            fee_paid_by_sender,
            postage.unwrap_or(DEFAULT_POSTAGE),
            total_fee,
        )
        .expect("should build");
        let mock_signed_txn = super::transaction::mock_signature(&txn);
        let tx_vsize = mock_signed_txn.vsize() as u64;

        if (tx_vsize * fee_per_vbyte) / 1000 == total_fee {
            if fee_paid_by_sender {
                return TransactionType::Runestone {
                    runic_utxos: runic_utxos_owned_by_sender,
                    bitcoin_utxos: btc_utxos_owned_by_sender,
                    addr: sender_addr.to_string(),
                    signer_account: sender_account.clone(),
                    signer_address: sender_address.clone(),
                    txn,
                };
            } else {
                return TransactionType::LegoRunestoneTransaction {
                    runeid: runeid.clone(),
                    amount,
                    postage: postage.unwrap_or(DEFAULT_POSTAGE),
                    paying_fee_amount: total_fee,
                    to_be_signed_by_sender: runic_utxos_owned_by_sender,
                    to_be_signed_by_receiver: btc_utxos_owned_by_receiver,
                    sender_addr: sender_addr.to_string(),
                    receiver_addr: sender_addr.to_string(),
                    sender_account: sender_account.clone(),
                    receiver_account: receiver_account.clone(),
                    sender_address: sender_address.clone(),
                    receiver_address: sender_address.clone(),
                };
            }
        } else {
            write_utxo_state(|state| {
                state.record_runic_utxos(sender_addr, runeid.clone(), runic_utxos_owned_by_sender);
                state.record_bitcoin_utxos(sender_addr, btc_utxos_owned_by_sender);
                state.record_bitcoin_utxos(receiver_addr, btc_utxos_owned_by_receiver);
            });
            total_fee = (tx_vsize * fee_per_vbyte) / 1000;
        }
    }
}

/*
 * returns (
 * transaction,
 * runic_utxos_owned_by_sender,
 * btc_utxos_owned_by_sender,
 * btc_utxos_owned_by_receiver,
 * )
*/

fn build_transaction_with_fee(
    runeid: &RuneId,
    amount: u128,
    sender_addr: &str,
    receiver_addr: &str,
    sender_address: &Address,
    receiver_address: &Address,
    fee_paid_by_sender: bool,
    postage: u64,
    fee: u64,
) -> Result<(Transaction, Vec<RunicUtxo>, Vec<Utxo>, Vec<Utxo>), (u64, String)> {
    const DOST_THRESHOLD: u64 = 1_000;
    let (runic_utxos, btc_value_in_runic, runic_total_spent) = write_utxo_state(|state| {
        let mut utxos: Vec<RunicUtxo> = vec![];
        let mut btc_value = 0;
        let mut total_spent = 0;
        while let Some(utxo) = state.get_runic_utxo(sender_addr, runeid.clone()) {
            btc_value += utxo.utxo.value;
            total_spent += utxo.balance;
            utxos.push(utxo);
            if total_spent > amount {
                break;
            }
        }
        if total_spent < amount {
            return Err((
                0, // runestone's amount is already known
                format!(
                    "Not enough balance\nRequired: {}, Current Balance: {}",
                    amount, total_spent
                ),
            ));
        }
        Ok((utxos, btc_value, total_spent))
    })?;

    let needs_rune_change_output = runic_utxos.len() > 1 || runic_total_spent > amount;

    let required_btc = if needs_rune_change_output {
        let postages = postage * 2;
        if btc_value_in_runic > postages {
            0
        } else {
            postages - btc_value_in_runic
        }
    } else {
        if btc_value_in_runic > postage {
            0
        } else {
            postage - btc_value_in_runic
        }
    };

    let (btc_utxos, btc_total_spent) = write_utxo_state(|state| {
        let mut utxos = vec![];
        let mut total_spent = 0;
        if fee_paid_by_sender {
            while let Some(utxo) = state.get_btc_utxo(sender_addr) {
                total_spent += utxo.value;
                utxos.push(utxo);
                if total_spent >= fee + required_btc {
                    break;
                }
            }
        } else {
            while let Some(utxo) = state.get_btc_utxo(receiver_addr) {
                total_spent += utxo.value;
                utxos.push(utxo);
                if total_spent >= fee + required_btc {
                    break;
                }
            }
        }
        if total_spent < fee + required_btc {
            return Err((
                fee + required_btc,
                format!(
                    "Not enough balance\nRequired: {}, Current Balance: {}",
                    fee + required_btc,
                    total_spent
                ),
            ));
        }
        Ok((utxos, total_spent))
    })?;

    let mut input: Vec<TxIn> = runic_utxos
        .iter()
        .map(|runic_utxo| TxIn {
            sequence: Sequence::MAX,
            witness: Witness::new(),
            script_sig: ScriptBuf::new(),
            previous_output: OutPoint {
                vout: runic_utxo.utxo.outpoint.vout,
                txid: Txid::from_raw_hash(
                    Hash::from_slice(&runic_utxo.utxo.outpoint.txid).expect("should hash"),
                ),
            },
        })
        .collect();

    btc_utxos.iter().for_each(|utxo| {
        let txin = TxIn {
            sequence: Sequence::MAX,
            witness: Witness::new(),
            script_sig: ScriptBuf::new(),
            previous_output: OutPoint {
                vout: utxo.outpoint.vout,
                txid: Txid::from_raw_hash(
                    Hash::from_slice(&utxo.outpoint.txid).expect("should hash"),
                ),
            },
        };
        input.push(txin)
    });

    let ordinal_runeid = ordinals::RuneId {
        block: runeid.block,
        tx: runeid.tx,
    };
    let runestone = Runestone {
        edicts: vec![Edict {
            amount,
            id: ordinal_runeid,
            output: 2,
        }],
        ..Default::default()
    };

    let mut output = if needs_rune_change_output {
        vec![
            TxOut {
                script_pubkey: runestone.encipher(),
                value: 0,
            },
            TxOut {
                script_pubkey: sender_address.script_pubkey(),
                value: postage,
            },
            TxOut {
                script_pubkey: receiver_address.script_pubkey(),
                value: postage,
            },
        ]
    } else {
        vec![TxOut {
            script_pubkey: receiver_address.script_pubkey(),
            value: postage,
        }]
    };

    let remaining_btc = btc_total_spent - fee - required_btc;
    if required_btc > 1_000 {
        output.push(TxOut {
            value: remaining_btc,
            script_pubkey: if fee_paid_by_sender {
                sender_address.script_pubkey()
            } else {
                receiver_address.script_pubkey()
            },
        })
    }
    let txn = Transaction {
        version: 2,
        lock_time: LockTime::ZERO,
        input,
        output,
    };
    let (sender_btc_utxos, receiver_btc_utxos) = if fee_paid_by_sender {
        (btc_utxos, vec![])
    } else {
        (vec![], btc_utxos)
    };
    Ok((txn, runic_utxos, sender_btc_utxos, receiver_btc_utxos))
}
