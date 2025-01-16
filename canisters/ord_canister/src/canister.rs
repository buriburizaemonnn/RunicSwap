use crate::{index::entry::Entry, OutPoint, Txid};
use crate::{rune_id_to_rune_entry, RuneEntry};
use candid::CandidType;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};
use rune_indexer_interface::*;
use serde::Deserialize;
use std::ops::Deref;
use std::str::FromStr;

#[query]
pub fn get_runes_by_utxo(txid: String, vout: u32) -> Result<Vec<RuneBalance>, OrdError> {
    let k = OutPoint::store(OutPoint {
        txid: Txid::from_str(&txid).map_err(|e| OrdError::Params(e.to_string()))?,
        vout,
    });
    let v = crate::outpoint_to_rune_balances(|b| {
        b.get(&k)
            .map(|v| v.deref().iter().map(|i| (*i).into()).collect())
    })
    .unwrap_or_default();
    Ok(v)
}

#[query]
pub fn get_height() -> Result<(u32, String), OrdError> {
    let (height, hash) = crate::highest_block();
    Ok((height, hash.to_string()))
}

#[query(hidden = true)]
pub fn rpc_transform(args: TransformArgs) -> HttpResponse {
    let headers = args
        .response
        .headers
        .into_iter()
        .filter(|h| crate::rpc::should_keep(h.name.as_str()))
        .collect::<Vec<_>>();
    HttpResponse {
        status: args.response.status.clone(),
        body: args.response.body.clone(),
        headers,
    }
}

#[update]
pub fn admin_set_url(url: String) -> Result<(), String> {
    let caller = ic_cdk::api::caller();
    if !ic_cdk::api::is_controller(&caller) {
        return Err("Not authorized".to_string());
    }
    crate::set_url(url);
    Ok(())
}

impl From<(ordinals::RuneId, RuneEntry)> for CandidRuneEntry {
    fn from(
        (
            ordinals::RuneId { block, tx },
            RuneEntry {
                block: entry_block,
                burned: _,
                divisibility,
                etching: _,
                mints: _,
                premine: _,
                spaced_rune,
                symbol,
                terms: _,
                timestamp: _,
                turbo: _,
            },
        ): (ordinals::RuneId, RuneEntry),
    ) -> Self {
        Self {
            runeid: CandidRuneId { block, tx },
            block: entry_block,
            divisibility,
            id: spaced_rune.rune.0,
            runename: spaced_rune.to_string(),
            symbol: match symbol {
                None => None,
                Some(symbol) => Some(symbol as u32),
            },
        }
    }
}

#[derive(CandidType, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct CandidRuneId {
    pub block: u64,
    pub tx: u32,
}

#[derive(CandidType)]
pub struct CandidRuneEntry {
    pub runeid: CandidRuneId,
    pub block: u64,
    pub divisibility: u8,
    pub id: u128,
    pub runename: String,
    pub symbol: Option<u32>,
}

#[query]
pub fn get_rune_entry_by_runeid(runeid: CandidRuneId) -> Option<CandidRuneEntry> {
    let runeid = ordinals::RuneId {
        block: runeid.block,
        tx: runeid.tx,
    };
    rune_id_to_rune_entry(|entries| entries.get(&runeid).map(|entry| (runeid, *entry).into()))
}

#[query]
pub fn get_50_rune_entries() -> Vec<CandidRuneEntry> {
    rune_id_to_rune_entry(|entries| {
        entries
            .iter()
            .take(50)
            .map(|(id, entry)| CandidRuneEntry::from((*id, *entry)))
            .collect()
    })
}

#[query(hidden = true)]
fn http_request(
    req: ic_canisters_http_types::HttpRequest,
) -> ic_canisters_http_types::HttpResponse {
    if ic_cdk::api::data_certificate().is_none() {
        ic_cdk::trap("update call rejected");
    }
    if req.path() == "/logs" {
        crate::ic_log::do_reply(req)
    } else {
        ic_canisters_http_types::HttpResponseBuilder::not_found().build()
    }
}

#[init]
pub fn init(url: String, first_block_hash: String) {
    crate::init_storage();
    crate::set_url(url);
    crate::index::init_rune(&first_block_hash);
    crate::set_first_block_hash(first_block_hash);
    crate::index::sync(1);
}

#[pre_upgrade]
fn pre_upgrade() {
    crate::persistence();
}

#[post_upgrade]
fn post_upgrade() {
    crate::restore();
    crate::index::sync(1);
}

ic_cdk::export_candid!();
