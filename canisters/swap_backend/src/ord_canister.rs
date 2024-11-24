use ic_cdk::api::call::CallResult;
use types::*;

use crate::{state::read_config, types::RuneId};

pub mod types;

pub type GetRunesResult = Result<Vec<RuneBalance>, OrdError>;

pub async fn get_runes_by_utxo(txid: String, vout: u32) -> CallResult<(GetRunesResult,)> {
    let ord_canister = read_config(|config| config.ord_canister());
    ic_cdk::call(ord_canister, "get_runes_by_utxo", (txid, vout)).await
}

pub type GetHeightResult = Result<(u32, String), OrdError>;

pub async fn get_height() -> CallResult<(GetHeightResult,)> {
    let ord_canister = read_config(|config| config.ord_canister());
    ic_cdk::call(ord_canister, "get_height", ()).await
}

/*
pub async fn get_rune_entry(runeid: RuneId) -> Option<()> {
    let ord_canister = read_config(|config| config.ord_canister());
    todo!()
}
*/
