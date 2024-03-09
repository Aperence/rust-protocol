
pub const MAX_SIZE: usize = 2560;

pub struct Connection{
    pub sequence : u64,
    pub ack : u64,
    pub window : u64,
    pub in_flight : u64
}

impl Connection{
    pub fn new(sequence : u64, ack : u64) -> Connection{
        Connection{sequence, ack, window : 4*(MAX_SIZE as u64), in_flight : 0}
    }
}