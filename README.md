# Rust Protocol

This repository contains a minimalist implementation of a TCP-like, reliable protocol in Rust above UDP.

Currently supported features are:

- Sending packet reliably, and handling losses automatically
- Receiving data
- Closing stream with the FIN flag
- Resetting streams
- Read/Write trait for connections (could be used for example with [Rust OpenSSL bindings](https://docs.rs/openssl/latest/openssl/ssl/struct.SslConnector.html))

Note that the only purpose of this implementation is to learn how a reliable protocol as TCP is built, and how we could design such a protocol in Rust.

Some example of usage are provided, you can run them using
```
# Launch a server listening for requests
cargo run --bin server -- --addr 127.0.0.1:8080

# Launch a client sending a request
cargo run --bin client -- --addr 127.0.0.1:8081 --peer 127.0.0.1:8080

# Launch 2 concurrent clients with big queries, to see the interleaving of requests
cargo run --bin client -- --addr 127.0.0.1:8081 --peer 127.0.0.1:8080 --size 1000000 \
 & cargo run --bin client -- --addr 127.0.0.1:8082 --peer 127.0.0.1:8080 --size 1000000
```