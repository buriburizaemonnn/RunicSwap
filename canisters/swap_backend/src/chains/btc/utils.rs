use crate::EcdsaPublicKey;
use ic_crypto_secp256k1::{DerivationIndex, DerivationPath, PublicKey};
use icrc_ledger_types::icrc1::account::Account;
use serde_bytes::ByteBuf;

use sha2::Digest;

pub fn account_to_derivation_path(account: &Account) -> Vec<ByteBuf> {
    vec![
        ByteBuf::from([1u8]),
        ByteBuf::from(account.owner.as_slice().to_vec()),
        ByteBuf::from(account.effective_subaccount()),
    ]
}

pub fn derive_public_key(ecdsa_public_key: &EcdsaPublicKey, path: &[ByteBuf]) -> EcdsaPublicKey {
    let path = DerivationPath::new(
        path.iter()
            .map(|x| DerivationIndex(x.clone().into_vec()))
            .collect(),
    );

    let pk = PublicKey::deserialize_sec1(&ecdsa_public_key.public_key)
        .expect("failed to ECDSA public key");

    let chain_code: [u8; 32] = ecdsa_public_key
        .chain_code
        .clone()
        .try_into()
        .expect("incorrect chain code size");

    let (derived_public_key, derived_chain_code) =
        pk.derive_subkey_with_chain_code(&path, &chain_code);
    EcdsaPublicKey {
        chain_code: derived_chain_code.to_vec(),
        public_key: derived_public_key.serialize_sec1(true),
    }
}

pub fn sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

pub fn ripemd160(data: &[u8]) -> Vec<u8> {
    let mut hasher = ripemd::Ripemd160::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

// Converts a SEC1 ECDSA signature to the DER format.
pub fn sec1_to_der(sec1_signature: Vec<u8>) -> Vec<u8> {
    let r: Vec<u8> = if sec1_signature[0] & 0x80 != 0 {
        // r is negative. Prepend a zero byte.
        let mut tmp = vec![0x00];
        tmp.extend(sec1_signature[..32].to_vec());
        tmp
    } else {
        // r is positive.
        sec1_signature[..32].to_vec()
    };

    let s: Vec<u8> = if sec1_signature[32] & 0x80 != 0 {
        // s is negative. Prepend a zero byte.
        let mut tmp = vec![0x00];
        tmp.extend(sec1_signature[32..].to_vec());
        tmp
    } else {
        // s is positive.
        sec1_signature[32..].to_vec()
    };

    // Convert signature to DER.
    vec![
        vec![0x30, 4 + r.len() as u8 + s.len() as u8, 0x02, r.len() as u8],
        r,
        vec![0x02, s.len() as u8],
        s,
    ]
    .into_iter()
    .flatten()
    .collect()
}
