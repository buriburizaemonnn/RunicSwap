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

pub struct RuneTransferArgs {
    pub runeid: RuneId,
    pub rune_amount: u128,
    pub rune_sender: Address,
    pub rune_receiver: Address,
    pub rune_sender_account: Account,
    pub fee_payer: Address,
    pub fee_payer_account: Account,
    pub postage: Option<u64>,
    pub fee_per_vbytes: u64,
}

pub fn transfer(
    RuneTransferArgs {
        runeid,
        rune_amount,
        rune_sender,
        rune_receiver,
        rune_sender_account,
        fee_payer,
        fee_payer_account,
        postage,
        fee_per_vbytes,
    }: RuneTransferArgs,
) -> Result<TransactionType, (u128, u64)> {
    let mut total_fee = 0;
    let postage = Amount::from_sat(postage.unwrap_or(DEFAULT_POSTAGE));
    loop {
        let (txn, runic_utxos, fee_utxos) = build_transaction_with_fee(
            &runeid,
            rune_amount,
            &rune_sender,
            &rune_receiver,
            &fee_payer,
            postage,
            total_fee,
        )?;
        let signed_txn = mock_signature(&txn);
        let txn_vsize = signed_txn.vsize() as u64;
        if (txn_vsize * fee_per_vbytes) / 1000 == total_fee {
            return Ok(TransactionType::Rune {
                runic_utxos,
                runeid,
                rune_amount,
                rune_sender: Box::new(rune_sender),
                rune_receiver: Box::new(rune_receiver),
                rune_sender_account,
                fee_utxos,
                fee: total_fee,
                fee_payer: Box::new(fee_payer),
                fee_payer_account,
                postage,
            });
        } else {
            write_utxo_manager(|manager| {
                manager.record_runic_utxos(rune_sender.to_string().as_str(), runeid, runic_utxos);
                manager.record_btc_utxos(fee_payer.to_string().as_str(), fee_utxos)
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
    fee_payer: &Address,
    postage: Amount,
    fee: u64,
) -> Result<(Transaction, Vec<RunicUtxo>, Vec<Utxo>), (u128, u64)> {
    let mut input = vec![];
    let mut output = vec![];
    let (runic_utxos, runic_total_spent, bitcoin_spent_in_runic) = write_utxo_manager(|manager| {
        let addr = rune_sender.to_string();
        let mut utxos = vec![];
        let mut runic_total_spent = 0;
        let mut bitcoin_spent_in_runic = 0;
        while let Some(utxo) = manager.get_runic_utxo(&addr, *runeid) {
            runic_total_spent += utxo.balance;
            bitcoin_spent_in_runic += utxo.utxo.value;
            utxos.push(utxo);
            if runic_total_spent > rune_amount {
                break;
            }
        }
        if runic_total_spent < rune_amount {
            manager.record_runic_utxos(&addr, *runeid, utxos);
            return Err((rune_amount, fee));
        }
        Ok((utxos, runic_total_spent, bitcoin_spent_in_runic))
    })?;

    runic_utxos
        .iter()
        .for_each(|RunicUtxo { utxo, balance: _ }| {
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

    let need_change_rune_output = runic_utxos.len() > 1 || runic_total_spent > rune_amount;

    if need_change_rune_output {
        let id = ordinals::RuneId {
            block: runeid.block,
            tx: runeid.tx,
        };
        let runestone = Runestone {
            edicts: vec![Edict {
                id,
                amount: rune_amount,
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

    let required_postage_bitcoin_amount = if need_change_rune_output {
        postage.to_sat() * 2
    } else {
        postage.to_sat()
    } - bitcoin_spent_in_runic;

    let required_bitcoin_amount = fee + required_postage_bitcoin_amount;

    let (fee_utxos, fee_total_spent) = write_utxo_manager(|manager| {
        let addr = fee_payer.to_string();
        let mut utxos = vec![];
        let mut fee_total_spent = 0;
        while let Some(utxo) = manager.get_bitcoin_utxo(&addr) {
            fee_total_spent += utxo.value;
            utxos.push(utxo);
            if fee_total_spent > required_bitcoin_amount {
                break;
            }
        }
        Ok((utxos, fee_total_spent))
    })?;

    fee_utxos.iter().for_each(|utxo| {
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

    let remaining = fee_total_spent - required_bitcoin_amount;

    if remaining >= DUST_THRESHOLD {
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

    Ok((txn, runic_utxos, fee_utxos))
}
