use clap::Parser;

pub mod protocol;
use protocol::Protocol;

/// Client for custom protocol
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Address used to bind the client
    #[arg(short, long)]
    addr: String,

    /// Address on which the other host is listening
    #[arg(short, long)]
    peer: String,

    // The size of the message to send
    #[arg(short, long, default_value_t = 50000)]
    size : u64
}

fn main() -> Result<(), std::io::Error>{
    let args : Args = Args::parse();

    println!("Hello from client");

    let mut client = Protocol::new(&args.addr)?;

    let mut connection = client.connect(args.peer.clone())?;

    let mut msg = vec![];
    for _ in 0..args.size {
        msg.push(b'a');
    }

    // first let's be polite and greet the server
    connection.send("Hey".as_bytes().to_vec())?;
    let data = connection.recv().unwrap();
    println!("Server said : {}", String::from_utf8(data).unwrap());

    // then let's send our long query
    connection.send(msg)?;
    // note that we can also use write for this
    // connection.write(&msg)?;

    // send a fin segment, meaning that we have finished transmitting
    connection.close()?;

    // wait for server to end transmitting
    while let Ok(data) = connection.recv(){
        println!("Server said : {}", String::from_utf8(data).unwrap());
    }

    // stop the client
    client.stop();
    
    Ok(())
}