pub mod protocol;

use std::thread;

use protocol::Protocol;

fn main() -> Result<(), std::io::Error>{
    println!("Hello from server");

    let mut server = Protocol::new("127.0.0.1:8080")?;

    loop{
        let mut connection = server.listen()?;
        thread::spawn(move ||{
            let _ = connection.send("Hello".as_bytes().to_vec());

            while let Ok(x) = connection.recv(){
                println!("Received data of size {}", x.len());
            }
        });
    }
    
}