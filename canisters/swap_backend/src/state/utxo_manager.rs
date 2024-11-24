use std::collections::{HashMap, HashSet};

use candid::{CandidType, Decode, Encode};
use ic_cdk::api::management_canister::bitcoin::Utxo;
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use serde::{Deserialize, Serialize};

use crate::types::RuneId;

use super::{
    memory::{Memory, MemoryIds},
    read_memory_manager,
};

#[derive(CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Default)]
pub struct RunicUtxoMap(HashMap<RuneId, HashSet<RunicUtxo>>);

impl Storable for RunicUtxoMap {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should decode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub type RunicMap = StableBTreeMap<String, RunicUtxoMap, Memory>;

pub fn init_runic_map() -> RunicMap {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::RunicMap.into());
        RunicMap::init(memory)
    })
}

#[derive(CandidType, Deserialize, Default)]
pub struct BitcoinUtxos(HashSet<Utxo>);

impl Storable for BitcoinUtxos {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should decode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub type BitcoinMap = StableBTreeMap<String, BitcoinUtxos, Memory>;

pub fn init_btc_map() -> BitcoinMap {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::BtcMap.into());
        BitcoinMap::init(memory)
    })
}

#[derive(Serialize, Deserialize)]
pub struct UtxoState {
    #[serde(skip, default = "init_runic_map")]
    pub runic_map: RunicMap,
    #[serde(skip, default = "init_btc_map")]
    pub btc_map: BitcoinMap,
}

impl Default for UtxoState {
    fn default() -> Self {
        Self {
            runic_map: init_runic_map(),
            btc_map: init_btc_map(),
        }
    }
}

impl UtxoState {
    pub fn contains_utxo(&self, user: &str, utxo: &Utxo) -> bool {
        let user = String::from(user);
        let btc_utxos = self.btc_map.get(&user).unwrap_or_default().0;
        if btc_utxos.contains(utxo) {
            return true;
        }
        let runic_map = self.runic_map.get(&user).unwrap_or_default().0;
        for (_, v) in runic_map.iter() {
            if v.contains(utxo) {
                return true;
            }
        }
        false
    }

    pub fn record_runic_utxos(&mut self, user: &str, runeid: RuneId, utxos: Vec<RunicUtxo>) {
        let user = String::from(user);
        let mut runic_map = self.runic_map.remove(&user).unwrap_or_default().0;
        let mut current_utxos = runic_map.remove(&runeid).unwrap_or_default();
        for utxo in utxos {
            if current_utxos.contains(&utxo) {
                continue;
            }
            current_utxos.insert(utxo);
        }
        runic_map.insert(runeid, current_utxos);
        self.runic_map.insert(user, RunicUtxoMap(runic_map));
    }

    pub fn record_bitcoin_utxos(&mut self, user: &str, utxos: Vec<Utxo>) {
        let user = String::from(user);
        let mut current_utxos = self.btc_map.remove(&user).unwrap_or_default().0;
        for utxo in utxos {
            if current_utxos.contains(&utxo) {
                continue;
            }
            current_utxos.insert(utxo);
        }
        self.btc_map.insert(user, BitcoinUtxos(current_utxos));
    }

    pub fn get_runic_utxo(&mut self, user: &str, runeid: RuneId) -> Option<RunicUtxo> {
        let user = String::from(user);
        let mut runic_map = self.runic_map.get(&user).unwrap_or_default().0;
        let mut r_utxos = runic_map.remove(&runeid).unwrap_or_default();
        let min_utxo: RunicUtxo = r_utxos.iter().min_by_key(|utxo| utxo.balance)?.clone();
        r_utxos.remove(&min_utxo);
        runic_map.insert(runeid, r_utxos);
        self.runic_map.insert(user, RunicUtxoMap(runic_map));
        Some(min_utxo)
    }

    pub fn get_btc_utxo(&mut self, user: &str) -> Option<Utxo> {
        let user = String::from(user);
        let mut b_utxos = self.btc_map.get(&user).unwrap_or_default().0;
        let min_utxo = b_utxos.iter().min_by_key(|utxo| utxo.value)?.clone();
        b_utxos.remove(&min_utxo);
        self.btc_map.insert(user, BitcoinUtxos(b_utxos));
        Some(min_utxo)
    }

    pub fn get_runestone_balance(&self, user: &str, runeid: &RuneId) -> u128 {
        let user = String::from(user);
        let runic_map = self.runic_map.get(&user).unwrap_or_default().0;
        let current_utxos = match runic_map.get(runeid) {
            None => &HashSet::new(),
            Some(utxos) => utxos,
        };
        current_utxos
            .iter()
            .fold(0, |balance, utxo| balance + utxo.balance)
    }

    pub fn get_bitcoin_balance(&self, user: &str) -> u64 {
        let user = String::from(user);
        let current_utxos = self.btc_map.get(&user).unwrap_or_default().0;
        current_utxos
            .iter()
            .fold(0, |balance, utxo| balance + utxo.value)
    }
}
