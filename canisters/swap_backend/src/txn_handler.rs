use bitcoin::{
    absolute::LockTime,
    hashes::Hash,
    script::{Builder, PushBytesBuf},
    sighash::SighashCache,
    transaction::Version,
    Address, Amount, EcdsaSighashType, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut,
    Txid, Witness,
};
use ic_cdk::api::management_canister::bitcoin::{
    bitcoin_send_transaction, SendTransactionRequest, Utxo,
};
use ic_ledger_types::TransferArgs;
use ic_management_canister_types::DerivationPath;
use icrc_ledger_types::icrc1::account::Account;
use ordinals::{Edict, Runestone};

use crate::{
    chains::btc::{
        signer::ecdsa_sign,
        utils::{account_to_derivation_path, derive_public_key, sec1_to_der},
    },
    state::read_config,
    types::{RuneId, RunicUtxo, SubmittedTxidType},
};

pub enum TransactionType {
    Combined {
        runeid: RuneId,
        rune_amount: u128,
        rune_sender: Box<Address>,
        rune_receiver: Box<Address>,
        rune_sender_account: Account,
        runic_utxos: Vec<RunicUtxo>,
        btc_amount: u64,
        btc_sender: Box<Address>,
        btc_receiver: Box<Address>,
        btc_sender_account: Account,
        btc_utxos: Vec<Utxo>,
        fee: u64,
        fee_payer_account: Account,
        fee_utxos: Vec<Utxo>,
        fee_payer: Box<Address>,
        postage: Amount,
    },
    Bitcoin {
        txn: Transaction,
        utxos: Vec<Utxo>,
        sender: Address,
        sender_account: Account,
    },
    Rune {
        rune: RuneId,
        rune_amount: u128,
        runic_utxos: Vec<RunicUtxo>,
        sender: Box<Address>,
        receiver: Box<Address>,
        sender_account: Account,
        fee: u64,
        fee_payer: Box<Address>,
        fee_payer_account: Account,
        fee_utxos: Vec<Utxo>,
        postage: Amount,
    },
    Icp {
        txn: TransferArgs,
    },
}

