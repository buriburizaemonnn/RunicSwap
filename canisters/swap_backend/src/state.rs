use std::cell::RefCell;

use config::{init_stable_config, Config, StableConfig};
use ic_stable_structures::{memory_manager::MemoryManager, DefaultMemoryImpl};
use pool_manager::PoolState;
use utxo_manager::UtxoManager;

mod config;
pub mod pool_manager;
mod user_manager;
mod utxo_manager;

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    pub static CONFIG: RefCell<StableConfig> = RefCell::new(init_stable_config());
    pub static UTXO_MANAGER: RefCell<UtxoManager> = RefCell::default();
    pub static POOL_MANAGER: RefCell<PoolState> = RefCell::default();
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

pub fn read_utxo_manager<F, R>(f: F) -> R
where
    F: FnOnce(&UtxoManager) -> R,
{
    UTXO_MANAGER.with_borrow(|manager| f(manager))
}

pub fn write_utxo_manager<F, R>(f: F) -> R
where
    F: FnOnce(&mut UtxoManager) -> R,
{
    UTXO_MANAGER.with_borrow_mut(|manager| f(manager))
}

pub fn read_pool_manager<F, R>(f: F) -> R
where
    F: FnOnce(&PoolState) -> R,
{
    POOL_MANAGER.with_borrow(|pools| f(pools))
}

pub fn write_pool_manager<F, R>(f: F) -> R
where
    F: FnOnce(&mut PoolState) -> R,
{
    POOL_MANAGER.with_borrow_mut(|pools| f(pools))
}
