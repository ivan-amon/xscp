//! # XSCP Server
//! 
//! This binary provides an instance of an XSCP which works over TCP, currently
//! it's a single threaded server, only can handle one connection.

use std::io::{BufRead, BufReader, Write};
use std::{env, process};
use std::{net::TcpListener};

/// Runs an XSCP Server.
/// 
/// # Arguments
/// - `args[0]`: Binary 
/// - `args[1]`: XSCP Server Port
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        process::exit(1)
    }

    let port: u16 = args[1].parse().expect("Port must be a number");
    let listener = TcpListener::bind(("0.0.0.0", port))
        .expect("Failed to bind address, port may be in use");
    println!("Listening on port {port}");

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let mut buf_reader = BufReader::new(&stream);

        let peer = stream.peer_addr().unwrap();
        println!("{peer} connected");

        loop {
            let mut request = String::new();
            match buf_reader.read_line(&mut request) {
                Ok(0) => { // EOF
                    println!("{peer} disconnected");
                    break;
                }
                Ok(_) => {
                    print!("Request from {peer}: {request}");
                    if let Err(err) = (&stream).write_all(request.as_bytes()) {
                        eprintln!("Write error: {err}");
                        break;
                    }
                }
                Err(err) => {
                    eprintln!("Read error: {err}");
                    break;
                }
            };
        }
    }
}
