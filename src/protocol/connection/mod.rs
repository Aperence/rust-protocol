use crate::protocol::packets::Packet;
use core::time;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::UdpSocket;
use std::thread::{self, sleep};
use std::sync::mpsc::{channel, Receiver, RecvError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;


pub const MAX_SIZE: usize = 2560;
pub const MSL: Duration = Duration::from_secs(120);

pub struct Connection{
    // address of other host
    addr : String,  
    // current sequence number
    sequence : u64,
    // current ack number for peer
    ack : u64,
    // size of the sending window
    window : u64,
    // number of bytes in flight
    in_flight : u64,
    // if we already sent a fin (stream closed to other host)
    sent_fin : bool,
    // if we received a fin
    received_fin : bool,
    // socket to other host
    socket : Arc<UdpSocket>,
    // buffer containing packets for this connection
    receiver : Receiver<Packet>,
    // data buffer
    buffer : Receiver<Vec<u8>>,
    // sender for the data buffer
    buffer_sender : Sender<Vec<u8>>,
    // map of all connections, used to clean up when receiving fin
    connections : Arc<Mutex<HashMap<String, Sender<Packet>>>>
}

impl Connection{
    pub fn new(sequence : u64, ack : u64, socket : Arc<UdpSocket>, addr : String, receiver : Receiver<Packet>, connections : Arc<Mutex<HashMap<String, Sender<Packet>>>>) -> Connection{
        let (tx, rx) = channel();
        Connection{sequence, 
            ack, window : 4*(MAX_SIZE as u64), 
            in_flight : 0, socket, addr, receiver, 
            buffer : rx, buffer_sender : tx, received_fin : false, 
            sent_fin : false, connections}
    }

    pub fn get_peer_addr(&self) -> String{
        self.addr.clone()
    }

    /**
     * Send a single part of data
     */
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
        self.in_flight += size_sending as u64;
        self.socket.send_to(&packet.to_bytes(), self.addr.clone())?;
        Ok(())
    }

    /**
     * Send some data to another host
     */
    pub fn send(&mut self, content : Vec<u8>) -> Result<(), Error>{
        let len = content.len() as u64;
        let init_sequence =  self.sequence;

        // go-back-n implementation
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

    /**
     * Core receive loop, 
     * return Ok(true) if real data was received
     * return Ok(false) if an ack was received
     * return Err otherwise (fin/reset)
     */
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
            }
            if packet.get_sequence() == self.ack{
                // data packet or reset or fin
                if packet.is_reset(){
                    return Err(Error::new(ErrorKind::ConnectionReset, ""))
                }
                if packet.is_fin(){
                    //println!("Received fin");
                    self.received_fin = true;
                    let ack = Packet::new_ack(self.sequence, self.ack+1);
                    self.socket.send(&ack.to_bytes())?;
                    // maintain state during 2*msl if fin_sent = true
                    if self.sent_fin{
                        let arc = self.connections.clone();
                        let addr = self.addr.clone();
                        thread::spawn(move ||{
                            sleep(3*MSL);
                            arc.lock().unwrap().remove(&addr);
                        });
                    }
                    return Err(Error::new(ErrorKind::Interrupted, "Fin"))
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
            }else{
                // resend ack
                let ack = Packet::new_ack(self.sequence, self.ack);
                let _ = self.socket.send_to(&ack.to_bytes(), self.addr.clone());
                return Ok(false);
            }
        }
    }

    /**
     * Receive some content from this connection
     */
    pub fn recv(&mut self) -> Result<Vec<u8>, RecvError>{
        // try to get some data from the buffer if already available
        if let Ok(data) = self.buffer.try_recv(){
            return Ok(data);
        }
        // loop until getting real data
        while let Ok(false) = self.receive(None){}
        if let Ok(data) = self.buffer.try_recv(){
            return Ok(data);
        }
        // no more data, end of stream
        Err(RecvError)
    }

    /**
     * Close the connection reliably
     */
    pub fn close(&mut self) -> Result<(), std::io::Error>{
        //println!("Sending fin");
        loop {
            let fin = Packet::new_fin(self.sequence);
            self.socket.send_to(&fin.to_bytes(), self.addr.clone())?;
            let rto = Duration::from_millis(100);
            let data = self.receive(Some(rto));
            if let Err(_) = data{
                self.sent_fin = true;
                if self.received_fin{
                    let mut map = self.connections.lock().unwrap();
                    map.remove(&self.addr);
                }
                break;
            }
        }
        Ok(())
    }

    /**
     * Send a reset packet, closing immediatly the connection
     * May create losses
     */
    pub fn reset(self) -> Result<(), std::io::Error>{
        let reset = Packet::new_reset(self.sequence);
        self.socket.send_to(&reset.to_bytes(), self.addr)?;
        Ok(())
    }
}