use bitcoin::{
    absolute::LockTime, transaction::Version, Address, Amount, OutPoint, ScriptBuf, Sequence,
    Transaction, TxIn, TxOut, Witness,
};
use ic_cdk::api::management_canister::bitcoin::Utxo;
use icrc_ledger_types::icrc1::account::Account;
use ordinals::{Edict, Runestone};

use crate::{
    chain::btc::{signer::mock_signature, utils::slice_to_txid, DEFAULT_POSTAGE, DUST_THRESHOLD},
    state::{utxo_manager::RunicUtxo, write_utxo_manager},
    txn_handler::TransactionType,
    RuneId,
};

pub struct CombinedTransferArgs {
    pub runeid: RuneId,
    pub rune_amount: u128,
    pub rune_sender: Address,
    pub rune_sender_account: Account,
    pub rune_receiver: Address,
    pub bitcoin_amount: u64,
    pub bitcoin_sender: Address,
    pub bitcoin_sender_account: Account,
    pub bitcoin_receiver: Address,
    pub fee_payer: Address,
    pub fee_payer_account: Account,
    pub postage: Option<u64>,
    pub fee_per_vbytes: u64,
}

pub fn transfer(
    CombinedTransferArgs {
        runeid,
        rune_amount,
        rune_sender,
        rune_sender_account,
        rune_receiver,
        bitcoin_amount,
        bitcoin_sender,
        bitcoin_sender_account,
        bitcoin_receiver,
        fee_payer,
        fee_payer_account,
        postage,
        fee_per_vbytes,
    }: CombinedTransferArgs,
) -> Result<TransactionType, (u128, u64, u64)> {
    let mut total_fee = 0;
    let postage = Amount::from_sat(postage.unwrap_or(DEFAULT_POSTAGE));
    loop {
        let (txn, runic_utxos, bitcoin_utxos, fee_utxos) = build_transaction_with_fee(
            &runeid,
            rune_amount,
            &rune_sender,
            &rune_receiver,
            bitcoin_amount,
            &bitcoin_sender,
            &bitcoin_receiver,
            &fee_payer,
            postage,
            total_fee,
        )?;
        let signed_txn = mock_signature(&txn);
        let txn_vsize = signed_txn.vsize() as u64;
        if (txn_vsize * fee_per_vbytes) / 1000 == total_fee {
            return Ok(TransactionType::Combined {
                runic_utxos,
                runeid,
                rune_amount,
                rune_sender: Box::new(rune_sender),
                rune_receiver: Box::new(rune_receiver),
                rune_sender_account,
                bitcoin_utxos,
                bitcoin_amount,
                bitcoin_sender: Box::new(bitcoin_sender),
                bitcoin_receiver: Box::new(bitcoin_receiver),
                bitcoin_sender_account,
                fee_utxos,
                fee: total_fee,
                fee_payer: Box::new(fee_payer),
                fee_payer_account,
                postage,
            });
        } else {
            write_utxo_manager(|manager| {
                manager.record_runic_utxos(rune_sender.to_string().as_str(), runeid, runic_utxos);
                manager.record_btc_utxos(bitcoin_sender.to_string().as_str(), bitcoin_utxos);
                manager.record_btc_utxos(fee_payer.to_string().as_str(), fee_utxos);
            });
            total_fee = (txn_vsize * fee_per_vbytes) / 1000;
        }
    }
}

