use std::{net::UdpSocket, time::Duration};

use crate::protocol::packets::{ack::Ack, packet::Packet};

const MAX_SIZE: usize = 2560;

pub struct Client{
    socket : UdpSocket,
    sequence : u64,
    next_send : u64,
    ack : u64,
    window : u64,
    in_flight : u64
}

impl Client {
    pub fn new(local_addr : &str, peer_addr : &str) -> Result<Client, std::io::Error>{
        let socket = UdpSocket::bind(local_addr)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        socket.connect(peer_addr)?;
        Ok(Client{socket, sequence : 0, ack : 0, next_send : 0, window : 4*(MAX_SIZE as u64), in_flight : 0})
    }

    fn send_packet(&mut self, content : &Vec<u8>, init_sequence : u64) -> Result<(), std::io::Error>{
        let offset = (self.next_send - init_sequence) as usize;
        let len: usize = content.len();
        let rem_window = (self.window - self.in_flight) as usize;
        let size_sending = usize::min(
            len - offset, 
            usize::min(MAX_SIZE, rem_window)

        );
        let sub = offset..offset+size_sending;
        let buf = content[sub].to_vec();
        let packet = Packet::new(buf, self.next_send);
        self.next_send = self.next_send + (size_sending as u64);
        println!("Sending packets with bytes from {} to {}", packet.get_sequence(), packet.get_sequence()+packet.get_size());
        self.in_flight += size_sending as u64;
        self.socket.send(&packet.to_bytes())?;
        Ok(())
    }

    fn wait_ack(&mut self) -> Result<(), std::io::Error>{
        let mut buf : [u8; 64] = [0; 64];
        let amt = self.socket.recv(&mut buf);
        match amt{
            Ok(amt) => {
                let ack = Ack::from_bytes(buf[..amt].to_vec());
                println!("{:?}", ack);
                if ack.get_acked() > self.sequence{
                    self.in_flight -= ack.get_acked() - self.sequence;
                    self.sequence = ack.get_acked();
                }
            },
            Err(_) => {
                // rto reached, must retransmit all
                self.in_flight = 0;
                self.next_send = self.sequence;
            }
        }
        Ok(())
    }

    /**
     * Go-back-n implementation of sending packets
     */
    pub fn send(&mut self, content : Vec<u8>) -> Result<(), std::io::Error>{
        let init_sequence: u64 = self.sequence;
        let len = content.len() as u64;
        while self.sequence < init_sequence + len{
            while self.in_flight < self.window 
            && self.next_send < init_sequence + len{
                self.send_packet(&content, init_sequence)?;
            }
            self.wait_ack()?;
        }
        Ok(())
    }
}