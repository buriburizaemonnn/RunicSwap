use bitcoin::{
    absolute::LockTime, hashes::Hash, transaction::Version, Address, Amount, OutPoint, ScriptBuf,
    Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};
use ic_cdk::api::management_canister::bitcoin::Utxo;
use icrc_ledger_types::icrc1::account::Account;
use ordinals::{Edict, Runestone};

use crate::{
    chains::btc::{runestone::DEFAULT_POSTAGE, signer::mock_signature, DUST_THRESHOLD},
    state::write_utxo_manager,
    txn_handler::TransactionType,
    types::{RuneId, RunicUtxo},
};

pub struct CombinedTransactionArgs {
    pub runeid: RuneId,
    pub rune_amount: u128,
    pub rune_sender: Address,
    pub rune_receiver: Address,
    pub rune_sender_account: Account,
    pub btc_amount: u64,
    pub bitcoin_sender: Address,
    pub bitcoin_receiver: Address,
    pub bitcoin_sender_account: Account,
    pub fee_payer: Address,
    pub fee_payer_account: Account,
    pub postage: Option<u64>,
    pub fee_per_vbytes: u64,
}

pub fn transfer(
    CombinedTransactionArgs {
        runeid,
        rune_amount,
        rune_sender,
        rune_receiver,
        rune_sender_account,
        btc_amount,
        bitcoin_sender,
        bitcoin_receiver,
        bitcoin_sender_account,
        fee_payer,
        fee_payer_account,
        postage,
        fee_per_vbytes,
    }: CombinedTransactionArgs,
) -> Result<TransactionType, (u128, u64, u64)> {
    let mut total_fee = 0;
    let postage = Amount::from_sat(postage.unwrap_or(DEFAULT_POSTAGE));
    loop {
        let (txn, runic_utxos, btc_utxos, fee_utxos) = build_transaction_with_fee(
            &runeid,
            rune_amount,
            &rune_sender,
            &rune_receiver,
            btc_amount,
            &bitcoin_sender,
            &bitcoin_receiver,
            total_fee,
            &fee_payer,
            postage,
        )?;

        let signed_txn = mock_signature(&txn);
        let txn_vsize = signed_txn.vsize() as u64;
        if (txn_vsize * fee_per_vbytes) / 1000 == total_fee {
            return Ok(TransactionType::Combined {
                runeid,
                rune_amount,
                rune_sender: Box::new(rune_sender),
                rune_receiver: Box::new(rune_receiver),
                rune_sender_account,
                runic_utxos,
                btc_amount,
                btc_sender: Box::new(bitcoin_sender),
                btc_receiver: Box::new(bitcoin_receiver),
                btc_sender_account: bitcoin_sender_account,
                btc_utxos,
                fee: total_fee,
                fee_payer_account,
                fee_utxos,
                fee_payer: Box::new(fee_payer),
                postage,
            });
        } else {
            write_utxo_manager(|manager| {
                manager.record_runic_utxos(
                    rune_sender.to_string().as_str(),
                    runeid.clone(),
                    runic_utxos,
                );
                manager.record_btc_utxos(bitcoin_sender.to_string().as_str(), btc_utxos);
                manager.record_btc_utxos(fee_payer.to_string().as_str(), fee_utxos);
            });
            total_fee = (txn_vsize * fee_per_vbytes) / 1000;
        }
    }
}

