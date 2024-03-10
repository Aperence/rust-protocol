pub mod protocol;
use std::sync::mpsc::channel;
use std::thread::{self, sleep};

use protocol::protocol::Protocol;

/*
    TODO:
        - send on server/receive on client
*/

fn main() -> Result<(), std::io::Error>{
    println!("Hello from server");

    let server = Protocol::new("127.0.0.1:8080")?;
    let nthreads = 8;

    let mut handles = Vec::new();
    let mut senders = Vec::new();

    for i in 0..nthreads{
        let (tx, rx) = channel();

        let idx = i;
        let handle = thread::spawn(move || {
            // some work here
            while let Ok((addr, _data)) = rx.recv(){
                println!("Received packet from {} in thread {}", addr, idx);
            }
        });

        handles.push(handle);
        senders.push(tx);
    }

    server.receive(senders)?;

    let handle = handles.remove(0);
    handle.join().unwrap();

    //println!("{}", _received);
    Ok(())
}