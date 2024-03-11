use clap::Parser;
use std::thread;

pub mod protocol;
use protocol::Protocol;

/// Server for custom protocol
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Address used to bind the server
    #[arg(short, long)]
    addr: String,
}

fn main() -> Result<(), std::io::Error>{
    let args : Args = Args::parse();

    let addr = args.addr;

    println!("Hello from server");

    let mut server = Protocol::new(&addr)?;

    loop{
        let mut connection = server.listen()?;
        // use one thread per connection
        thread::spawn(move ||{
            let peer = connection.get_peer_addr();
            let msg = connection.recv().unwrap();
            // we can also use read for receiving
            // ex: 
            // let mut buf : [u8; 2560] = [0; 2560];
            // let amt = connection.read(&mut buf).unwrap();
            // let msg = buf[..amt].to_vec();

            println!("Client {} said : {}", peer, String::from_utf8(msg).unwrap());

            // first we greet each other, then wait for its query
            let _ = connection.send("Hello".as_bytes().to_vec());

            // get all data from client
            while let Ok(x) = connection.recv(){
                println!("Received data of size {} from {}", x.len(), peer);
            }
            // we received a close, client has finished

            // send him a simple goodbye
            println!("Sending goodbye for client {}...", peer);
            let _ = connection.send("Goodbye, have a nice day".as_bytes().to_vec());

            // close our stream
            let _ = connection.close();
        });
    }

    //let _ = server.stop();
    
}