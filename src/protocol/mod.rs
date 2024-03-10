use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, net::UdpSocket, sync::mpsc::Sender, thread, time::Duration};
use std::hash::{DefaultHasher, Hash, Hasher};
use rand::{self, random};

use std::io::{Error, ErrorKind};

pub mod packets;
pub mod connection;
use connection::Connection;
use packets::Packet;

fn hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub struct Protocol{
    pub socket : Arc<UdpSocket>,
    // next received connections
    listeners : Receiver<Connection>,
    // sender channel use to propagate the incoming connections
    sender : Arc<Sender<Connection>>,
    // a handle used to stop the receiving thread
    handle : Option<Sender<()>>,
    // map of <host, Sender> used to send packets to the corresponding connection
    connections : Arc<Mutex<HashMap<String, Sender<Packet>>>>
}


impl Protocol{

    /**
     * Create a new socket
     */
    pub fn new(addr : &str) -> Result<Protocol, std::io::Error>{
        let socket = Arc::new(UdpSocket::bind(addr)?);
        let (sender, listeners) = channel();
        let sender = Arc::new(sender);
        Ok(Protocol{socket, listeners, sender, handle : None, connections : Arc::new(Mutex::new(HashMap::new()))})
    }

    /**
     * Connect this socket to another host, used for client applications
     */
    pub fn connect(&mut self, addr : String) -> Result<Connection, std::io::Error>{
        let mut rto = Duration::from_millis(100);
        let max_transmit = 5;
        let mut transmit = 0;
        loop {
            transmit += 1;
            let seq : u16 = random(); // random between 0 and 64000
            let seq = seq as u64;
            let syn = Packet::new_syn(seq);
            self.socket.send_to(&syn.to_bytes(), addr.clone())?;
            println!("Sent syn");
    
            let mut buf = [0; 2560];
            self.socket.set_read_timeout(Some(rto))?;
            let amt = self.socket.recv(&mut buf);
            self.socket.set_read_timeout(None)?;

            if let Err(_) = amt{
                // exponential backoff
                if transmit > max_transmit{
                    return Err(Error::new(ErrorKind::ConnectionAborted, "Failed to connect"))
                }
                rto *= 2;
                //println!("Syn ack not received, retrying...");
                continue;
            }
            let amt = amt.unwrap();
            //println!("Received synack");
            let synack = Packet::from_bytes(buf[..amt].to_vec());
            if !synack.is_syn() || !synack.is_ack() || seq + 1 != synack.get_acked(){
                continue;
            }

            let (tx, rx) = channel();

            let connection = Connection::new(seq+1, synack.get_sequence()+1, self.socket.clone(), addr.clone(), rx, self.connections.clone());
     
            let ack = Packet::new_ack(seq+1, synack.get_sequence()+1);
            self.socket.send_to(&ack.to_bytes(), addr.clone())?;
            self.receive_loop(Some((addr.clone(), tx)));
            return Ok(connection);
        }
    }

    fn receive_loop(&mut self, conn : Option<(String, Sender<Packet>)>){
        if self.handle.is_some(){
            return;
        }

        let (tx, finished) = channel();
        self.handle = Some(tx);
        let sock = self.socket.clone();
        let sender = self.sender.clone();

        let connections = self.connections.clone();

        thread::spawn(move ||{
            if let Some((addr, sender)) = conn{
                connections.clone().lock().unwrap().insert(addr, sender);
            }
            loop {
                if finished.try_recv().is_ok(){
                    break;
                }
                let mut buf = [0; 2560];
                let res = sock.recv_from(&mut buf);
                if res.is_err(){
                    continue;
                }
                let (amt, src) = res.unwrap();
                let received = Packet::from_bytes(buf[..amt].to_vec());
    
                if received.is_syn(){
                    // begin handshake by sending syn-ack
                    let seq = hash(&src); // use an hash to avoid syn flooding
                    let synack = Packet::new_synack(seq, received.get_sequence()+1);
                    let _ = sock.send_to(&synack.to_bytes(), src);
                    continue;
                }

                let addr = src.to_string();

                if !connections.clone().lock().unwrap().contains_key(&addr){
                    // finalize the handshake
                    if !received.is_ack(){
                        continue;
                    }
                    let hashed = hash(&src);
                    if received.get_acked() != hashed + 1{
                        continue;
                    }
                    //println!("Correct ack, creating connection");
                    let (tx, rx) = channel();
                    let connection = Connection::new(hashed+1, received.get_sequence(), sock.clone(), addr.clone(), rx, connections.clone());
                    let _ = sender.send(connection);
                    connections.clone().lock().unwrap().insert(addr, tx);
                }else{
                    // data packet/reset, serve to correct connection
                    let arc = connections.clone();
                    let locked = arc.lock().unwrap();
                    let conn = locked.get(&addr).unwrap();
                    if received.is_reset(){
                        // remove directly without waiting for an ack
                        let _ = conn.send(received);
                        connections.lock().unwrap().remove(&addr);
                    }else{
                        // data/fin packet, send it to the connection
                        let _ = conn.send(received);
                    }
                }
            }
        });
    }

    /**
     * Listen for new connection, and return when a connection has been established
     */
    pub fn listen(&mut self) -> Result<Connection, Error>{
        self.receive_loop(None);
        match self.listeners.recv(){
            Ok(conn) => Ok(conn),
            Err(_) => Err(Error::new(ErrorKind::NotConnected, ""))
        }
    }

    /**
     * Stop the socket completly, note that this should be done after closing all connections
     */
    pub fn stop(self){
        if let Some(handle) = self.handle{
            let _ = handle.send(());
        }
    }
}