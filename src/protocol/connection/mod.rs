use crate::protocol::packets::Packet;
use core::time;
use std::io::{Error, ErrorKind};
use std::net::UdpSocket;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender};
use std::sync::Arc;
use std::time::Duration;


pub const MAX_SIZE: usize = 2560;

pub struct Connection{
    pub addr : String,
    pub sequence : u64,
    pub ack : u64,
    pub window : u64,
    pub in_flight : u64,
    pub socket : Arc<UdpSocket>,
    pub receiver : Receiver<Packet>,
    pub buffer : Receiver<Vec<u8>>,
    pub buffer_sender : Sender<Vec<u8>>
}

impl Connection{
    pub fn new(sequence : u64, ack : u64, socket : Arc<UdpSocket>, addr : String, receiver : Receiver<Packet>) -> Connection{
        let (tx, rx) = channel();
        Connection{sequence, ack, window : 4*(MAX_SIZE as u64), in_flight : 0, socket, addr, receiver, buffer : rx, buffer_sender : tx}
    }

    fn send_packet(&mut self, content : &Vec<u8>, init_sequence : u64) -> Result<(), Error>{
        let offset = (self.sequence + self.in_flight - init_sequence) as usize;
        let len: usize = content.len();
        let rem_window = (self.window - self.in_flight) as usize;
        let size_sending = usize::min(
            len - offset, 
            usize::min(MAX_SIZE, rem_window)
        );
        let sub = offset..offset+size_sending;
        let buf = content[sub].to_vec();
        let packet = Packet::new_data(buf, self.sequence + self.in_flight);
        println!("Sending packets with bytes from {} to {}", packet.get_sequence() - init_sequence, packet.get_sequence()+packet.get_size() - init_sequence);
        self.in_flight += size_sending as u64;
        self.socket.send_to(&packet.to_bytes(), self.addr.clone())?;
        Ok(())
    }

    pub fn send(&mut self, content : Vec<u8>) -> Result<(), Error>{
        
        let len = content.len() as u64;
        let init_sequence =  self.sequence;

        while self.sequence < init_sequence + len{
            let mut remaining = self.sequence + self.in_flight - init_sequence;
            while self.in_flight < self.window && remaining < len{
                self.send_packet(&content, init_sequence)?;
                remaining = self.sequence + self.in_flight - init_sequence;
            }
            let rto = Duration::from_millis(100);
            let data = self.receive(Some(rto));
            if let Err(_) = data{
                // rto reached
                self.in_flight = 0;
            }
        }
        Ok(())
    }

    fn receive(&mut self, timeout : Option<time::Duration>) -> Result<bool, Error>{
        loop{
            let packet ;
            if timeout.is_none(){
                let res = self.receiver.recv();
                if res.is_err(){
                    return Err(Error::new(ErrorKind::Interrupted, "Error"));
                }
                packet = res.unwrap();
            }else{
                let res = self.receiver.recv_timeout(timeout.unwrap());
                if let Err(_) = res{
                    return Err(Error::new(ErrorKind::Interrupted, ""));
                }
                packet = res.unwrap();
            }
            if packet.is_ack(){
                if packet.get_acked() > self.sequence{
                    // correct sequence, move on in window
                    self.in_flight -= packet.get_acked() - self.sequence;
                    self.sequence = packet.get_acked();
                    return Ok(false);
                }
                // resend ack
                let ack = Packet::new_ack(self.sequence, self.ack);
                let _ = self.socket.send_to(&ack.to_bytes(), self.addr.clone());
                return Ok(false);
            }
            if packet.get_sequence() == self.ack{
                // data packet or reset
                if packet.is_reset(){
                    return Err(Error::new(ErrorKind::ConnectionReset, ""))
                }
                let ack = Packet::new_ack(self.sequence, self.ack + packet.get_size());
                let err = self.socket.send_to(&ack.to_bytes(), self.addr.clone());
                if let Err(_) = err{
                    return Err(Error::new(ErrorKind::Interrupted, "No data"));
                }
                self.ack += packet.get_size();
                // serve data to application
                let _ = self.buffer_sender.send(packet.get_content());
                return Ok(true)
            }
        }
    }

    pub fn recv(&mut self) -> Result<Vec<u8>, RecvError>{
        if let Ok(data) = self.buffer.try_recv(){
            return Ok(data);
        }
        while let Ok(false) = self.receive(None){}
        if let Ok(data) = self.buffer.try_recv(){
            return Ok(data);
        }
        Err(RecvError)
    }

    pub fn close(self) -> Result<(), std::io::Error>{
        let reset = Packet::new_reset(self.sequence);
        self.socket.send_to(&reset.to_bytes(), self.addr)?;
        Ok(())
    }
}