use bitcoin::{
    script::{Builder, PushBytesBuf},
    sighash::EcdsaSighashType,
    Sequence, Transaction, TxIn, Witness,
};
use ic_cdk::api::management_canister::ecdsa::{
    sign_with_ecdsa, SignWithEcdsaArgument, SignWithEcdsaResponse,
};

use crate::state::read_config;

use super::utils::*;

pub fn mock_signature(txn: &Transaction) -> Transaction {
    let pubkey = read_config(|config| {
        let ecdsa_key = config.ecdsa_public_key();
        let path = vec![];
        derive_public_key(&ecdsa_key, &path).public_key
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
