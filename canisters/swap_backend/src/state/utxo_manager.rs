use std::collections::{HashMap, HashSet};

use candid::{CandidType, Decode, Encode};
use ic_cdk::api::management_canister::bitcoin::Utxo;
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use serde::{Deserialize, Serialize};

use crate::{
    memory::{Memory, MemoryIds},
    types::{RuneId, RunicUtxo},
};

use super::read_memory_manager;

#[derive(CandidType, Deserialize, Default)]
pub struct RunicUtxoMap(HashMap<RuneId, HashSet<RunicUtxo>>);

impl Storable for RunicUtxoMap {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub type RunicMap = StableBTreeMap<String, RunicUtxoMap, Memory>;

pub fn init_runic_map() -> RunicMap {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::Runic.into());
        RunicMap::init(memory)
    })
}

#[derive(CandidType, Deserialize, Default)]
pub struct BitcoinUtxos(HashSet<Utxo>);

impl Storable for BitcoinUtxos {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub type BtcMap = StableBTreeMap<String, BitcoinUtxos, Memory>;

pub fn init_btc_map() -> BtcMap {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::Bitcoin.into());
        BtcMap::init(memory)
    })
}

#[derive(Serialize, Deserialize)]
pub struct UtxoManager {
    #[serde(skip, default = "init_runic_map")]
    pub r: RunicMap,
    #[serde(skip, default = "init_btc_map")]
    pub b: BtcMap,
}

impl Default for UtxoManager {
    fn default() -> Self {
        Self {
            r: init_runic_map(),
            b: init_btc_map(),
        }
    }
}

impl UtxoManager {
    pub fn record_runic_utxos(&mut self, addr: &str, runeid: RuneId, utxos: Vec<RunicUtxo>) {
        let addr = String::from(addr);
        let mut map = self.r.get(&addr).unwrap_or_default().0;
        let mut current_utxos = map.remove(&runeid).unwrap_or_default();
        for utxo in utxos {
            if current_utxos.contains(&utxo) {
                continue;
            }
            current_utxos.insert(utxo);
        }
        map.insert(runeid, current_utxos);
        self.r.insert(addr, RunicUtxoMap(map));
    }

    pub fn record_btc_utxos(&mut self, addr: &str, utxos: Vec<Utxo>) {
        let addr = String::from(addr);
        let mut current_utxos = self.b.get(&addr).unwrap_or_default().0;
        for utxo in utxos {
            if current_utxos.contains(&utxo) {
                continue;
            }
            current_utxos.insert(utxo);
        }
        self.b.insert(addr, BitcoinUtxos(current_utxos));
    }

    pub fn get_bitcoin_utxo(&mut self, addr: &str) -> Option<Utxo> {
        let addr = String::from(addr);
        ic_cdk::println!("checking for utxo with lowest balance");
        let mut utxos = self.b.get(&addr)?.0;
        let min_utxo = utxos.iter().min_by_key(|utxo| utxo.value)?.clone();
        ic_cdk::println!("utxo found with balance of: {}", min_utxo.value);
        utxos.remove(&min_utxo);
        self.b.insert(addr, BitcoinUtxos(utxos));
        Some(min_utxo)
    }

    pub fn get_runic_utxo(&mut self, addr: &str, runeid: RuneId) -> Option<RunicUtxo> {
        let addr = String::from(addr);
        ic_cdk::println!("checking for utxo with lowest balance");
        let mut map = self.r.get(&addr)?.0;
        let mut utxos = map.remove(&runeid).unwrap_or_default();
        let min_utxo = utxos.iter().min_by_key(|utxo| utxo.balance)?.clone();
        ic_cdk::println!("utxo found with balance of: {}", min_utxo.balance);
        utxos.remove(&min_utxo);
        map.insert(runeid, utxos);
        self.r.insert(addr, RunicUtxoMap(map));
        Some(min_utxo)
    }

    pub fn is_recorded_as_runic(&self, addr: &str, utxo: &Utxo) -> bool {
        let addr = String::from(addr);
        let mut flag = false;
        if let Some(map) = self.r.get(&addr) {
            for (_, utxos) in map.0.iter() {
                if utxos.contains(utxo) {
                    flag = true;
                    break;
                }
            }
        }
        flag
    }

    pub fn get_runestone_balance(&self, addr: &str, runeid: &RuneId) -> u128 {
        let addr = String::from(addr);
        let mut balance = 0;
        if let Some(map) = self.r.get(&addr) {
            if let Some(utxos) = map.0.get(runeid) {
                balance = utxos.iter().fold(0, |balance, utxo| balance + utxo.balance);
            }
        }
        balance
    }

    pub fn get_bitcoin_balance(&self, addr: &str) -> u64 {
        let addr = String::from(addr);
        let mut balance = 0;
        if let Some(utxos) = self.b.get(&addr) {
            balance = utxos.0.iter().fold(0, |balance, utxo| balance + utxo.value);
        }
        balance
    }

    pub fn all_rune_with_balances(&self, addr: &str) -> HashMap<RuneId, u128> {
        let addr = String::from(addr);
        let mut balances = HashMap::new();
        if let Some(map) = self.r.get(&addr) {
            for (r, utxos) in map.0.iter() {
                let balance = utxos.iter().fold(0, |balance, utxo| balance + utxo.balance);
                balances.insert(r.clone(), balance);
            }
        }
        balances
    }

    pub fn remove_btc_utxo(&mut self, addr: &str, utxo: &Utxo) {
        let addr = String::from(addr);
        let mut current_utxos = self.b.get(&addr).unwrap_or_default().0;
        ic_cdk::println!("btx utxo's len before removal: {}", current_utxos.len());
        current_utxos.remove(utxo);
        ic_cdk::println!("btc utxo's len after removal: {}", current_utxos.len());
        self.b.insert(addr, BitcoinUtxos(current_utxos));
    }
}
