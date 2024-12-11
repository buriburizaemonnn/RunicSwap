use crate::{
    memory::{Memory, MemoryIds},
    EcdsaPublicKey,
};
use candid::{CandidType, Decode, Encode, Principal};
use ic_cdk::api::management_canister::{
    bitcoin::BitcoinNetwork,
    ecdsa::{EcdsaCurve, EcdsaKeyId},
};
use ic_stable_structures::{storable::Bound, StableCell, Storable};
use serde::Deserialize;

use super::read_memory_manager;

#[derive(CandidType, Deserialize, Default, Clone)]
pub struct Config {
    pub auth: Option<Principal>,
    pub bitcoin_network: Option<BitcoinNetwork>,
    pub keyname: Option<String>,
    pub ecdsa_public_key: Option<EcdsaPublicKey>,
    pub commission_receiver_principal: Option<Principal>,
}

impl Storable for Config {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl Config {
    pub fn bitcoin_network(&self) -> BitcoinNetwork {
        if let Some(network) = self.bitcoin_network {
            network
        } else {
            ic_cdk::trap("canister's config uninitialized")
        }
    }

    pub fn keyname(&self) -> String {
        if let Some(ref keyname) = self.keyname {
            keyname.clone()
        } else {
            ic_cdk::trap("canister's config uninitialized")
        }
    }

    pub fn ecdsa_public_key(&self) -> EcdsaPublicKey {
        if let Some(ref ecdsa_key) = self.ecdsa_public_key {
            ecdsa_key.clone()
        } else {
            ic_cdk::trap("canister's config uninitialized")
        }
    }

    pub fn ecdsakeyid(&self) -> EcdsaKeyId {
        let name = self.keyname();
        EcdsaKeyId {
            name,
            curve: EcdsaCurve::Secp256k1,
        }
    }

    pub fn commission_receiver_principal(&self) -> Principal {
        self.commission_receiver_principal.unwrap_or(ic_cdk::id())
    }
}

pub type StableConfig = StableCell<Config, Memory>;

pub fn init_stable_config() -> StableConfig {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::Config.into());
        StableConfig::new(memory, Config::default()).expect("failed to initialize config")
    })
}
