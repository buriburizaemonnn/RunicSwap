use ic_stable_structures::{
    memory_manager::{MemoryId, VirtualMemory},
    DefaultMemoryImpl,
};

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

pub enum MemoryIds {
    Config,
    RunicMap,
    BtcMap,
    UserMapping,
    PoolMapping,
    AssociatedPoolMapping,
    RuneEntries,
}

impl From<MemoryIds> for MemoryId {
    fn from(value: MemoryIds) -> Self {
        match value {
            MemoryIds::Config => MemoryId::new(0),
            MemoryIds::RunicMap => MemoryId::new(1),
            MemoryIds::BtcMap => MemoryId::new(2),
            MemoryIds::UserMapping => MemoryId::new(3),
            MemoryIds::PoolMapping => MemoryId::new(4),
            MemoryIds::AssociatedPoolMapping => MemoryId::new(5),
            MemoryIds::RuneEntries => MemoryId::new(6),
        }
    }
}
