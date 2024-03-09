pub mod protocol;
use protocol::client::Client;

fn main() -> Result<(), std::io::Error>{
    println!("Hello from client");

    let mut client = Client::new("127.0.0.1:8081", "127.0.0.1:8080")?;

    let mut msg : [u8; 50000] = [0; 50000];
    for i in 0..50000 {
        msg[i] = b'a';
    }

    client.send(msg.to_vec())?;
    // client.send(msg.to_vec())?;
    
    Ok(())
}