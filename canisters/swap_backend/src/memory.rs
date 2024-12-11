use ic_stable_structures::{
    memory_manager::{MemoryId, VirtualMemory},
    DefaultMemoryImpl,
};

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

pub enum MemoryIds {
    Config,
    Pools,
    AssociatedPoolSet,
    Bitcoin,
    Runic,
}

impl From<MemoryIds> for MemoryId {
    fn from(value: MemoryIds) -> Self {
        let id = match value {
            MemoryIds::Config => 1,
            MemoryIds::Pools => 2,
            MemoryIds::AssociatedPoolSet => 3,
            MemoryIds::Bitcoin => 4,
            MemoryIds::Runic => 5,
        };
        MemoryId::new(id)
    }
}