fn build_transaction_with_fee(
    runeid: &RuneId,
    rune_amount: u128,
    rune_sender: &Address,
    rune_receiver: &Address,
    btc_amount: u64,
    btc_sender: &Address,
    btc_receiver: &Address,
    fee: u64,
    fee_payer: &Address,
    postage: Amount,
) -> Result<(Transaction, Vec<RunicUtxo>, Vec<Utxo>, Vec<Utxo>), (u128, u64, u64)> {
    let (mut input, mut output) = (vec![], vec![]);

    let (runic_utxos, runic_total_spent, btc_in_runic_spent) = write_utxo_manager(|manager| {
        let addr = rune_sender.to_string();
        let (mut utxos, mut r_total_spent, mut b_total_spent) = (vec![], 0, 0);

        while let Some(utxo) = manager.get_runic_utxo(&addr, runeid.clone()) {
            r_total_spent += utxo.balance;
            b_total_spent += utxo.utxo.value;
            utxos.push(utxo);
            if r_total_spent > rune_amount {
                break;
            }
        }

        if r_total_spent < rune_amount {
            manager.record_runic_utxos(&addr, runeid.clone(), utxos);
            return Err((rune_amount, btc_amount, fee));
        }

        Ok((utxos, r_total_spent, b_total_spent))
    })?;

    let (need_rune_change_output, required_postage_btc) = {
        let need_rune_change_output = runic_total_spent > rune_amount || runic_utxos.len() > 1;
        let required_postage_btc = if need_rune_change_output {
            postage.to_sat() * 2
        } else {
            postage.to_sat()
        };

        (
            need_rune_change_output,
            required_postage_btc - btc_in_runic_spent,
        )
    };

    let (mut btc_utxos, mut btc_total_spent) = write_utxo_manager(|manager| {
        let addr = btc_sender.to_string();
        let (mut utxos, mut total_spent) = (vec![], 0);

        while let Some(utxo) = manager.get_bitcoin_utxo(&addr) {
            total_spent += utxo.value;
            utxos.push(utxo);
            if total_spent > btc_amount {
                break;
            }
        }
        if total_spent < btc_amount {
            manager.record_runic_utxos(
                &rune_sender.to_string(),
                runeid.clone(),
                runic_utxos.clone(),
            );
            manager.record_btc_utxos(&addr, utxos);
            return Err((rune_amount, btc_amount, fee));
        }
        Ok((utxos, total_spent))
    })?;

    let (fee_utxos, fee_total_spent) = write_utxo_manager(|manager| {
        let addr = fee_payer.to_string();
        let (mut utxos, mut total_spent) = (vec![], 0);

        if fee_payer != btc_sender {
            while let Some(utxo) = manager.get_bitcoin_utxo(&addr) {
                total_spent += utxo.value;
                utxos.push(utxo);
                if total_spent > fee + required_postage_btc {
                    break;
                }
            }

            if total_spent < fee + required_postage_btc {
                manager.record_runic_utxos(
                    &rune_sender.to_string(),
                    runeid.clone(),
                    runic_utxos.clone(),
                );
                manager.record_btc_utxos(&btc_sender.to_string(), btc_utxos.clone());
                manager.record_btc_utxos(&addr, utxos);
                return Err((rune_amount, btc_amount, fee + required_postage_btc));
            }
        } else {
            while let Some(utxo) = manager.get_bitcoin_utxo(&addr) {
                btc_total_spent += utxo.value;
                btc_utxos.push(utxo);
                if btc_total_spent > btc_amount + fee + required_postage_btc {
                    break;
                }
            }
            if btc_total_spent < btc_amount + fee + required_postage_btc {
                return Err((rune_amount, btc_amount + fee + required_postage_btc, 0));
            }
        }

        Ok((utxos, total_spent))
    })?;

    // transaction's input

    runic_utxos
        .iter()
        .for_each(|RunicUtxo { utxo, balance: _ }| {
            input.push(TxIn {
                script_sig: ScriptBuf::new(),
                witness: Witness::new(),
                sequence: Sequence::MAX,
                previous_output: OutPoint {
                    txid: Txid::from_raw_hash(Hash::from_slice(&utxo.outpoint.txid).unwrap()),
                    vout: utxo.outpoint.vout,
                },
            });
        });

    btc_utxos.iter().for_each(|utxo| {
        input.push(TxIn {
            script_sig: ScriptBuf::new(),
            witness: Witness::new(),
            sequence: Sequence::MAX,
            previous_output: OutPoint {
                txid: Txid::from_raw_hash(Hash::from_slice(&utxo.outpoint.txid).unwrap()),
                vout: utxo.outpoint.vout,
            },
        });
    });

    fee_utxos.iter().for_each(|utxo| {
        input.push(TxIn {
            script_sig: ScriptBuf::new(),
            witness: Witness::new(),
            sequence: Sequence::MAX,
            previous_output: OutPoint {
                txid: Txid::from_raw_hash(Hash::from_slice(&utxo.outpoint.txid).unwrap()),
                vout: utxo.outpoint.vout,
            },
        });
    });

    // transaction's output

    // rune

    if need_rune_change_output {
        let runestone = Runestone {
            edicts: vec![Edict {
                amount: rune_amount,
                id: ordinals::RuneId {
                    block: runeid.block,
                    tx: runeid.tx,
                },
                output: 2,
            }],
            ..Default::default()
        };

        output.push(TxOut {
            value: Amount::from_sat(0),
            script_pubkey: runestone.encipher(),
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

    // btc

    if fee_payer == btc_sender {
        let remaining = btc_total_spent - btc_amount - fee - required_postage_btc;
        output.push(TxOut {
            value: Amount::from_sat(btc_amount),
            script_pubkey: btc_receiver.script_pubkey(),
        });

        if remaining > DUST_THRESHOLD {
            output.push(TxOut {
                value: Amount::from_sat(remaining),
                script_pubkey: btc_sender.script_pubkey(),
            });
        }
    } else {
        output.push(TxOut {
            value: Amount::from_sat(btc_amount),
            script_pubkey: btc_receiver.script_pubkey(),
        });
    }

    // fee

    let remaining = fee_total_spent - fee - required_postage_btc;
    if remaining > DUST_THRESHOLD {
        output.push(TxOut {
            value: Amount::from_sat(remaining),
            script_pubkey: fee_payer.script_pubkey(),
        });
    }

    let txn = Transaction {
        input,
        output,
        version: Version(2),
        lock_time: LockTime::ZERO,
    };
    Ok((txn, runic_utxos, btc_utxos, fee_utxos))
}
