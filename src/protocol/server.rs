use std::net::UdpSocket;

use crate::protocol::packets::packet::Packet;

use super::packets::packet;

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

    fn wait_connect(&mut self) -> Result<bool, std::io::Error>{
        let mut buf = [0; 2560];
        let (amt, src) = self.socket.recv_from(&mut buf)?;

        let received = Packet::from_bytes(buf[..amt].to_vec());
        if !received.is_syn(){
            return Ok(false);
        }

        let seq = received.get_sequence();
        self.ack = seq+1;
        let ack = Packet::new_synack(self.sequence, self.ack);
        self.socket.send_to(&ack.to_bytes(), src)?;

        let (amt, _src) = self.socket.recv_from(&mut buf)?;

        let received = Packet::from_bytes(buf[..amt].to_vec());
        if !received.is_ack(){
            return Ok(false);
        }
        if received.get_acked() != self.sequence+1{
            return Ok(false);
        }
        self.sequence += 1;

        Ok(true)
    }

    pub fn receive(&mut self) -> Result<(), std::io::Error>{
        while !self.wait_connect()?{

        }
        println!("Done handshake");
        loop {
            let mut buf = [0; 2560];
            let (amt, src) = self.socket.recv_from(&mut buf)?;
    
            let received = Packet::from_bytes(buf[..amt].to_vec());

            if received.is_reset(){
                if received.get_sequence() < self.ack{
                    continue;
                }
                break;
            }
    
            println!("Received with state seq={}", self.ack);
            println!("Received packet with seq={}, size={}", received.get_sequence(), received.get_size());
    
            if received.get_sequence() == self.ack{
                let ack = Packet::new_ack(self.sequence, self.ack + received.get_size());
                self.socket.send_to(&ack.to_bytes(), src)?;
                self.ack += received.get_size();
            }
    
            println!("{}", amt);
        }

        Ok(())
    }
}