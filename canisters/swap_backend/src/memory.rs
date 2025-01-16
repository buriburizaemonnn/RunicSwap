use ic_stable_structures::{
    memory_manager::{MemoryId, VirtualMemory},
    DefaultMemoryImpl,
};

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

pub enum MemoryIds {
    Config,
    Runic,
    Bitcoin,
    Pool,
    AssociatedPoolIdSet,
    User,
}

impl From<MemoryIds> for MemoryId {
    fn from(value: MemoryIds) -> Self {
        let id = match value {
            MemoryIds::Config => 1,
            MemoryIds::Runic => 2,
            MemoryIds::Bitcoin => 3,
            MemoryIds::Pool => 4,
            MemoryIds::AssociatedPoolIdSet => 5,
            MemoryIds::User => 6,
        };
        MemoryId::new(id)
    }
}
