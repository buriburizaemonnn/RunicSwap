use candid::{CandidType, Decode, Encode, Principal};
use ic_cdk::api::management_canister::{
    bitcoin::BitcoinNetwork,
    ecdsa::{EcdsaCurve, EcdsaKeyId},
};
use ic_ledger_types::AccountIdentifier;
use ic_stable_structures::{storable::Bound, StableCell, Storable};
use icrc_ledger_types::icrc1::account::Account;
use serde::Deserialize;

use crate::EcdsaPublicKey;

use super::{
    memory::{Memory, MemoryIds},
    read_memory_manager,
};

#[derive(CandidType, Deserialize, Default, Clone)]
pub struct Config {
    pub auth: Option<Principal>,
    pub bitcoin_network: Option<BitcoinNetwork>,
    pub ecdsa_key: Option<EcdsaPublicKey>,
    pub keyname: Option<String>,
    pub ord_canister: Option<Principal>,
    pub ckbtc: Option<Principal>,
    pub commission_receiver_principal: Option<Principal>,
    pub commission_receiver_account_identifier: Option<AccountIdentifier>,
    pub commission_receiver_account: Option<Account>,
    pub commission_receiver_bitcoin: Option<String>,
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
    pub fn auth(&self) -> Principal {
        match self.auth {
            None => ic_cdk::trap("canister's config uninitialized"),
            Some(auth) => auth,
        }
    }

    pub fn bitcoin_network(&self) -> BitcoinNetwork {
        match self.bitcoin_network {
            None => ic_cdk::trap("canister's config uninitialized"),
            Some(network) => network,
        }
    }

    pub fn ecdsa_key(&self) -> EcdsaPublicKey {
        if let Some(ref publickey) = self.ecdsa_key {
            publickey.clone()
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

    pub fn ecdsakeyid(&self) -> EcdsaKeyId {
        match self.keyname {
            None => ic_cdk::trap("canister's config uninitialized"),
            Some(ref keyname) => EcdsaKeyId {
                curve: EcdsaCurve::Secp256k1,
                name: keyname.clone(),
            },
        }
    }

    pub fn ord_canister(&self) -> Principal {
        match self.ord_canister {
            None => ic_cdk::trap("canister's config uninitialized"),
            Some(canister) => canister,
        }
    }

    pub fn ckbtc(&self) -> Principal {
        self.ckbtc.expect("canister's config uninitialized")
    }

    pub fn commission_receiver_principal(&self) -> Principal {
        if let Some(principal) = self.commission_receiver_principal {
            principal
        } else {
            ic_cdk::trap("canister's config uninitialized")
        }
    }

    pub fn commission_receiver_account_identifier(&self) -> AccountIdentifier {
        if let Some(ref account_identifier) = self.commission_receiver_account_identifier {
            *account_identifier
        } else {
            AccountIdentifier::new(&ic_cdk::id(), &ic_ledger_types::DEFAULT_SUBACCOUNT)
        }
    }

    pub fn commission_receiver_account(&self) -> Account {
        if let Some(ref account) = self.commission_receiver_account {
            account.clone()
        } else {
            ic_cdk::trap("canister's config uninitialized")
        }
    }

    pub fn commission_receiver_bitcoin(&self) -> String {
        if let Some(ref addr) = self.commission_receiver_bitcoin {
            addr.clone()
        } else {
            ic_cdk::trap("canister's config uninitialized")
        }
    }
}

pub type StableConfig = StableCell<Config, Memory>;

pub fn init_config() -> StableConfig {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::Config.into());
        StableConfig::init(memory, Config::default()).expect("failed to init config")
    })
}
