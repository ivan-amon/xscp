//! # XSCP Client CLI
//!
//! This binary provides a CLI to send simple messages to an XSCP server over TCP.

use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::{env, process};

/// Connects to XSCP Server as a client.
///
/// # Arguments
/// - `args[0]`: Binary
/// - `args[1]`: XSCP Server IP Address
/// - `args[2]`: XSCP Server Port
/// - `args[3]`: Message
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Invalid number of arguments");
        process::exit(1)
    }

    // TODO: Verify that args[1] is an IP Address and args[2] is a port
    // (maybe some REGEX crate could do that)
    let ip_addr = args[1].as_str();
    let port: u16 = args[2].parse().expect("Port must be a number");
    let stream = TcpStream::connect((ip_addr, port)).unwrap();
    println!("Connected successfully to {ip_addr}:{port}");

    let mut buf_reader = BufReader::new(&stream);

    loop {
        let mut msg = String::new();
        io::stdin().read_line(&mut msg).expect("Error reading line");
        (&stream).write_all(msg.as_bytes()).unwrap();

        let mut response = String::new();
        match buf_reader.read_line(&mut response) {
            Ok(0) => { // EOF
                println!("Server closed the connection");
            }
            Ok(_) => print!("Echo: {response}"),
            Err(err) => {
                eprintln!("Read error: {err}");
                break;
            }
        };
    }
}
