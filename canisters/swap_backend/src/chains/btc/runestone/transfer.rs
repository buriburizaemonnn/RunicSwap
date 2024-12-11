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

pub struct RuneTransferArgs {
    pub runeid: RuneId,
    pub amount: u128,
    pub sender: Address,
    pub sender_account: Account,
    pub receiver: Address,
    pub fee_payer: Address,
    pub fee_payer_account: Account,
    pub postage: Option<u64>,
    pub fee_per_vbytes: u64,
}

pub fn transfer(
    RuneTransferArgs {
        runeid,
        amount,
        sender,
        sender_account,
        receiver,
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
            &runeid, amount, &sender, &receiver, &fee_payer, postage, total_fee,
        )?;

        let signed_txn = mock_signature(&txn);
        let txn_vsize = signed_txn.vsize() as u64;

        if (txn_vsize * fee_per_vbytes) / 1000 == total_fee {
            return Ok(TransactionType::Rune {
                rune: runeid,
                rune_amount: amount,
                runic_utxos,
                sender: Box::new(sender),
                receiver: Box::new(receiver),
                sender_account,
                fee: total_fee,
                fee_payer: Box::new(fee_payer),
                fee_payer_account,
                fee_utxos,
                postage,
            });
        } else {
            write_utxo_manager(|manager| {
                manager.record_runic_utxos(
                    sender.to_string().as_str(),
                    runeid.clone(),
                    runic_utxos,
                );
                manager.record_btc_utxos(fee_payer.to_string().as_str(), fee_utxos);
            });
            total_fee = (txn_vsize * fee_per_vbytes) / 1000;
        }
    }
}

fn build_transaction_with_fee(
    runeid: &RuneId,
    amount: u128,
    sender: &Address,
    receiver: &Address,
    fee_payer: &Address,
    postage: Amount,
    fee: u64,
) -> Result<(Transaction, Vec<RunicUtxo>, Vec<Utxo>), (u128, u64)> {
    let (mut input, mut output) = (vec![], vec![]);

    let (runic_utxos, runic_total_spent, btc_in_runic_spent) = write_utxo_manager(|manager| {
        let (mut utxos, mut runic_total_spent, mut btc_in_runic_spent) = (vec![], 0, 0);
        let addr = sender.to_string();
        while let Some(utxo) = manager.get_runic_utxo(&addr, runeid.clone()) {
            runic_total_spent += utxo.balance;
            btc_in_runic_spent += utxo.utxo.value;
            utxos.push(utxo);
            if runic_total_spent >= amount {
                break;
            }
        }

        if runic_total_spent < amount {
            manager.record_runic_utxos(&addr, runeid.clone(), utxos);
            return Err((amount, fee));
        }
        Ok((utxos, runic_total_spent, btc_in_runic_spent))
    })?;

    let need_change_rune_output = runic_utxos.len() > 1 || runic_total_spent > amount;

    let required_postage_btc = if need_change_rune_output {
        postage.to_sat() * 2
    } else {
        postage.to_sat()
    } - btc_in_runic_spent;

    let (fee_utxos, fee_total_spent) = write_utxo_manager(|manager| {
        let (mut utxos, mut total_spent) = (vec![], 0);

        let addr = fee_payer.to_string();
        while let Some(utxo) = manager.get_bitcoin_utxo(&addr) {
            total_spent += utxo.value;
            utxos.push(utxo);
            if total_spent >= fee + required_postage_btc {
                break;
            }
        }

        if total_spent < fee + required_postage_btc {
            manager.record_btc_utxos(&addr, utxos);
            return Err((amount, fee + required_postage_btc));
        }

        Ok((utxos, total_spent))
    })?;

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

    if need_change_rune_output {
        let runestone = Runestone {
            edicts: vec![Edict {
                id: ordinals::RuneId {
                    block: runeid.block,
                    tx: runeid.tx,
                },
                amount,
                output: 2,
            }],
            ..Default::default()
        };

        output.push(TxOut {
            script_pubkey: runestone.encipher(),
            value: Amount::from_sat(0),
        });

        output.push(TxOut {
            script_pubkey: sender.script_pubkey(),
            value: postage,
        });

        output.push(TxOut {
            script_pubkey: receiver.script_pubkey(),
            value: postage,
        });
    } else {
        output.push(TxOut {
            script_pubkey: receiver.script_pubkey(),
            value: postage,
        });
    }

    let remaining = fee_total_spent - fee - required_postage_btc;

    if remaining > DUST_THRESHOLD {
        output.push(TxOut {
            script_pubkey: fee_payer.script_pubkey(),
            value: Amount::from_sat(remaining),
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
