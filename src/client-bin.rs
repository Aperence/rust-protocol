pub mod protocol;
use protocol::protocol::Protocol;

fn main() -> Result<(), std::io::Error>{
    println!("Hello from client");

    let mut client = Protocol::new("127.0.0.1:8081")?;

    let mut msg : [u8; 50000] = [0; 50000];
    for i in 0..50000 {
        msg[i] = b'a';
    }

    client.send(msg.to_vec(), "127.0.0.1:8080".to_string())?;
    // client.send(msg.to_vec())?;
    
    Ok(())
}