impl TransactionType {
    pub async fn build_and_submit(&self) -> SubmittedTxidType {
        match self {
            Self::Combined {
                runeid,
                rune_amount,
                rune_sender,
                rune_receiver,
                rune_sender_account,
                runic_utxos,
                btc_amount,
                btc_sender,
                btc_receiver,
                btc_sender_account,
                btc_utxos,
                fee,
                fee_payer_account,
                fee_utxos,
                fee_payer,
                postage,
            } => {
                const DUST_THRESHOLD: u64 = 1_000;

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
                            txid: Txid::from_raw_hash(
                                Hash::from_slice(&utxo.outpoint.txid).unwrap(),
                            ),
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
                btc_utxos.iter().for_each(|utxo| {
                    btc_total_spent += utxo.value;
                    let txin = TxIn {
                        script_sig: ScriptBuf::new(),
                        witness: Witness::new(),
                        sequence: Sequence::MAX,
                        previous_output: OutPoint {
                            txid: Txid::from_raw_hash(
                                Hash::from_slice(&utxo.outpoint.txid).unwrap(),
                            ),
                            vout: utxo.outpoint.vout,
                        },
                    };
                    let len = input.len();
                    index_of_utxos_to_be_signed_by_btc_sender.push(len);
                    input.push(txin);
                });

                if fee_payer == btc_sender {
                    let remaining = btc_total_spent - btc_amount - fee - required_postage_btc;

                    output.push(TxOut {
                        script_pubkey: btc_receiver.script_pubkey(),
                        value: Amount::from_sat(*btc_amount),
                    });

                    if remaining > DUST_THRESHOLD {
                        output.push(TxOut {
                            script_pubkey: btc_sender.script_pubkey(),
                            value: Amount::from_sat(remaining),
                        });
                    }
                } else {
                    output.push(TxOut {
                        script_pubkey: btc_receiver.script_pubkey(),
                        value: Amount::from_sat(*btc_amount),
                    });

                    // fee

                    fee_utxos.iter().for_each(|utxo| {
                        fee_total_spent += utxo.value;
                        let txin = TxIn {
                            sequence: Sequence::MAX,
                            script_sig: ScriptBuf::new(),
                            witness: Witness::new(),
                            previous_output: OutPoint {
                                txid: Txid::from_raw_hash(
                                    Hash::from_slice(&utxo.outpoint.txid).unwrap(),
                                ),
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
                    btc_sender_path,
                    btc_sender_pubkey,
                    fee_payer_path,
                    fee_payer_pubkey,
                ) = read_config(|config| {
                    let ecdsa_key = config.ecdsa_public_key();
                    let rune_sender_path = account_to_derivation_path(rune_sender_account);
                    let rune_sender_pubkey = derive_public_key(&ecdsa_key, &rune_sender_path);
                    let btc_sender_path = account_to_derivation_path(btc_sender_account);
                    let btc_sender_pubkey = derive_public_key(&ecdsa_key, &btc_sender_path);
                    let fee_payer_path = account_to_derivation_path(fee_payer_account);
                    let fee_payer_pubkey = derive_public_key(&ecdsa_key, &fee_payer_path);
                    (
                        DerivationPath::new(rune_sender_path),
                        rune_sender_pubkey.public_key,
                        DerivationPath::new(btc_sender_path),
                        btc_sender_pubkey.public_key,
                        DerivationPath::new(fee_payer_path),
                        fee_payer_pubkey.public_key,
                    )
                });

                let txn_cache = SighashCache::new(txn.clone());

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
                                &btc_sender.script_pubkey(),
                                EcdsaSighashType::All.to_u32(),
                            )
                            .unwrap();
                        let signature = ecdsa_sign(
                            sighash.as_byte_array().to_vec(),
                            btc_sender_path.clone().into_inner(),
                        )
                        .await
                        .signature;
                        let mut signature = sec1_to_der(signature);
                        signature.push(EcdsaSighashType::All.to_u32() as u8);
                        let signature = PushBytesBuf::try_from(signature).unwrap();
                        let pubkey = PushBytesBuf::try_from(btc_sender_pubkey.clone()).unwrap();
                        input.script_sig = Builder::new()
                            .push_slice(signature)
                            .push_slice(pubkey)
                            .into_script();
                        input.witness.clear();
                    } else if index_of_utxos_to_be_signed_by_fee_payer.contains(&index) {
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
                    network: read_config(|config| config.bitcoin_network()),
                    transaction: txn_bytes,
                })
                .await
                .expect("failed to submit bitcoin txn");

                SubmittedTxidType::Bitcoin { txid }
            }
            Self::Bitcoin {
                txn,
                utxos: _,
                sender,
                sender_account,
            } => {
                let (path, pubkey) = read_config(|config| {
                    let ecdsa_key = config.ecdsa_public_key();
                    let path = account_to_derivation_path(sender_account);
                    let pubkey = derive_public_key(&ecdsa_key, &path).public_key;
                    (DerivationPath::new(path), pubkey)
                });
                let mut txn = txn.clone();
                let txn_cache = SighashCache::new(txn.clone());
                for (index, input) in txn.input.iter_mut().enumerate() {
                    let sighash = txn_cache
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
                    network: read_config(|config| config.bitcoin_network()),
                })
                .await
                .unwrap();
                SubmittedTxidType::Bitcoin { txid }
            }
            Self::Rune {
                rune,
                rune_amount,
                runic_utxos,
                sender,
                receiver,
                sender_account,
                fee,
                fee_payer,
                fee_payer_account,
                fee_utxos,
                postage,
            } => {
                const DUST_THRESHOLD: u64 = 1_000;

                let mut runic_total_spent = 0;
                let mut btc_in_runic_spent = 0;
                let mut btc_total_spent = 0;
                let mut fee_total_spent = 0;

                let mut index_of_utxos_to_be_signed_by_rune_sender = vec![];

                let (mut input, mut output) = (vec![], vec![]);

                runic_utxos.iter().for_each(|RunicUtxo { utxo, balance }| {
                    runic_total_spent += balance;
                    btc_in_runic_spent += utxo.value;
                    let txin = TxIn {
                        script_sig: ScriptBuf::new(),
                        witness: Witness::new(),
                        sequence: Sequence::MAX,
                        previous_output: OutPoint {
                            txid: Txid::from_raw_hash(
                                Hash::from_slice(&utxo.outpoint.txid).unwrap(),
                            ),
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
                                block: rune.block,
                                tx: rune.tx,
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
                        script_pubkey: sender.script_pubkey(),
                        value: *postage,
                    });

                    output.push(TxOut {
                        script_pubkey: receiver.script_pubkey(),
                        value: *postage,
                    });
                } else {
                    output.push(TxOut {
                        script_pubkey: receiver.script_pubkey(),
                        value: *postage,
                    });
                }

                // fee

                fee_utxos.iter().for_each(|utxo| {
                    fee_total_spent += utxo.value;
                    let txin = TxIn {
                        sequence: Sequence::MAX,
                        script_sig: ScriptBuf::new(),
                        witness: Witness::new(),
                        previous_output: OutPoint {
                            txid: Txid::from_raw_hash(
                                Hash::from_slice(&utxo.outpoint.txid).unwrap(),
                            ),
                            vout: utxo.outpoint.vout,
                        },
                    };
                    let len = input.len();
                    input.push(txin);
                });

                let remaining = fee_total_spent - fee - required_postage_btc;
                output.push(TxOut {
                    script_pubkey: fee_payer.script_pubkey(),
                    value: Amount::from_sat(remaining),
                });

                let mut txn = Transaction {
                    input,
                    output,
                    version: Version(2),
                    lock_time: LockTime::ZERO,
                };

                // signing

                let (rune_sender_path, rune_sender_pubkey, fee_payer_path, fee_payer_pubkey) =
                    read_config(|config| {
                        let ecdsa_key = config.ecdsa_public_key();
                        let rune_sender_path = account_to_derivation_path(sender_account);
                        let rune_sender_pubkey = derive_public_key(&ecdsa_key, &rune_sender_path);
                        let fee_payer_path = account_to_derivation_path(fee_payer_account);
                        let fee_payer_pubkey = derive_public_key(&ecdsa_key, &fee_payer_path);
                        (
                            DerivationPath::new(rune_sender_path),
                            rune_sender_pubkey.public_key,
                            DerivationPath::new(fee_payer_path),
                            fee_payer_pubkey.public_key,
                        )
                    });

                let txn_cache = SighashCache::new(txn.clone());

                for (index, input) in txn.input.iter_mut().enumerate() {
                    if index_of_utxos_to_be_signed_by_rune_sender.contains(&index) {
                        let sighash = txn_cache
                            .legacy_signature_hash(
                                index,
                                &sender.script_pubkey(),
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
                    network: read_config(|config| config.bitcoin_network()),
                    transaction: txn_bytes,
                })
                .await
                .expect("failed to submit bitcoin txn");

                SubmittedTxidType::Bitcoin { txid }
            }
            Self::Icp { txn } => {
                let txid = ic_ledger_types::transfer(
                    ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                    txn.clone(),
                )
                .await
                .unwrap()
                .unwrap();
                SubmittedTxidType::Ic { txid }
            }
            _ => unimplemented!(),
        }
    }
}
