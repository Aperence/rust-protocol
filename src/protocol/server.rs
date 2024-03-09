use std::net::UdpSocket;

use crate::protocol::packets::{ack::Ack, packet::Packet};

pub struct Server{
    socket : UdpSocket,
    sequence : u64,
    ack : u64,
}

impl Server {
    pub fn new(addr : &str) -> Result<Server, std::io::Error>{
        let socket = UdpSocket::bind(addr)?;
        Ok(Server{socket, sequence : 0, ack : 0})
    }

    pub fn receive(&mut self) -> Result<String, std::io::Error>{
        let mut buf = [0; 2560];
        let (amt, src) = self.socket.recv_from(&mut buf)?;

        let received = Packet::from_bytes(buf[..amt].to_vec());

        println!("Received with state seq={}", self.ack);
        println!("Received packet with seq={}, size={}", received.get_sequence(), received.get_size());

        if received.get_sequence() == self.ack{
            let ack = Ack::new(self.sequence, self.ack + received.get_size());
            self.socket.send_to(&ack.to_bytes(), src)?;
            self.ack += received.get_size();
        }

        println!("{}", amt);
        
        Ok(String::from_utf8(received.get_content()).unwrap())
    }
}