/*
 * return a Result
 * Ok => (transaction, runic_utxos, bitcoin_utxos, fee_utxos)
 * Err => (balance_required_for_rune, balance_required_for_bitcoin, balance_required_for_fee)
 * NOTE: when fee_payer and bitcoin_sender are same, utxos for fee is also added to the
 * bitcoin_utxos,
*/
fn build_transaction_with_fee(
    runeid: &RuneId,
    rune_amount: u128,
    rune_sender: &Address,
    rune_receiver: &Address,
    bitcoin_amount: u64,
    bitcoin_sender: &Address,
    bitcoin_receiver: &Address,
    fee_payer: &Address,
    postage: Amount,
    fee: u64,
) -> Result<(Transaction, Vec<RunicUtxo>, Vec<Utxo>, Vec<Utxo>), (u128, u64, u64)> {
    write_utxo_manager(|manager| {
        let rune_sender_addr = rune_sender.to_string();

        let mut runic_utxos = vec![];
        let mut runic_total_spent = 0;
        let mut bitcoin_spent_in_runic = 0;

        while let Some(utxo) = manager.get_runic_utxo(&rune_sender_addr, *runeid) {
            runic_total_spent += utxo.balance;
            bitcoin_spent_in_runic += utxo.utxo.value;
            runic_utxos.push(utxo);
            if runic_total_spent > rune_amount {
                break;
            }
        }

        if runic_total_spent < rune_amount {
            manager.record_runic_utxos(&rune_sender_addr, *runeid, runic_utxos);
            return Err((rune_amount, bitcoin_amount, fee));
        }

        let need_change_rune_output = runic_utxos.len() > 1 || runic_total_spent > rune_amount;

        let required_postage_amount = if need_change_rune_output {
            postage.to_sat() * 2
        } else {
            postage.to_sat()
        } - bitcoin_spent_in_runic;

        let required_total_bitcoin_fee = fee + required_postage_amount;

        let bitcoin_sender_addr = bitcoin_sender.to_string();
        let mut bitcoin_utxos = vec![];
        let mut bitcoin_total_spent = 0;

        while let Some(utxo) = manager.get_bitcoin_utxo(&bitcoin_sender_addr) {
            bitcoin_total_spent += utxo.value;
            bitcoin_utxos.push(utxo);
            if bitcoin_total_spent > bitcoin_amount {
                break;
            }
        }

        if bitcoin_total_spent < bitcoin_amount {
            manager.record_runic_utxos(&rune_sender_addr, *runeid, runic_utxos);
            manager.record_btc_utxos(&bitcoin_sender_addr, bitcoin_utxos);
            return Err((rune_amount, bitcoin_amount, fee));
        }

        let fee_payer_addr = fee_payer.to_string();
        let mut fee_utxos = vec![];
        let mut fee_total_spent = 0;

        // NOTE: fee payer and bitcoin sender can be same
        if fee_payer == bitcoin_sender
            && (bitcoin_total_spent - bitcoin_amount) < required_total_bitcoin_fee
        {
            while let Some(utxo) = manager.get_bitcoin_utxo(&fee_payer_addr) {
                bitcoin_total_spent += utxo.value;
                bitcoin_utxos.push(utxo);
                if bitcoin_total_spent > (bitcoin_amount + required_total_bitcoin_fee) {
                    break;
                }
                if bitcoin_total_spent < (bitcoin_amount + required_total_bitcoin_fee) {
                    manager.record_runic_utxos(&rune_sender_addr, *runeid, runic_utxos);
                    manager.record_btc_utxos(&bitcoin_sender_addr, bitcoin_utxos);
                    return Err((rune_amount, bitcoin_amount, fee));
                }
            }
        } else {
            while let Some(utxo) = manager.get_bitcoin_utxo(&fee_payer_addr) {
                fee_total_spent += utxo.value;
                fee_utxos.push(utxo);
                if fee_total_spent > required_total_bitcoin_fee {
                    break;
                }
                if fee_total_spent < required_total_bitcoin_fee {
                    manager.record_runic_utxos(&rune_sender_addr, *runeid, runic_utxos);
                    manager.record_btc_utxos(&bitcoin_sender_addr, bitcoin_utxos);
                    manager.record_btc_utxos(&fee_payer_addr, fee_utxos);
                    return Err((rune_amount, bitcoin_amount, fee));
                }
            }
        }

        let mut input = vec![];
        let mut output = vec![];

        // runic utxos
        runic_utxos
            .iter()
            .for_each(|RunicUtxo { utxo, balance: _ }| {
                let txin = TxIn {
                    previous_output: OutPoint {
                        txid: slice_to_txid(&utxo.outpoint.txid),
                        vout: utxo.outpoint.vout,
                    },
                    script_sig: ScriptBuf::new(),
                    sequence: Sequence::MAX,
                    witness: Witness::new(),
                };
                input.push(txin);
            });

        // bitcoin utxos
        bitcoin_utxos.iter().for_each(|utxo| {
            let txin = TxIn {
                previous_output: OutPoint {
                    txid: slice_to_txid(&utxo.outpoint.txid),
                    vout: utxo.outpoint.vout,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            };
            input.push(txin);
        });

        // fee utxos
        fee_utxos.iter().for_each(|utxo| {
            let txin = TxIn {
                previous_output: OutPoint {
                    txid: slice_to_txid(&utxo.outpoint.txid),
                    vout: utxo.outpoint.vout,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            };
            input.push(txin);
        });

        // output

        // runic
        if need_change_rune_output {
            let id = ordinals::RuneId {
                block: runeid.block,
                tx: runeid.tx,
            };
            let rune = Runestone {
                edicts: vec![Edict {
                    id,
                    amount: rune_amount,
                    output: 2,
                }],
                ..Default::default()
            };

            output.push(TxOut {
                script_pubkey: rune.encipher(),
                value: Amount::from_sat(0),
            });
            output.push(TxOut {
                value: postage,
                script_pubkey: rune_sender.script_pubkey(),
            });
            output.push(TxOut {
                value: postage,
                script_pubkey: rune_receiver.script_pubkey(),
            });
        } else {
            output.push(TxOut {
                value: postage,
                script_pubkey: rune_receiver.script_pubkey(),
            });
        }

        output.push(TxOut {
            value: Amount::from_sat(bitcoin_amount),
            script_pubkey: bitcoin_receiver.script_pubkey(),
        });

        if bitcoin_sender == fee_payer {
            let remaining = bitcoin_total_spent - bitcoin_amount - required_total_bitcoin_fee;
            if remaining >= DUST_THRESHOLD {
                output.push(TxOut {
                    value: Amount::from_sat(remaining),
                    script_pubkey: bitcoin_sender.script_pubkey(),
                });
            }
        } else {
            let remaining_bitcoin = bitcoin_total_spent - bitcoin_amount;
            if remaining_bitcoin >= DUST_THRESHOLD {
                output.push(TxOut {
                    value: Amount::from_sat(remaining_bitcoin),
                    script_pubkey: bitcoin_sender.script_pubkey(),
                });
            }

            let remaining_fee = fee_total_spent - required_total_bitcoin_fee;
            if remaining_fee >= DUST_THRESHOLD {
                output.push(TxOut {
                    value: Amount::from_sat(remaining_fee),
                    script_pubkey: fee_payer.script_pubkey(),
                });
            }
        }

        let txn = Transaction {
            input,
            output,
            version: Version(2),
            lock_time: LockTime::ZERO,
        };
        Ok((txn, runic_utxos, bitcoin_utxos, fee_utxos))
    })
}
