use std::{io::Read, net::UdpSocket};

mod packets;
use packets::packet::Packet;
use packets::ack::Ack;

struct Server{
    socket : UdpSocket
}

impl Server {
    pub fn new(addr : &str) -> Result<Server, std::io::Error>{
        let socket = UdpSocket::bind(addr)?;
        Ok(Server{socket})
    }

    pub fn receive(&self) -> Result<String, std::io::Error>{
        let mut buf = [0; 64000];
        let (amt, src) = self.socket.recv_from(&mut buf)?;

        let received = Packet::from_bytes(buf[..amt].to_vec());

        let ack = Ack::new(received.get_sequence(), received.get_sequence() + received.get_size());
        self.socket.send_to(&ack.to_bytes(), src)?;

        println!("{}", amt);
    
        
        Ok(String::from_utf8(received.get_content()).unwrap())
    }
}

/*
    TODO:
        - Go-back-n
        - Handle losses
        - send on server/receive on client
        - Multiple threads for connections
*/

fn main() -> Result<(), std::io::Error>{
    println!("Hello from server");
    
    let server = Server::new("127.0.0.1:8080")?;
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.
    loop {
        let received = server.receive().unwrap();
    }


    //println!("{}", received);
    Ok(())
}