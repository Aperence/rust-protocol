pub mod protocol;
use std::sync::mpsc::channel;

use protocol::protocol::Protocol;

fn main() -> Result<(), std::io::Error>{
    println!("Hello from client");

    let mut client = Protocol::new("127.0.0.1:8081")?;

    let mut msg : [u8; 50000] = [0; 50000];
    for i in 0..50000 {
        msg[i] = b'a';
    }

    let addr = "127.0.0.1:8080".to_string();

    client.connect(addr.clone())?;
    client.send(msg.to_vec(), addr.clone())?;
    // client.send(msg.to_vec())?;

    client.close(addr)?;
    
    Ok(())
}