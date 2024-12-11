use candid::{CandidType, Decode, Deserialize, Encode};
use ic_cdk::api::management_canister::bitcoin::Utxo;
use ic_stable_structures::{storable::Bound, Storable};

#[derive(CandidType, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RuneId {
    pub block: u64,
    pub tx: u32,
}

impl Storable for RuneId {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct RunicUtxo {
    pub utxo: Utxo,
    pub balance: u128,
}

impl std::hash::Hash for RunicUtxo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.utxo.hash(state)
    }
}

impl std::borrow::Borrow<Utxo> for RunicUtxo {
    fn borrow(&self) -> &Utxo {
        &self.utxo
    }
}

impl Storable for RunicUtxo {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenType {
    Bitcoin,
    Icp,
    CkBTC,
    Runestone(RuneId),
}

impl Storable for TokenType {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType)]
pub enum SubmittedTxidType {
    Bitcoin { txid: String },
    Ic { txid: u64 },
    Icrc1 { txid: u128 },
}
