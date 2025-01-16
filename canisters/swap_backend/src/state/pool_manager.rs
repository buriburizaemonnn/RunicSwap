use std::collections::HashMap;

use candid::{CandidType, Decode, Encode, Principal};
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use num_integer::sqrt;
use serde::{Deserialize, Serialize};

use crate::{
    addresses::Addresses,
    memory::{Memory, MemoryIds},
    token_type::TokenType,
};

use super::{read_config, read_memory_manager};

const MINIMUM_LIQUIDITY: u64 = 1_000;

#[derive(CandidType, Deserialize)]
pub struct PairInfo {
    pub created_at: u64,
    pub allocated_raw_subaccount: [u8; 32],
    pub pool_id: u128,
    pub token0: TokenType,
    pub token1: TokenType,
    pub reserve0: u64,
    pub reserve1: u64,
    pub total_supply: u64,
    pub k_last: u64,
    pub last_updated: u64,
    pub holders: HashMap<Principal, u64>,
}

impl Storable for PairInfo {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub struct BurnResult {
    pub raw_subaccount: [u8; 32],
    pub token0: TokenType,
    pub token1: TokenType,
    pub amount0: u64,
    pub amount1: u64,
}

pub struct SwapResult {
    pub raw_subaccount: [u8; 32],
    pub token: TokenType,
    pub amount: u64,
}

impl PairInfo {
    /* pub fn into_query(&self) -> PoolInfoQuery {
        PoolInfoQuery {
            pool_id: self.pool_id,
            deposit_addresses: self.deposit_addresses(),
            token0: self.token0.clone(),
            token1: self.token1.clone(),
            reserve0: self.reserve0,
            reserve1: self.reserve1,
        }
    } */

    // returns amount0in, amount0out, amount1in, amount1out
    pub fn sort_tokens(
        &self,
        token_in: &TokenType,
        amount_in: u64,
        amount_out: u64,
    ) -> (u64, u64, u64, u64) {
        if &self.token0 == token_in {
            (amount_in, 0, 0, amount_out)
        } else {
            (0, amount_out, amount_in, 0)
        }
    }

    pub fn deposit_addresses(&self) -> Addresses {
        Addresses::from(self.allocated_raw_subaccount)
    }

    pub fn is_reserve_empty(&self) -> bool {
        self.reserve0 == 0 && self.reserve1 == 0
    }

    fn _update(&mut self, amount0_in: u64, amount1_in: u64, amount0_out: u64, amount1_out: u64) {
        let current_time = ic_cdk::api::time();
        // NOTE: skipped cumulative price for now.
        // let time_elapsed = current_time - self.last_updated;
        // if time_elapsed > 0 && !self.is_reserve_empty() {}
        self.reserve0 += amount0_in;
        self.reserve0 -= amount0_out;
        self.reserve1 += amount1_in;
        self.reserve1 -= amount1_out;
        self.last_updated = current_time;
    }

    pub fn quote_a(&self, amount0: u64) -> Result<u64, String> {
        if amount0 == 0 {
            return Err(String::from("Insuficient Input Amount"));
        }
        Ok((amount0 * self.reserve1) / self.reserve0)
    }

    pub fn quote_b(&self, amount1: u64) -> Result<u64, String> {
        if amount1 == 0 {
            return Err(String::from("Insuficient Input Amount"));
        }
        Ok((amount1 * self.reserve0) / self.reserve1)
    }

    pub fn pre_mint_check(&self, amount0: u64, amount1: u64) -> Result<(), String> {
        let liquidity = if self.total_supply == 0 {
            sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY
        } else {
            let val0 = (amount0 * self.total_supply) / self.reserve0;
            let val1 = (amount1 * self.total_supply) / self.reserve1;
            val0.min(val1)
        };
        if liquidity == 0 {
            return Err(String::from("INSUFFICIENT LIQUIDITY MINTED"));
        }
        Ok(())
    }

    pub fn _mint(&mut self, to: &Principal, amount: u64) {
        let current_liqquidity = self.holders.get(to).unwrap_or(&0);
        self.holders.insert(*to, current_liqquidity + amount);
        self.total_supply += amount;
    }

    fn _mint_fee(&mut self) {
        let commission_receiver = read_config(|config| config.commission_receiver_principal());
        if self.k_last != 0 {
            let rootk = sqrt(self.reserve0 * self.reserve1);
            let rootk_last = sqrt(self.k_last);
            if rootk > rootk_last {
                let numerator = self.total_supply * (rootk - rootk_last);
                let denominator = (rootk * 5) + rootk_last;

                let liquidity = numerator / denominator;
                if liquidity > 0 {
                    self._mint(&commission_receiver, liquidity);
                }
            }
        }
    }

    pub fn mint(&mut self, receiver: Principal, amount0: u64, amount1: u64) -> Result<u64, String> {
        let liquidity: u64;
        if self.total_supply == 0 {
            liquidity = sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY;
            self.holders.insert(ic_cdk::id(), MINIMUM_LIQUIDITY);
            self.total_supply = MINIMUM_LIQUIDITY;
        } else {
            let val0 = (amount0 * self.total_supply) / self.reserve0;
            let val1 = (amount1 * self.total_supply) / self.reserve1;
            liquidity = val0.min(val1);
        }
        if liquidity == 0 {
            return Err(String::from("INSUFFICIENT LIQUIDITY MINTED"));
        }
        self.reserve0 += amount0;
        self.reserve1 += amount1;

        self._mint(&receiver, liquidity);
        self.k_last = self.reserve0 * self.reserve1;
        Ok(liquidity)
    }

