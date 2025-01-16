use bitcoin::{
    absolute::LockTime,
    hashes::Hash,
    script::{Builder, PushBytesBuf},
    sighash::SighashCache,
    transaction::Version,
    Address, Amount, EcdsaSighashType, LegacySighash, OutPoint, ScriptBuf, Sequence, Transaction,
    TxIn, TxOut, Witness,
};
use candid::CandidType;
use ic_cdk::api::management_canister::bitcoin::{
    bitcoin_send_transaction, SendTransactionRequest, Utxo,
};
use ic_management_canister_types::DerivationPath;
use icrc_ledger_types::icrc1::account::Account;
use ordinals::{Edict, Runestone};

use crate::{
    chain::btc::{
        signer::ecdsa_sign,
        utils::{account_to_derivation_path, derive_public_key, sec1_to_der, slice_to_txid},
        DUST_THRESHOLD,
    },
    state::{read_config, utxo_manager::RunicUtxo},
    updater::RuneId,
};

#[derive(CandidType)]
pub enum SubmittedTxidType {
    Bitcoin { txid: String },
}

pub enum TransactionType {
    Bitcoin {
        utxos: Vec<Utxo>,
        txn: Box<Transaction>,
        sender: Box<Address>,
        sender_account: Account,
    },
    Rune {
        runic_utxos: Vec<RunicUtxo>,
        runeid: RuneId,
        rune_amount: u128,
        rune_sender: Box<Address>,
        rune_receiver: Box<Address>,
        rune_sender_account: Account,
        fee_utxos: Vec<Utxo>,
        fee: u64,
        fee_payer: Box<Address>,
        fee_payer_account: Account,
        postage: Amount,
    },
    Combined {
        runic_utxos: Vec<RunicUtxo>,
        runeid: RuneId,
        rune_amount: u128,
        rune_sender: Box<Address>,
        rune_receiver: Box<Address>,
        rune_sender_account: Account,
        bitcoin_utxos: Vec<Utxo>,
        bitcoin_amount: u64,
        bitcoin_sender: Box<Address>,
        bitcoin_receiver: Box<Address>,
        bitcoin_sender_account: Account,
        fee_utxos: Vec<Utxo>,
        fee: u64,
        fee_payer: Box<Address>,
        fee_payer_account: Account,
        postage: Amount,
    },
}

