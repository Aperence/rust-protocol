use std::net::UdpSocket;

mod packets;
use packets::{ack::Ack, packet::Packet};

struct Client{
    socket : UdpSocket,
    sequence : u64,
    ack : u64
}

impl Client {
    pub fn new(local_addr : &str, peer_addr : &str) -> Result<Client, std::io::Error>{
        let socket = UdpSocket::bind(local_addr)?;
        socket.connect(peer_addr)?;
        Ok(Client{socket, sequence : 0, ack : 0})
    }

    pub fn send(&mut self, content : Vec<u8>) -> Result<(), std::io::Error>{
        let packet = Packet::new(content, self.sequence);
        self.socket.send(&packet.to_bytes())?;
        let mut buf : [u8; 64] = [0; 64];
        let amt = self.socket.recv(&mut buf)?;
        let ack = Ack::from_bytes(buf[..amt].to_vec());
        println!("{:?}", ack);
        if ack.get_sequence() == self.sequence{
            self.sequence = ack.get_acked();
        }
        Ok(())
    }
}

fn main() -> Result<(), std::io::Error>{
    println!("Hello from client");

    let mut client = Client::new("127.0.0.1:8081", "127.0.0.1:8080")?;

    let mut msg : [u8; 50000] = [0; 50000];
    for i in 0..50000 {
        msg[i] = b'a';
    }

    client.send(msg.to_vec())?;
    client.send(msg.to_vec())?;
    
    Ok(())
}