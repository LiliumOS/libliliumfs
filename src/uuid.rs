use bytemuck::{Zeroable, Pod};


#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Zeroable, Pod)]
#[repr(C)]
pub struct Uuid{
    pub lo: u64,
    pub hi: u64,
}


