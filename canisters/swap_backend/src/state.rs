use std::cell::RefCell;

use config::{init_config, Config, StableConfig};
use ic_stable_structures::{memory_manager::MemoryManager, DefaultMemoryImpl};
use pools::PoolState;
use rune_entry::{init_rune_entries, RuneEntries};
use users::{init_user_mapping, UserMap};
use utxo_manager::UtxoState;

mod config;
mod memory;
pub mod pools;
mod rune_entry;
mod users;
pub mod utxo_manager;

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    pub static CONFIG: RefCell<StableConfig> = RefCell::new(init_config());
    pub static UTXO_STATE: RefCell<UtxoState> = RefCell::default();
    pub static USERS: RefCell<UserMap> = RefCell::new(init_user_mapping());
    pub static RUNE_ENTRIES: RefCell<RuneEntries> = RefCell::new(init_rune_entries());
    pub static POOL_STATE: RefCell<PoolState> = RefCell::default();
}

pub fn read_memory_manager<F, R>(f: F) -> R
where
    F: FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R,
{
    MEMORY_MANAGER.with_borrow(|manager| f(manager))
}

pub fn read_config<F, R>(f: F) -> R
where
    F: FnOnce(&Config) -> R,
{
    CONFIG.with_borrow(|config| f(config.get()))
}

pub fn write_config<F, R>(f: F) -> R
where
    F: FnOnce(&mut StableConfig) -> R,
{
    CONFIG.with_borrow_mut(|config| f(config))
}

pub fn read_utxo_state<F, R>(f: F) -> R
where
    F: FnOnce(&UtxoState) -> R,
{
    UTXO_STATE.with_borrow(|state| f(state))
}

pub fn write_utxo_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut UtxoState) -> R,
{
    UTXO_STATE.with_borrow_mut(|state| f(state))
}

pub fn read_user_mapping<F, R>(f: F) -> R
where
    F: FnOnce(&UserMap) -> R,
{
    USERS.with_borrow(|users| f(users))
}

pub fn write_user_mapping<F, R>(f: F) -> R
where
    F: FnOnce(&mut UserMap) -> R,
{
    USERS.with_borrow_mut(|users| f(users))
}

pub fn read_rune_entries<F, R>(f: F) -> R
where
    F: FnOnce(&RuneEntries) -> R,
{
    RUNE_ENTRIES.with_borrow(|entries| f(entries))
}

pub fn write_rune_entries<F, R>(f: F) -> R
where
    F: FnOnce(&mut RuneEntries) -> R,
{
    RUNE_ENTRIES.with_borrow_mut(|entries| f(entries))
}

pub fn read_pool_state<F, R>(f: F) -> R
where
    F: FnOnce(&PoolState) -> R,
{
    POOL_STATE.with_borrow(|pools| f(pools))
}

pub fn write_pool_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut PoolState) -> R,
{
    POOL_STATE.with_borrow_mut(|pools| f(pools))
}
