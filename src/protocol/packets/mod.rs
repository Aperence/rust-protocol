#[derive(Debug)]
pub struct Packet{
    size : u64,
    content : Vec<u8>,
    sequence : u64,
    acked : u64,
    syn : bool,
    ack : bool,
    reset : bool,
    fin : bool
}

impl Packet{
    pub fn new_data(content : Vec<u8>, sequence : u64) -> Packet{
        let size = content.len() as u64;
        let content = Vec::from(content);
        Packet{size, content, sequence, acked:0, syn:false, ack:false, reset:false, fin : false}
    }

    pub fn new_ack(sequence : u64, acked : u64) -> Packet{
        Packet{size:0, content:Vec::new(), sequence, acked, syn:false, ack:true, reset:false, fin : false}
    }

    pub fn new_synack(sequence : u64, acked : u64) -> Packet{
        Packet{size:0, content:Vec::new(), sequence, acked, syn:true, ack:true, reset:false, fin : false}
    }

    pub fn new_syn(sequence : u64) -> Packet{
        Packet{size:0, content:Vec::new(), sequence, acked:0, syn:true, ack:false, reset:false, fin : false}
    }

    pub fn new_reset(sequence : u64) -> Packet{
        Packet{size:0, content:Vec::new(), sequence, acked:0, syn:false, ack:false, reset:true, fin : false}
    }

    pub fn new_fin(sequence : u64) -> Packet{
        Packet{size:0, content:Vec::new(), sequence, acked:0, syn:false, ack:false, reset:false, fin : true}
    }

    pub fn get_content(self) -> Vec<u8>{
        self.content
    }

    pub fn get_sequence(&self) -> u64{
        self.sequence
    }

    pub fn get_acked(&self) -> u64{
        self.acked
    }

    pub fn get_size(&self) -> u64{
        self.size
    }

    pub fn is_syn(&self) -> bool{
        self.syn
    }

    pub fn is_ack(&self) -> bool{
        self.ack
    }

    pub fn is_reset(&self) -> bool{
        self.reset
    }

    pub fn is_fin(&self) -> bool{
        self.fin
    }

    pub fn to_bytes(mut self) -> Vec<u8>{
        let mut vec : Vec<u8> = Vec::new();
        let mut flags : u8 = 0;
        if self.fin{
            flags = flags | 0x8;
        }
        if self.reset{
            flags = flags | 0x4;
        }
        if self.syn{
            flags = flags | 0x2;
        }
        if self.ack{
            flags = flags | 0x1;
        }
        vec.push(flags);
        vec.append(&mut self.size.to_ne_bytes().to_vec());
        vec.append(&mut self.sequence.to_ne_bytes().to_vec());
        vec.append(&mut self.acked.to_ne_bytes().to_vec());
        vec.append(&mut self.content);
        vec
    }

    pub fn from_bytes(bytes : Vec<u8>) -> Packet{
        let flags = bytes[0];
        let fin = (flags & 0x8) != 0;
        let reset = (flags & 0x4) != 0;
        let syn = (flags & 0x2) != 0;
        let ack = (flags & 0x1) != 0;
        let size = u64::from_ne_bytes(bytes[1..9].try_into().unwrap());
        let sequence = u64::from_ne_bytes(bytes[9..17].try_into().unwrap());
        let acked = u64::from_ne_bytes(bytes[17..25].try_into().unwrap());
        let content : Vec<u8> = bytes[25..].to_vec();
        Packet{size, content, sequence, acked, syn, ack, reset, fin}
    }
}