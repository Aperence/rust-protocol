use std::{collections::HashMap, net::UdpSocket, sync::mpsc::Sender, thread, time::Duration};
use std::hash::{DefaultHasher, Hash, Hasher};
use rand::{self, random};

use std::io::{Error, ErrorKind};

use crate::protocol::packets::packet::Packet;
use crate::protocol::connection::connection::{Connection, MAX_SIZE};

fn hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub struct Protocol{
    socket : UdpSocket,
    connections : HashMap<String, Connection>
}


impl Protocol{

    pub fn new(addr : &str) -> Result<Protocol, std::io::Error>{
        let socket = UdpSocket::bind(addr)?;
        Ok(Protocol{socket, connections : HashMap::new()})
    }

    fn send_packet(&mut self, content : &Vec<u8>, init_sequence : u64, addr : String) -> Result<(), std::io::Error>{
        let connection = self.connections.get_mut(&addr).unwrap();
        let offset = (connection.sequence + connection.in_flight - init_sequence) as usize;
        let len: usize = content.len();
        let rem_window = (connection.window - connection.in_flight) as usize;
        let size_sending = usize::min(
            len - offset, 
            usize::min(MAX_SIZE, rem_window)
        );
        let sub = offset..offset+size_sending;
        let buf = content[sub].to_vec();
        let packet = Packet::new_data(buf, connection.sequence + connection.in_flight);
        println!("Sending packets with bytes from {} to {}", packet.get_sequence(), packet.get_sequence()+packet.get_size());
        connection.in_flight += size_sending as u64;
        self.socket.send_to(&packet.to_bytes(), addr)?;
        Ok(())
    }

    fn wait_ack(&mut self, addr : String) -> Result<(), std::io::Error>{
        let connection = self.connections.get_mut(&addr).unwrap();
        let mut buf : [u8; 64] = [0; 64];
        self.socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        let amt = self.socket.recv(&mut buf);
        self.socket.set_read_timeout(None)?;
        match amt{
            Ok(amt) => {
                let ack = Packet::from_bytes(buf[..amt].to_vec());
                if ack.get_acked() > connection.sequence{
                    connection.in_flight -= ack.get_acked() - connection.sequence;
                    connection.sequence = ack.get_acked();
                }
            },
            Err(_) => {
                // rto reached, must retransmit all
                connection.in_flight = 0;
            }
        }
        Ok(())
    }

    pub fn connect(&mut self, addr : String) -> Result<bool, std::io::Error>{
        loop {
            let seq : u16 = random(); // random between 0 and 64000
            let seq = seq as u64;
            let syn = Packet::new_syn(seq);
            self.socket.send_to(&syn.to_bytes(), addr.clone())?;
            println!("Sent syn");
    
    
            let mut buf = [0; 2560];
            let amt = self.socket.recv(&mut buf)?;
            println!("Received synack");
            let synack = Packet::from_bytes(buf[..amt].to_vec());
            if !synack.is_syn() || !synack.is_ack() || seq + 1 != synack.get_acked(){
                continue;
            }
            let connection = Connection::new(seq+1, synack.get_sequence()+1);
     
            let ack = Packet::new_ack(connection.sequence, synack.get_sequence()+1);
            self.socket.send_to(&ack.to_bytes(), addr.clone())?;
    
            self.connections.insert(addr, connection);
    
            break;
        }
        Ok(true)
    }

    /**
     * Go-back-n implementation for sending packets
     */
    pub fn send(&mut self, content : Vec<u8>, addr : String) -> Result<(), std::io::Error>{
        if !self.connections.contains_key(&addr){
            return Err(Error::new(ErrorKind::NotConnected, "Expected to first connect before sending content"));
        }
        
        println!("Done handshake");
        
        let len = content.len() as u64;
        let init_sequence ;
        let mut curr_seq ;
        let mut inflight ;
        let mut remaining ;
        let mut window ;

        {
            let connection = self.connections.get(&addr).unwrap();
            curr_seq = connection.sequence;
            init_sequence = connection.sequence;
        }
        while curr_seq < init_sequence + len{
            {
                let connection = self.connections.get(&addr).unwrap();
                remaining = connection.sequence + connection.in_flight - init_sequence;
                inflight = connection.in_flight;
                window = connection.window;
                curr_seq = connection.sequence;
            }
            while inflight < window && remaining < len{
                self.send_packet(&content, init_sequence, addr.clone())?;
                {
                    let connection = self.connections.get(&addr).unwrap();
                    remaining = connection.sequence + connection.in_flight - init_sequence;
                    inflight = connection.in_flight;
                    window = connection.window;
                    curr_seq = connection.sequence;
                }
            }
            self.wait_ack(addr.clone())?;
        }
        Ok(())
    }

    pub fn close(&self, addr : String) -> Result<(), std::io::Error>{
        let connection = self.connections.get(&addr).unwrap();
        let reset = Packet::new_reset(connection.sequence);
        self.socket.send_to(&reset.to_bytes(), addr)?;
        Ok(())
    }

    fn received_packet_connection(&mut self, packet : Packet, addr : String, tx : Vec<Sender<(String, Vec<u8>)>>) -> Result<(), std::io::Error>{
        let connection = self.connections.get_mut(&addr).unwrap();
        if packet.get_sequence() == connection.ack{
            if packet.is_reset(){
                println!("Received reset for {}", addr);
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
            loop {
                let t = tx.clone();
                self.receive_packet(t).unwrap();
            }
        });
 
        Ok(())
    }
}