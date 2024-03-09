use std::collections::HashMap;
use std::{net::UdpSocket, time::Duration};

use crate::protocol::connection;
use crate::protocol::packets::packet::Packet;

use crate::protocol::connection::connection::{Connection, MAX_SIZE};
use rand::{self, random};

pub struct Client{
    socket : UdpSocket,
    connections : HashMap<String, Connection>
}

impl Client {
    pub fn new(local_addr : &str) -> Result<Client, std::io::Error>{
        let socket = UdpSocket::bind(local_addr)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        Ok(Client{socket, connections : HashMap::new()})
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
        let amt = self.socket.recv(&mut buf);
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

    fn connect(&mut self, addr : String) -> Result<bool, std::io::Error>{
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
            return Ok(false);
        }
        let connection = Connection::new(seq+1, synack.get_acked()+1);
 
        let ack = Packet::new_ack(connection.sequence, synack.get_sequence()+1);
        self.socket.send_to(&ack.to_bytes(), addr.clone())?;

        self.connections.insert(addr, connection);

        Ok(true)
    }

    /**
     * Go-back-n implementation for sending packets
     */
    pub fn send(&mut self, content : Vec<u8>, addr : String) -> Result<(), std::io::Error>{
        while !self.connect(addr.clone()).unwrap(){}
        println!("Done handshake");
        
        let len = content.len() as u64;
        let mut init_sequence = 0;
        let mut curr_seq = 0;
        let mut inflight = 0;
        let mut remaining = 0;
        let mut window = 0;

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
        let connection = self.connections.get(&addr).unwrap();
        let reset = Packet::new_reset(connection.sequence);
        self.socket.send_to(&reset.to_bytes(), addr)?;
        Ok(())
    }
}