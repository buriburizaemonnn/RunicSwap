use bitcoin::{
    absolute::LockTime,
    hashes::Hash,
    script::{Builder, PushBytesBuf},
    sighash::{EcdsaSighashType, SighashCache},
    Address, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};
use ic_cdk::api::management_canister::bitcoin::{
    bitcoin_send_transaction, SendTransactionRequest, Utxo,
};
use ic_management_canister_types::DerivationPath;
use icrc_ledger_types::icrc1::account::Account;
use ordinals::{Edict, Runestone};

use crate::{
    chains::{
        self,
        bitcoin::{
            transaction::ecdsa_sign,
            utils::{account_to_derivation_path, derive_public_key, sec1_to_der},
        },
    },
    state::{read_config, utxo_manager::RunicUtxo},
    types::{RuneId, SubmittedTransactionIdType},
};

pub enum TransactionType {
    Runestone {
        runic_utxos: Vec<RunicUtxo>,
        bitcoin_utxos: Vec<Utxo>,
        addr: String,
        signer_account: Account,
        signer_address: Address,
        txn: Transaction,
    },
    LegoRunestoneTransaction {
        runeid: RuneId,
        amount: u128,
        paying_fee_amount: u64,
        postage: u64,
        to_be_signed_by_sender: Vec<RunicUtxo>,
        to_be_signed_by_receiver: Vec<Utxo>,
        sender_addr: String,
        receiver_addr: String,
        sender_account: Account,
        receiver_account: Account,
        sender_address: Address,
        receiver_address: Address,
    },
    Bitcoin {
        utxos: Vec<Utxo>,
        addr: String,
        signer_account: Account,
        signer_address: Address,
        txn: Transaction,
    },
    Icp(ic_ledger_types::TransferArgs),
    Icrc1(icrc_ledger_types::icrc1::transfer::TransferArg),
}

