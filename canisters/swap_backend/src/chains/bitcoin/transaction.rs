use bitcoin::{
    absolute::LockTime,
    hashes::Hash,
    script::{Builder, PushBytesBuf},
    sighash::{EcdsaSighashType, SighashCache},
    Address, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};
use ic_cdk::api::management_canister::{
    bitcoin::Utxo,
    ecdsa::{sign_with_ecdsa, SignWithEcdsaArgument, SignWithEcdsaResponse},
};
use ic_management_canister_types::DerivationPath;
use icrc_ledger_types::icrc1::account::Account;

use crate::{
    state::{read_config, write_utxo_state},
    transaction_handler::TransactionType,
};

use super::utils::{account_to_derivation_path, derive_public_key, sec1_to_der};

pub fn transfer(
    addr: &str,
    from: &Address,
    from_account: &Account,
    to: &Address,
    amount: u64,
    fee_per_vbytes: u64,
    paid_by_sender: bool,
) -> TransactionType {
    let mut total_fee = 0;
    loop {
        let (transaction, utxos) =
            build_transaction_with_fee(addr, from, to, amount, total_fee, paid_by_sender)
                .expect("should build the transaction");
        let signed_transaction = mock_signature(&transaction);

        let txn_vsize = signed_transaction.vsize() as u64;
        if (txn_vsize * fee_per_vbytes) / 1000 == total_fee {
            return TransactionType::Bitcoin {
                utxos,
                addr: addr.to_string(),
                signer_account: from_account.clone(),
                signer_address: from.clone(),
                txn: transaction,
            };
        } else {
            total_fee = (txn_vsize * fee_per_vbytes) / 1000;
        }
    }
}

fn build_transaction_with_fee(
    addr: &str,
    from: &Address,
    to: &Address,
    amount: u64,
    fee: u64,
    paid_by_sender: bool,
) -> Result<(Transaction, Vec<Utxo>), String> {
    const DUST_THRESHOLD: u64 = 1_000;
    let total_amount = if paid_by_sender { amount + fee } else { amount };
    let (utxos_to_spend, total_spent) = write_utxo_state(|state| {
        let mut utxos = vec![];
        let mut value = 0u64;
        while let Some(utxo) = state.get_btc_utxo(addr) {
            value += utxo.value;
            utxos.push(utxo);
            if value > total_amount {
                break;
            }
        }
        if value < total_amount {
            state.record_bitcoin_utxos(addr, utxos);
            return Err(String::from("Not enough balance"));
        }
        Ok((utxos, value))
    })?;

    let input: Vec<TxIn> = utxos_to_spend
        .iter()
        .map(|utxo| TxIn {
            previous_output: OutPoint {
                txid: Txid::from_raw_hash(
                    Hash::from_slice(&utxo.outpoint.txid).expect("should return hash"),
                ),
                vout: utxo.outpoint.vout,
            },
            sequence: Sequence::max_value(),
            witness: Witness::new(),
            script_sig: ScriptBuf::new(),
        })
        .collect();
    let mut output = vec![TxOut {
        script_pubkey: to.script_pubkey(),
        value: if paid_by_sender { amount } else { amount - fee },
    }];
    let remaining = total_spent - total_amount;
    if remaining > DUST_THRESHOLD {
        output.push(TxOut {
            value: remaining,
            script_pubkey: from.script_pubkey(),
        });
    }
    let transaction = Transaction {
        input,
        output,
        lock_time: LockTime::ZERO,
        version: 2,
    };
    Ok((transaction, utxos_to_spend))
}

pub fn mock_signature(txn: &Transaction) -> Transaction {
    let pubkey = read_config(|config| {
        let ecdsa_key = config
            .ecdsa_key
            .as_ref()
            .expect("Canister's config unitialized")
            .clone();
        let path = vec![];
        derive_public_key(ecdsa_key, &path).public_key
    });
    let input = txn
        .input
        .iter()
        .map(|input| {
            let signature = vec![255; 64];
            let mut der_signature = sec1_to_der(signature);
            der_signature.push(EcdsaSighashType::All.to_u32() as u8);
            let signature_as_pushbytes = PushBytesBuf::try_from(der_signature).unwrap();
            let publickey_as_pushbytes = PushBytesBuf::try_from(pubkey.clone()).unwrap();
            TxIn {
                previous_output: input.previous_output,
                witness: Witness::new(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                script_sig: Builder::new()
                    .push_slice(signature_as_pushbytes)
                    .push_slice(publickey_as_pushbytes)
                    .into_script(),
            }
        })
        .collect::<Vec<TxIn>>();
    Transaction {
        input,
        output: txn.output.clone(),
        version: txn.version,
        lock_time: txn.lock_time,
    }
}

pub async fn sign_transaction(
    account: &Account,
    signer: &Address,
    mut txn: Transaction,
) -> Transaction {
    let (path, pubkey) = read_config(|config| {
        let ecdsa_key = config.ecdsa_key();
        let path = account_to_derivation_path(account);
        let ecdsa_pubkey = derive_public_key(ecdsa_key, &path).public_key;
        (DerivationPath::new(path), ecdsa_pubkey)
    });
    let txn_cache = SighashCache::new(txn.clone());
    for (index, input) in txn.input.iter_mut().enumerate() {
        let sighash = txn_cache
            .legacy_signature_hash(
                index,
                &signer.script_pubkey(),
                EcdsaSighashType::All.to_u32(),
            )
            .unwrap();
        let signature = ecdsa_sign(
            sighash.to_raw_hash().to_byte_array().to_vec(),
            path.clone().into_inner(),
        )
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
    txn
}

pub async fn ecdsa_sign(
    message_hash: Vec<u8>,
    derivation_path: Vec<Vec<u8>>,
) -> SignWithEcdsaResponse {
    let key_id = read_config(|config| config.ecdsakeyid());

    sign_with_ecdsa(SignWithEcdsaArgument {
        message_hash,
        derivation_path,
        key_id,
    })
    .await
    .unwrap()
    .0
}