    fn _burn(&mut self, from: &Principal, liquidity: u64) {
        let current_liquidity = self.holders.get(from).expect("should exist");
        self.total_supply -= liquidity;
        self.holders.insert(*from, current_liquidity - liquidity);
    }

    pub fn burn(
        &mut self,
        caller: &Principal,
        liquidity: u64,
        amount0_min: u64,
        amount1_min: u64,
    ) -> Result<BurnResult, String> {
        let current_liquidity = self.holders.get(caller).copied().unwrap_or(0);
        if liquidity > current_liquidity {
            return Err(String::from("Not enough Liquidity"));
        }
        let amount0 = (liquidity * self.reserve0) / self.total_supply;
        let amount1 = (liquidity * self.reserve1) / self.total_supply;
        if amount0 == 0 && amount1 == 0 {
            return Err(String::from("Insuficient Liquidity Burned"));
        }
        if amount0 < amount0_min {
            return Err(String::from("Insuficient A_AMOUNT"));
        }
        if amount1 < amount1_min {
            return Err(String::from("Insuficient B_AMOUNT"));
        }
        self._mint_fee();
        self._burn(caller, liquidity);
        self._update(0, 0, amount0, amount1);
        self.k_last = self.reserve0 * self.reserve1;
        Ok(BurnResult {
            raw_subaccount: self.allocated_raw_subaccount,
            token0: self.token0.clone(),
            token1: self.token1.clone(),
            amount0,
            amount1,
        })
    }

    pub fn get_amount_out(&self, amount_in: u64, token_in: &TokenType) -> u64 {
        let amount_in_with_fee = amount_in * 997;
        let (reserve_in, reserve_out) = if token_in == &self.token0 {
            (self.reserve0, self.reserve1)
        } else {
            (self.reserve1, self.reserve0)
        };
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = (reserve_in * 1000) + amount_in_with_fee;
        numerator / denominator
    }

    pub fn swap(
        &mut self,
        amount0_in: u64,
        amount1_in: u64,
        amount0_out: u64,
        amount1_out: u64,
    ) -> Result<SwapResult, String> {
        let token0_balance = self.reserve0 + amount0_in;
        let token1_balance = self.reserve1 + amount1_in;
        if amount0_out == 0 && amount1_out == 0 {
            return Err(String::from("Insuficient Output Amount"));
        }
        if amount0_out >= self.reserve0 || amount1_out >= self.reserve1 {
            return Err(String::from("Insuficient Liquidity"));
        }
        if amount0_in == 0 && amount1_in == 0 {
            return Err(String::from("Insuficient Input Amount"));
        }
        let balance0adjusted = token0_balance * 1000 - (amount0_in * 3);
        let balance1adjusted = token1_balance * 1000 - (amount1_in * 3);
        if balance0adjusted * balance1adjusted < (self.reserve0 * self.reserve1) * 1000 * 1000 {
            return Err(String::from("Invalid K"));
        }
        self._update(amount0_in, amount1_in, amount0_out, amount1_out);
        let (token, amount) = if amount0_out > 0 {
            (self.token0.clone(), amount0_out)
        } else {
            (self.token1.clone(), amount1_out)
        };
        Ok(SwapResult {
            raw_subaccount: self.allocated_raw_subaccount,
            token,
            amount,
        })
    }
}

pub type PoolMap = StableBTreeMap<u128, PairInfo, Memory>;

pub fn init_pool_mapping() -> PoolMap {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::Pool.into());
        PoolMap::init(memory)
    })
}

#[derive(CandidType, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssociatedPoolKey(TokenType, TokenType);

impl Storable for AssociatedPoolKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).expect("should encode"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("should decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub type AssociatedPoolIdSet = StableBTreeMap<AssociatedPoolKey, u128, Memory>;

pub fn init_associated_pool_id_set() -> AssociatedPoolIdSet {
    read_memory_manager(|manager| {
        let memory = manager.get(MemoryIds::AssociatedPoolIdSet.into());
        AssociatedPoolIdSet::new(memory)
    })
}

#[derive(Serialize, Deserialize)]
pub struct PoolState {
    #[serde(skip, default = "init_pool_mapping")]
    pub pool_map: PoolMap,
    #[serde(skip, default = "init_associated_pool_id_set")]
    associated_pool_id_set: AssociatedPoolIdSet,
}

impl Default for PoolState {
    fn default() -> Self {
        Self {
            pool_map: init_pool_mapping(),
            associated_pool_id_set: init_associated_pool_id_set(),
        }
    }
}

impl PoolState {
    pub fn get_pool_id(&self, token0: TokenType, token1: TokenType) -> Option<u128> {
        let key0 = AssociatedPoolKey(token0.clone(), token1.clone());
        if let Some(id) = self.associated_pool_id_set.get(&key0) {
            return Some(id);
        }
        let key1 = AssociatedPoolKey(token1, token0);
        if let Some(id) = self.associated_pool_id_set.get(&key1) {
            return Some(id);
        }
        None
    }

    pub fn record_pair(&mut self, pair_info: PairInfo) {
        let key = AssociatedPoolKey(pair_info.token0.clone(), pair_info.token1.clone());
        self.associated_pool_id_set.insert(key, pair_info.pool_id);
        self.pool_map.insert(pair_info.pool_id, pair_info);
    }
}
