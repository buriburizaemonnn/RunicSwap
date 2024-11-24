use candid::CandidType;
use serde::Deserialize;

use crate::types::RuneId;

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
    Index,
    Rpc(RpcError),
}
