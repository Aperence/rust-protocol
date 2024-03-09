pub mod protocol;
use protocol::server::Server;

/*
    TODO:
        - send on server/receive on client
        - Multiple threads for connections
        - serve content to server
*/

fn main() -> Result<(), std::io::Error>{
    println!("Hello from server");
    
    let mut server = Server::new("127.0.0.1:8080")?;
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.
    loop {
        let _received = server.receive().unwrap();
    }


    //println!("{}", _received);
    // Ok(())
}