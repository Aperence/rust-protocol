#[derive(Debug)]
pub struct Ack{
    sequence : u64,
    acked : u64,
}

impl Ack{
    pub fn new(sequence : u64, acked : u64) -> Ack{
        Ack{sequence, acked}
    }

    pub fn to_bytes(self) -> Vec<u8>{
        let mut vec : Vec<u8> = Vec::new();
        let mut s = Vec::from(self.sequence.to_ne_bytes());
        let mut acked = Vec::from(self.acked.to_ne_bytes());
        vec.append(&mut s);
        vec.append(&mut acked);
        vec
    }

    pub fn get_sequence(&self) -> u64{
        self.sequence
    }

    pub fn get_acked(&self) -> u64{
        self.acked
    }

    pub fn from_bytes(bytes : Vec<u8>) -> Ack{
        let sequence = u64::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let acked = u64::from_ne_bytes(bytes[8..16].try_into().unwrap());
        Ack{sequence, acked}
    }
}