impl TransactionType {
    pub async fn sign_and_submit(&self) -> Option<SubmittedTxidType> {
        match self {
            Self::Bitcoin {
                utxos: _,
                txn,
                sender,
                sender_account,
            } => {
                let mut txn: Transaction = *txn.clone();
                let (path, pubkey) = read_config(|config| {
                    let ecdsa_pubkey = config.ecdsa_public_key();
                    let path = account_to_derivation_path(sender_account);
                    let pubkey = derive_public_key(&ecdsa_pubkey, &path).public_key;
                    (DerivationPath::new(path), pubkey)
                });

                let txn_cache = SighashCache::new(txn.clone());
                let network = read_config(|config| config.bitcoin_network());

                for (index, input) in txn.input.iter_mut().enumerate() {
                    let sighash: LegacySighash = txn_cache
                        .legacy_signature_hash(
                            index,
                            &sender.script_pubkey(),
                            EcdsaSighashType::All.to_u32(),
                        )
                        .unwrap();

                    let signature =
                        ecdsa_sign(sighash.as_byte_array().to_vec(), path.clone().into_inner())
                            .await
                            .signature;
                    let mut signature = sec1_to_der(signature);
                    signature.push(EcdsaSighashType::All.to_u32() as u8);
                    let signature = PushBytesBuf::try_from(signature).unwrap();
                    let pubkey = PushBytesBuf::try_from(pubkey.clone()).unwrap();
                    input.script_sig = Builder::new()
                        .push_slice(signature)
                        .push_slice(pubkey)
                        .into_script();
                    input.witness.clear();
                }
                let txid = txn.compute_txid().to_string();
                let txn_bytes = bitcoin::consensus::serialize(&txn);
                ic_cdk::println!("{}", hex::encode(&txn_bytes));
                bitcoin_send_transaction(SendTransactionRequest {
                    transaction: txn_bytes,
                    network,
                })
                .await
                .unwrap();
                Some(SubmittedTxidType::Bitcoin { txid })
            }
            Self::Rune {
                runic_utxos,
                runeid,
                rune_amount,
                rune_sender,
                rune_receiver,
                rune_sender_account,
                fee_utxos,
                fee,
                fee_payer,
                fee_payer_account,
                postage,
            } => {
                let mut input = vec![];
                let mut output = vec![];

                let mut runic_total_spent = 0;
                let mut bitcoin_spent_in_runic = 0;
                let mut fee_total_spent = 0;

                let mut indexes_for_sender = vec![];

                runic_utxos.iter().for_each(|RunicUtxo { utxo, balance }| {
                    runic_total_spent += balance;
                    bitcoin_spent_in_runic += utxo.value;
                    let txin = TxIn {
                        sequence: Sequence::MAX,
                        witness: Witness::new(),
                        script_sig: ScriptBuf::new(),
                        previous_output: OutPoint {
                            txid: slice_to_txid(&utxo.outpoint.txid),
                            vout: utxo.outpoint.vout,
                        },
                    };
                    let len = input.len();
                    input.push(txin);
                    indexes_for_sender.push(len);
                });

                let need_change_rune_output =
                    runic_total_spent > *rune_amount || runic_utxos.len() > 1;

                let required_bitcoin_for_postage = if need_change_rune_output {
                    postage.to_sat() * 2
                } else {
                    postage.to_sat()
                };

                if need_change_rune_output {
                    let id = ordinals::RuneId {
                        block: runeid.block,
                        tx: runeid.tx,
                    };
                    let rune = Runestone {
                        edicts: vec![Edict {
                            id,
                            amount: *rune_amount,
                            output: 2,
                        }],
                        ..Default::default()
                    };
                    output.push(TxOut {
                        script_pubkey: rune.encipher(),
                        value: Amount::from_sat(0),
                    });
                    output.push(TxOut {
                        script_pubkey: rune_sender.script_pubkey(),
                        value: *postage,
                    });
                    output.push(TxOut {
                        script_pubkey: rune_receiver.script_pubkey(),
                        value: *postage,
                    });
                } else {
                    output.push(TxOut {
                        script_pubkey: rune_receiver.script_pubkey(),
                        value: *postage,
                    });
                }

                // fee

                fee_utxos.iter().for_each(|utxo| {
                    fee_total_spent += utxo.value;
                    let txin = TxIn {
                        sequence: Sequence::MAX,
                        witness: Witness::new(),
                        script_sig: ScriptBuf::new(),
                        previous_output: OutPoint {
                            txid: slice_to_txid(&utxo.outpoint.txid),
                            vout: utxo.outpoint.vout,
                        },
                    };
                    input.push(txin);
                });

                let remaining = fee_total_spent - fee - required_bitcoin_for_postage;
                if remaining >= DUST_THRESHOLD {
                    output.push(TxOut {
                        script_pubkey: fee_payer.script_pubkey(),
                        value: Amount::from_sat(remaining),
                    });
                }

                let mut txn = Transaction {
                    input,
                    output,
                    version: Version(2),
                    lock_time: LockTime::ZERO,
                };

                // signing the transaction

                let (rune_sender_path, rune_sender_pubkey, fee_payer_path, fee_payer_pubkey) =
                    read_config(|config| {
                        let ecdsa_public_key = config.ecdsa_public_key();
                        let rune_sender_path = account_to_derivation_path(rune_sender_account);
                        let rune_sender_pubkey =
                            derive_public_key(&ecdsa_public_key, &rune_sender_path).public_key;
                        let fee_payer_path = account_to_derivation_path(fee_payer_account);
                        let fee_payer_pubkey =
                            derive_public_key(&ecdsa_public_key, &fee_payer_path).public_key;
                        (
                            DerivationPath::new(rune_sender_path),
                            rune_sender_pubkey,
                            DerivationPath::new(fee_payer_path),
                            fee_payer_pubkey,
                        )
                    });

                let txn_cache = SighashCache::new(txn.clone());
                let network = read_config(|config| config.bitcoin_network());

                for (index, input) in txn.input.iter_mut().enumerate() {
                    if indexes_for_sender.contains(&index) {
                        let sighash = txn_cache
                            .legacy_signature_hash(
                                index,
                                &rune_sender.script_pubkey(),
                                EcdsaSighashType::All.to_u32(),
                            )
                            .unwrap();
                        let signature = ecdsa_sign(
                            sighash.as_byte_array().to_vec(),
                            rune_sender_path.clone().into_inner(),
                        )
                        .await
                        .signature;
                        let mut signature = sec1_to_der(signature);
                        signature.push(EcdsaSighashType::All.to_u32() as u8);
                        let signature = PushBytesBuf::try_from(signature).unwrap();
                        let pubkey = PushBytesBuf::try_from(rune_sender_pubkey.clone()).unwrap();

                        input.script_sig = Builder::new()
                            .push_slice(signature)
                            .push_slice(pubkey)
                            .into_script();
                        input.witness.clear();
                    } else {
                        let sighash = txn_cache
                            .legacy_signature_hash(
                                index,
                                &fee_payer.script_pubkey(),
                                EcdsaSighashType::All.to_u32(),
                            )
                            .unwrap();
                        let signature = ecdsa_sign(
                            sighash.as_byte_array().to_vec(),
                            fee_payer_path.clone().into_inner(),
                        )
                        .await
                        .signature;
                        let mut signature = sec1_to_der(signature);
                        signature.push(EcdsaSighashType::All.to_u32() as u8);
                        let signature = PushBytesBuf::try_from(signature).unwrap();
                        let pubkey = PushBytesBuf::try_from(fee_payer_pubkey.clone()).unwrap();

                        input.script_sig = Builder::new()
                            .push_slice(signature)
                            .push_slice(pubkey)
                            .into_script();
                        input.witness.clear();
                    }
                }

                let txid = txn.compute_txid().to_string();
                let txn_bytes = bitcoin::consensus::serialize(&txn);
                ic_cdk::println!("{}", hex::encode(&txn_bytes));

                bitcoin_send_transaction(SendTransactionRequest {
                    transaction: txn_bytes,
                    network,
                })
                .await
                .unwrap();

                Some(SubmittedTxidType::Bitcoin { txid })
            }
            Self::Combined {
                runic_utxos,
                runeid,
                rune_amount,
                rune_sender,
                rune_receiver,
                rune_sender_account,
                bitcoin_utxos,
                bitcoin_amount,
                bitcoin_sender,
                bitcoin_receiver,
                bitcoin_sender_account,
                fee_utxos,
                fee,
                fee_payer,
                fee_payer_account,
                postage,
            } => {
                /* let mut input = vec![];
                let mut output = vec![];

                let mut runic_total_spent = 0;
                let mut bitcoin_spent_in_runic = 0;
                let mut bitcoin_total_spent = 0;
                let mut fee_total_spent = 0;

                let mut indexes_for_rune_sender = vec![];
                let mut indexes_for_bitcoin_sender = vec![];

                // runic
                runic_utxos.iter().for_each(|RunicUtxo { utxo, balance }| {
                    runic_total_spent += balance;
                    bitcoin_spent_in_runic += utxo.value;
                    let txin = TxIn {
                        sequence: Sequence::MAX,
                        witness: Witness::new(),
                        script_sig: ScriptBuf::new(),
                        previous_output: OutPoint {
                            txid: slice_to_txid(&utxo.outpoint.txid),
                            vout: utxo.outpoint.vout,
                        },
                    };
                    let len = input.len();
                    indexes_for_rune_sender.push(len);
                    input.push(txin);
                });

                let need_change_rune_output =
                    runic_total_spent < *rune_amount || runic_utxos.len() > 1;

                let required_bitcoin_for_postage = if need_change_rune_output {
                    postage.to_sat() * 2
                } else {
                    postage.to_sat()
                };

                if need_change_rune_output {
                    let id = ordinals::RuneId {
                        block: runeid.block,
                        tx: runeid.tx,
                    };
                    let rune = Runestone {
                        edicts: vec![Edict {
                            id,
                            amount: *rune_amount,
                            output: 2,
                        }],
                        ..Default::default()
                    };
                    output.push(TxOut {
                        script_pubkey: rune.encipher(),
                        value: Amount::from_sat(0),
                    });
                    output.push(TxOut {
                        script_pubkey: rune_sender.script_pubkey(),
                        value: *postage,
                    });
                    output.push(TxOut {
                        script_pubkey: rune_receiver.script_pubkey(),
                        value: *postage,
                    });
                } else {
                    output.push(TxOut {
                        script_pubkey: rune_receiver.script_pubkey(),
                        value: *postage,
                    });
                }

                if bitcoin_sender == fee_payer {
                    // bitcoin
                    bitcoin_utxos.iter().for_each(|utxo| {
                        bitcoin_total_spent += utxo.value;
                        let txin = TxIn {
                            sequence: Sequence::MAX,
                            witness: Witness::new(),
                            script_sig: ScriptBuf::new(),
                            previous_output: OutPoint {
                                txid: slice_to_txid(&utxo.outpoint.txid),
                                vout: utxo.outpoint.vout,
                            },
                        };
                        let len = input.len();
                        indexes_for_bitcoin_sender.push(len);
                        input.push(txin);
                    });

                    output.push(TxOut {
                        value: Amount::from_sat(*bitcoin_amount),
                        script_pubkey: bitcoin_receiver.script_pubkey(),
                    });

                    let remaining = bitcoin_total_spent - fee - required_bitcoin_for_postage;

                    if remaining >= DUST_THRESHOLD {
                        output.push(TxOut {
                            value: Amount::from_sat(remaining),
                            script_pubkey: bitcoin_sender.script_pubkey(),
                        });
                    }
                } else {

                    // bitcoin
                    bitcoin_utxos.iter().for_each(|utxo| {
                        bitcoin_total_spent += utxo.value;
                        let txin = TxIn {
                            sequence: Sequence::MAX,
                            witness: Witness::new(),
                            script_sig: ScriptBuf::new(),
                            previous_output: OutPoint {
                                txid: slice_to_txid(&utxo.outpoint.txid),
                                vout: utxo.outpoint.vout,
                            },
                        };
                        let len = input.len();
                        indexes_for_bitcoin_sender.push(len);
                        input.push(txin);
                    });

                    let remaining_bitcoin = bitcoin_total_spent - bitcoin_amount;

                    output.push(TxOut {
                        value: Amount::from_sat(*bitcoin_amount),
                        script_pubkey: bitcoin_receiver.script_pubkey(),
                    });

                    if remaining_bitcoin >= DUST_THRESHOLD {
                        output.push(TxOut {
                            value: Amount::from_sat(remaining_bitcoin),
                            script_pubkey: bitcoin_sender.script_pubkey(),
                        });
                    }

                    // fee
                    fee_utxos.iter().for_each(|utxo| {
                        fee_total_spent += utxo.value;
                        let txin = TxIn {
                            sequence: Sequence::MAX,
                            witness: Witness::new(),
                            script_sig: ScriptBuf::new(),
                            previous_output: OutPoint {
                                txid: slice_to_txid(&utxo.outpoint.txid),
                                vout: utxo.outpoint.vout,
                            },
                        };
                        input.push(txin);
                    });

                    let remaining_fee = fee_total_spent - fee - required_bitcoin_for_postage;
                    if remaining_fee > DUST_THRESHOLD {
                        output.push(TxOut {
                            value: Amount::from_sat(remaining_fee),
                            script_pubkey: fee_payer.script_pubkey(),
                        });
                    }
                }

                let mut txn = Transaction {
                    input,
                    output,
                    lock_time: LockTime::ZERO,
                    version: Version(2),
                }; */

                let mut runic_total_spent = 0;
                let mut btc_in_runic_spent = 0;
                let mut btc_total_spent = 0;
                let mut fee_total_spent = 0;

                let mut index_of_utxos_to_be_signed_by_rune_sender = vec![];
                let mut index_of_utxos_to_be_signed_by_btc_sender = vec![];
                let mut index_of_utxos_to_be_signed_by_fee_payer = vec![];

                let (mut input, mut output) = (vec![], vec![]);

                runic_utxos.iter().for_each(|RunicUtxo { utxo, balance }| {
                    runic_total_spent += balance;
                    btc_in_runic_spent += utxo.value;
                    let txin = TxIn {
                        script_sig: ScriptBuf::new(),
                        witness: Witness::new(),
                        sequence: Sequence::MAX,
                        previous_output: OutPoint {
                            txid: slice_to_txid(&utxo.outpoint.txid),
                            vout: utxo.outpoint.vout,
                        },
                    };
                    let len = input.len();
                    index_of_utxos_to_be_signed_by_rune_sender.push(len);
                    input.push(txin);
                });

                let need_change_rune_output =
                    runic_total_spent > *rune_amount || runic_utxos.len() > 1;
                let required_postage_btc = if need_change_rune_output {
                    postage.to_sat() * 2
                } else {
                    postage.to_sat()
                } - btc_in_runic_spent;

                if need_change_rune_output {
                    let runestone = Runestone {
                        edicts: vec![Edict {
                            id: ordinals::RuneId {
                                block: runeid.block,
                                tx: runeid.tx,
                            },
                            amount: *rune_amount,
                            output: 2,
                        }],
                        ..Default::default()
                    };
                    output.push(TxOut {
                        script_pubkey: runestone.encipher(),
                        value: Amount::from_sat(0),
                    });

                    output.push(TxOut {
                        script_pubkey: rune_sender.script_pubkey(),
                        value: *postage,
                    });

                    output.push(TxOut {
                        script_pubkey: rune_receiver.script_pubkey(),
                        value: *postage,
                    });
                } else {
                    output.push(TxOut {
                        script_pubkey: rune_receiver.script_pubkey(),
                        value: *postage,
                    });
                }

                // btc
                bitcoin_utxos.iter().for_each(|utxo| {
                    btc_total_spent += utxo.value;
                    let txin = TxIn {
                        script_sig: ScriptBuf::new(),
                        witness: Witness::new(),
                        sequence: Sequence::MAX,
                        previous_output: OutPoint {
                            txid: slice_to_txid(&utxo.outpoint.txid),
                            vout: utxo.outpoint.vout,
                        },
                    };
                    let len = input.len();
                    index_of_utxos_to_be_signed_by_btc_sender.push(len);
                    input.push(txin);
                });

                if fee_payer == bitcoin_sender {
                    let remaining = btc_total_spent - bitcoin_amount - fee - required_postage_btc;

                    output.push(TxOut {
                        script_pubkey: bitcoin_receiver.script_pubkey(),
                        value: Amount::from_sat(*bitcoin_amount),
                    });

                    if remaining > DUST_THRESHOLD {
                        output.push(TxOut {
                            script_pubkey: bitcoin_sender.script_pubkey(),
                            value: Amount::from_sat(remaining),
                        });
                    }
                } else {
                    output.push(TxOut {
                        script_pubkey: bitcoin_receiver.script_pubkey(),
                        value: Amount::from_sat(*bitcoin_amount),
                    });

                    // fee

                    fee_utxos.iter().for_each(|utxo| {
                        fee_total_spent += utxo.value;
                        let txin = TxIn {
                            sequence: Sequence::MAX,
                            script_sig: ScriptBuf::new(),
                            witness: Witness::new(),
                            previous_output: OutPoint {
                                txid: slice_to_txid(&utxo.outpoint.txid),
                                vout: utxo.outpoint.vout,
                            },
                        };
                        let len = input.len();
                        index_of_utxos_to_be_signed_by_fee_payer.push(len);
                        input.push(txin);
                    });

                    let remaining = fee_total_spent - fee - required_postage_btc;
                    output.push(TxOut {
                        script_pubkey: fee_payer.script_pubkey(),
                        value: Amount::from_sat(remaining),
                    });
                }

                let mut txn = Transaction {
                    input,
                    output,
                    version: Version(2),
                    lock_time: LockTime::ZERO,
                };

                // signing

                let (
                    rune_sender_path,
                    rune_sender_pubkey,
                    bitcoin_sender_path,
                    bitcoin_sender_pubkey,
                    fee_payer_path,
                    fee_payer_pubkey,
                ) = read_config(|config| {
                    let ecdsa_public_key = config.ecdsa_public_key();
                    let rune_sender_path = account_to_derivation_path(rune_sender_account);
                    let rune_sender_pubkey =
                        derive_public_key(&ecdsa_public_key, &rune_sender_path).public_key;
                    let bitcoin_sender_path = account_to_derivation_path(bitcoin_sender_account);
                    let bitcoin_sender_pubkey =
                        derive_public_key(&ecdsa_public_key, &bitcoin_sender_path).public_key;
                    let fee_payer_path = account_to_derivation_path(fee_payer_account);
                    let fee_payer_pubkey =
                        derive_public_key(&ecdsa_public_key, &fee_payer_path).public_key;
                    (
                        DerivationPath::new(rune_sender_path),
                        rune_sender_pubkey,
                        DerivationPath::new(bitcoin_sender_path),
                        bitcoin_sender_pubkey,
                        DerivationPath::new(fee_payer_path),
                        fee_payer_pubkey,
                    )
                });

                let txn_cache = SighashCache::new(txn.clone());

                let network = read_config(|config| config.bitcoin_network());

                for (index, input) in txn.input.iter_mut().enumerate() {
                    if index_of_utxos_to_be_signed_by_rune_sender.contains(&index) {
                        let sighash = txn_cache
                            .legacy_signature_hash(
                                index,
                                &rune_sender.script_pubkey(),
                                EcdsaSighashType::All.to_u32(),
                            )
                            .unwrap();
                        let signature = ecdsa_sign(
                            sighash.as_byte_array().to_vec(),
                            rune_sender_path.clone().into_inner(),
                        )
                        .await
                        .signature;
                        let mut signature = sec1_to_der(signature);
                        signature.push(EcdsaSighashType::All.to_u32() as u8);
                        let signature = PushBytesBuf::try_from(signature).unwrap();
                        let pubkey = PushBytesBuf::try_from(rune_sender_pubkey.clone()).unwrap();

                        input.script_sig = Builder::new()
                            .push_slice(signature)
                            .push_slice(pubkey)
                            .into_script();
                        input.witness.clear();
                    } else if index_of_utxos_to_be_signed_by_btc_sender.contains(&index) {
                        let sighash = txn_cache
                            .legacy_signature_hash(
                                index,
                                &bitcoin_sender.script_pubkey(),
                                EcdsaSighashType::All.to_u32(),
                            )
                            .unwrap();
                        let signature = ecdsa_sign(
                            sighash.as_byte_array().to_vec(),
                            bitcoin_sender_path.clone().into_inner(),
                        )
                        .await
                        .signature;
                        let mut signature = sec1_to_der(signature);
                        signature.push(EcdsaSighashType::All.to_u32() as u8);
                        let signature = PushBytesBuf::try_from(signature).unwrap();
                        let pubkey = PushBytesBuf::try_from(bitcoin_sender_pubkey.clone()).unwrap();

                        input.script_sig = Builder::new()
                            .push_slice(signature)
                            .push_slice(pubkey)
                            .into_script();
                        input.witness.clear();
                    } else {
                        let sighash = txn_cache
                            .legacy_signature_hash(
                                index,
                                &fee_payer.script_pubkey(),
                                EcdsaSighashType::All.to_u32(),
                            )
                            .unwrap();
                        let signature = ecdsa_sign(
                            sighash.as_byte_array().to_vec(),
                            fee_payer_path.clone().into_inner(),
                        )
                        .await
                        .signature;
                        let mut signature = sec1_to_der(signature);
                        signature.push(EcdsaSighashType::All.to_u32() as u8);
                        let signature = PushBytesBuf::try_from(signature).unwrap();
                        let pubkey = PushBytesBuf::try_from(fee_payer_pubkey.clone()).unwrap();

                        input.script_sig = Builder::new()
                            .push_slice(signature)
                            .push_slice(pubkey)
                            .into_script();
                        input.witness.clear();
                    }
                }

                let txid = txn.compute_txid().to_string();
                let txn_bytes = bitcoin::consensus::serialize(&txn);
                ic_cdk::println!("{}", hex::encode(&txn_bytes));

                bitcoin_send_transaction(SendTransactionRequest {
                    transaction: txn_bytes,
                    network,
                })
                .await
                .unwrap();

                Some(SubmittedTxidType::Bitcoin { txid })
            }
        }
    }
}
