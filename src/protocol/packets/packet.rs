#[derive(Debug)]
pub struct Packet{
    size : u64,
    content : Vec<u8>,
    sequence : u64
}

impl Packet{
    pub fn new(content : Vec<u8>, sequence : u64) -> Packet{
        let size = content.len() as u64;
        let content = Vec::from(content);
        Packet{size, content, sequence}
    }

    pub fn get_content(self) -> Vec<u8>{
        self.content
    }

    pub fn get_sequence(&self) -> u64{
        self.sequence
    }

    pub fn get_size(&self) -> u64{
        self.size
    }

    pub fn to_bytes(mut self) -> Vec<u8>{
        let mut vec : Vec<u8> = Vec::new();
        vec.append(&mut self.size.to_ne_bytes().to_vec());
        vec.append(&mut self.sequence.to_ne_bytes().to_vec());
        vec.append(&mut self.content);
        vec
    }

    pub fn from_bytes(bytes : Vec<u8>) -> Packet{
        let size = u64::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let sequence = u64::from_ne_bytes(bytes[8..16].try_into().unwrap());
        let content : Vec<u8> = bytes[16..].to_vec();
        Packet{size, content, sequence}
    }
}