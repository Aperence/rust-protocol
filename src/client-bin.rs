pub mod protocol;

use protocol::Protocol;

fn main() -> Result<(), std::io::Error>{
    println!("Hello from client");

    let mut client = Protocol::new("127.0.0.1:8081")?;


    let peer_addr = "127.0.0.1:8080".to_string();

    let mut connection = client.connect(peer_addr.clone())?;

    let mut msg : [u8; 50000] = [0; 50000];
    for i in 0..50000 {
        msg[i] = b'a';
    }

    connection.send(msg.to_vec())?;

    let data = connection.recv().unwrap();
    println!("{}", String::from_utf8(data).unwrap());
    // client.send(msg.to_vec())?;

    connection.close()?;
    
    Ok(())
}