// TODO: better error handling
impl TransactionType {
    pub async fn build_and_submit(&self) -> Option<SubmittedTransactionIdType> {
        match self {
            Self::Icp(txn) => {
                let txid = ic_ledger_types::transfer(
                    ic_ledger_types::MAINNET_LEDGER_CANISTER_ID,
                    txn.clone(),
                )
                .await
                .expect("Failed to call canister")
                .expect("transfer failed");
                Some(SubmittedTransactionIdType::Icp { txid })
            }
            Self::Icrc1(_txn) => {
                unimplemented!()
            }
            Self::Bitcoin {
                utxos: _,
                addr: _,
                signer_account,
                signer_address,
                txn,
            } => {
                let signed_transaction = chains::bitcoin::transaction::sign_transaction(
                    signer_account,
                    signer_address,
                    txn.clone(),
                )
                .await;
                let txid = signed_transaction.txid().to_string();
                let txn_bytes = bitcoin::consensus::serialize(&signed_transaction);
                ic_cdk::println!("{:?}", txn_bytes);
                bitcoin_send_transaction(SendTransactionRequest {
                    transaction: txn_bytes,
                    network: read_config(|config| config.bitcoin_network()),
                })
                .await
                .expect("failed to submit transaction");
                Some(SubmittedTransactionIdType::Bitcoin { txid })
            }
            Self::Runestone {
                runic_utxos: _,
                bitcoin_utxos: _,
                addr: _,
                signer_account,
                signer_address,
                txn,
            } => {
                let signed_transaction = chains::bitcoin::transaction::sign_transaction(
                    signer_account,
                    signer_address,
                    txn.clone(),
                )
                .await;
                let txid = signed_transaction.txid().to_string();
                let txn_bytes = bitcoin::consensus::serialize(&signed_transaction);
                ic_cdk::println!("{:?}", txn_bytes);
                bitcoin_send_transaction(SendTransactionRequest {
                    network: read_config(|config| config.bitcoin_network()),
                    transaction: txn_bytes,
                })
                .await
                .expect("failed to submit transaction");
                Some(SubmittedTransactionIdType::Bitcoin { txid })
            }
            Self::LegoRunestoneTransaction {
                runeid,
                amount,
                paying_fee_amount,
                postage,
                to_be_signed_by_sender,
                to_be_signed_by_receiver,
                sender_addr: _,
                receiver_addr: _,
                sender_account,
                receiver_account,
                sender_address,
                receiver_address,
            } => {
                let inputs_count = to_be_signed_by_receiver.len() + to_be_signed_by_sender.len();
                let mut input = Vec::<TxIn>::with_capacity(inputs_count);
                let mut btc_in_r_input = 0;
                let mut rune_balance_in_input = 0;
                for utxo in to_be_signed_by_sender.iter() {
                    btc_in_r_input += utxo.utxo.value;
                    rune_balance_in_input += utxo.balance;
                    input.push(TxIn {
                        previous_output: OutPoint {
                            vout: utxo.utxo.outpoint.vout,
                            txid: Txid::from_raw_hash(
                                Hash::from_slice(&utxo.utxo.outpoint.txid).expect("should hash"),
                            ),
                        },
                        script_sig: ScriptBuf::new(),
                        sequence: Sequence::MAX,
                        witness: Witness::new(),
                    })
                }

                // fee covering utxo
                let mut btc_in_f_input = 0;
                for utxo in to_be_signed_by_receiver.iter() {
                    btc_in_f_input += utxo.value;
                    input.push(TxIn {
                        previous_output: OutPoint {
                            txid: Txid::from_raw_hash(
                                Hash::from_slice(&utxo.outpoint.txid).expect("should hash"),
                            ),
                            vout: utxo.outpoint.vout,
                        },
                        script_sig: ScriptBuf::new(),
                        sequence: Sequence::MAX,
                        witness: Witness::new(),
                    })
                }

                let need_change_rune_output =
                    rune_balance_in_input > *amount || to_be_signed_by_sender.len() > 1;

                let runestone = Runestone {
                    edicts: vec![Edict {
                        id: ordinals::RuneId {
                            block: runeid.block,
                            tx: runeid.tx,
                        },
                        amount: *amount,
                        output: 2,
                    }],
                    ..Default::default()
                };

                let mut output = if need_change_rune_output {
                    vec![
                        TxOut {
                            script_pubkey: runestone.encipher(),
                            value: 0,
                        },
                        TxOut {
                            script_pubkey: sender_address.script_pubkey(),
                            value: *postage,
                        },
                        TxOut {
                            script_pubkey: receiver_address.script_pubkey(),
                            value: *postage,
                        },
                    ]
                } else {
                    vec![TxOut {
                        script_pubkey: receiver_address.script_pubkey(),
                        value: *postage,
                    }]
                };

                let remaining_btc = btc_in_f_input - btc_in_r_input - paying_fee_amount;
                if remaining_btc > 1_000 {
                    output.push(TxOut {
                        value: remaining_btc,
                        script_pubkey: receiver_address.script_pubkey(),
                    })
                }

                let mut txn = Transaction {
                    input,
                    output,
                    lock_time: LockTime::ZERO,
                    version: 2,
                };

                // signing the transaction
                let (sender_path, sender_pubkey, receiver_path, receiver_pubkey) =
                    read_config(|config| {
                        let ecdsa_key = config.ecdsa_key();
                        let sender_path = account_to_derivation_path(sender_account);
                        let receiver_path = account_to_derivation_path(receiver_account);
                        let sender_pubkey =
                            derive_public_key(ecdsa_key.clone(), &sender_path).public_key;
                        let receiver_pubkey =
                            derive_public_key(ecdsa_key, &receiver_path).public_key;
                        (
                            DerivationPath::new(sender_path),
                            sender_pubkey,
                            DerivationPath::new(receiver_path),
                            receiver_pubkey,
                        )
                    });

                let txn_cache = SighashCache::new(txn.clone());
                for (index, input) in txn.input.iter_mut().enumerate() {
                    if index < to_be_signed_by_sender.len() {
                        let sighash = txn_cache
                            .legacy_signature_hash(
                                index,
                                &sender_address.script_pubkey(),
                                EcdsaSighashType::All.to_u32(),
                            )
                            .unwrap();
                        let signature = ecdsa_sign(
                            sighash.to_raw_hash().to_byte_array().to_vec(),
                            sender_path.clone().into_inner(),
                        )
                        .await
                        .signature;
                        let mut signature = sec1_to_der(signature);
                        signature.push(EcdsaSighashType::All.to_u32() as u8);
                        let signature = PushBytesBuf::try_from(signature).unwrap();
                        let pubkey = PushBytesBuf::try_from(sender_pubkey.clone()).unwrap();
                        input.script_sig = Builder::new()
                            .push_slice(signature)
                            .push_slice(pubkey)
                            .into_script();
                        input.witness.clear();
                    } else {
                        let sighash = txn_cache
                            .legacy_signature_hash(
                                index,
                                &receiver_address.script_pubkey(),
                                EcdsaSighashType::All.to_u32(),
                            )
                            .unwrap();
                        let signature = ecdsa_sign(
                            sighash.to_raw_hash().to_byte_array().to_vec(),
                            receiver_path.clone().into_inner(),
                        )
                        .await
                        .signature;
                        let mut signature = sec1_to_der(signature);
                        signature.push(EcdsaSighashType::All.to_u32() as u8);
                        let signature = PushBytesBuf::try_from(signature).unwrap();
                        let pubkey = PushBytesBuf::try_from(receiver_pubkey.clone()).unwrap();
                        input.script_sig = Builder::new()
                            .push_slice(signature)
                            .push_slice(pubkey)
                            .into_script();
                        input.witness.clear();
                    };
                }

                let txid = txn.txid().to_string();
                let txn_bytes = bitcoin::consensus::serialize(&txn);
                ic_cdk::println!("{:?}", txn_bytes);
                bitcoin_send_transaction(SendTransactionRequest {
                    network: read_config(|config| config.bitcoin_network()),
                    transaction: txn_bytes,
                })
                .await
                .expect("failed to submit transaction");
                Some(SubmittedTransactionIdType::Bitcoin { txid })
            }
        }
    }

    pub fn cancel(self) {}
}
