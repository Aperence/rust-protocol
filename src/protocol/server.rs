use std::{collections::HashMap, net::UdpSocket, sync::mpsc::Sender, thread::Thread};

use crate::protocol::packets::packet::Packet;
use crate::protocol::connection::connection::Connection;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::thread;

fn hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub struct Server{
    socket : UdpSocket,
    connections : HashMap<String, Connection>

}

impl Server {
    pub fn new(addr : &str) -> Result<Server, std::io::Error>{
        let socket = UdpSocket::bind(addr)?;
        Ok(Server{socket, connections : HashMap::new()})
    }

    fn received_packet_connection(&mut self, packet : Packet, addr : String, tx : Vec<Sender<(String, Vec<u8>)>>) -> Result<(), std::io::Error>{
        let connection = self.connections.get_mut(&addr).unwrap();
        if packet.get_sequence() == connection.ack{
            if packet.is_reset(){
                self.connections.remove(&addr);
                return Ok(());
            }
            let ack = Packet::new_ack(connection.sequence, connection.ack + packet.get_size());
            self.socket.send_to(&ack.to_bytes(), addr.clone())?;
            connection.ack += packet.get_size();
            let idx = (hash(&addr) as usize) % tx.len();
            tx.get(idx).unwrap().send((addr, packet.get_content())).expect("Failed to send to receiver");
        }
        Ok(())
    }

    fn receive_packet(&mut self, tx : Vec<Sender<(String, Vec<u8>)>>) -> Result<(), std::io::Error>{
        let mut buf = [0; 2560];
        let (amt, src) = self.socket.recv_from(&mut buf)?;
        
        let received = Packet::from_bytes(buf[..amt].to_vec());
        if received.is_syn(){
            // begin handshake by sending syn-ack
            let seq = hash(&src); // use an hash to avoid syn flooding
            let synack = Packet::new_synack(seq, received.get_sequence()+1);
            self.socket.send_to(&synack.to_bytes(), src)?;
            return Ok(());
        }

        let addr = src.to_string();

        if !self.connections.contains_key(&addr){
            // finalize the handshake
            if !received.is_ack(){
                return Ok(());
            }
            let hashed = hash(&src);
            if received.get_acked() != hashed + 1{
                return Ok(());
            }
            println!("Correct ack, creating connection");
            self.connections.insert(addr, Connection::new(hashed+1, received.get_sequence()));
        }else{
            // data packet/reset, serve to correct connection
            self.received_packet_connection(received, addr, tx)?;
        }
        Ok(())
    }

    pub fn receive(mut self, tx : Vec<Sender<(String, Vec<u8>)>>) -> Result<(), std::io::Error>{
        
        thread::spawn(move || {
            // some work here
            loop {
                let t = tx.clone();
                self.receive_packet(t).unwrap();
            }
        });
 
        Ok(())
    }
}