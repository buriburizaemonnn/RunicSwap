use std::cell::RefCell;

use config::{init_stable_config, Config, StableConfig};
use ic_stable_structures::{memory_manager::MemoryManager, DefaultMemoryImpl};
use pool_manager::PoolState;
use user_manager::{init_user_mapping, UserMapping};
use utxo_manager::UtxoManager;

pub mod config;
pub mod pool_manager;
pub mod user_manager;
pub mod utxo_manager;

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    pub static CONFIG: RefCell<StableConfig> = RefCell::new(init_stable_config());
    pub static POOL_MANAGER: RefCell<PoolState> = RefCell::default();
    pub static USER_MANAGER: RefCell<UserMapping> = RefCell::new(init_user_mapping());
    pub static UTXO_MANAGER: RefCell<UtxoManager> = RefCell::default();
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

pub fn read_pool_manager<F, R>(f: F) -> R
where
    F: FnOnce(&PoolState) -> R,
{
    POOL_MANAGER.with_borrow(|manager| f(manager))
}

pub fn write_pool_manager<F, R>(f: F) -> R
where
    F: FnOnce(&mut PoolState) -> R,
{
    POOL_MANAGER.with_borrow_mut(|manager| f(manager))
}

pub fn read_user_manager<F, R>(f: F) -> R
where
    F: FnOnce(&UserMapping) -> R,
{
    USER_MANAGER.with_borrow(|manager| f(manager))
}

pub fn write_user_manager<F, R>(f: F) -> R
where
    F: FnOnce(&mut UserMapping) -> R,
{
    USER_MANAGER.with_borrow_mut(|manager| f(manager))
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
