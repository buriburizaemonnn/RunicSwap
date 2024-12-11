use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::call::CallResult;

use crate::types::RuneId;

const ORD_CANISTER: &str = "o25oi-jaaaa-aaaal-ajj6a-cai";

#[derive(CandidType, Deserialize, Debug)]
pub struct RuneBalance {
    pub id: RuneId,
    pub balance: u128,
}

#[derive(CandidType, Deserialize)]
pub enum MintError {
    Cap(u128),
    End(u64),
    Start(u64),
    Unmintable,
}

#[derive(CandidType, Deserialize)]
pub enum RpcError {
    Io(String, String, String),
    Decode(String, String, String),
    Endpoint(String, String, String),
}

#[derive(CandidType, Deserialize)]
pub enum OrdError {
    Params(String),
    Overflow,
    BlockVerification(u32),
    Index(MintError),
    Rpc(RpcError),
}

pub type GetRunesResult = Result<Vec<RuneBalance>, OrdError>;

pub async fn get_runes_by_utxo(txid: String, vout: u32) -> CallResult<(GetRunesResult,)> {
    let ord_canister = Principal::from_text(ORD_CANISTER).unwrap();
    ic_cdk::call(ord_canister, "get_runes_by_utxo", (txid, vout)).await
}
