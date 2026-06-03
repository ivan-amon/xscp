//! # XSCP Client CLI
//!
//! This binary provides a CLI to send simple messages to an XSCP server over TCP.

use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    let stream = TcpStream::connect("127.0.0.1:7878").await.expect("Failed to connect to server");
    println!("Connected to 127.0.0.1:7878");

    let mut socket_lines = BufReader::new(stream).lines();
    let mut stdin_lines = BufReader::new(io::stdin()).lines();

    loop {
        tokio::select! {
            line = stdin_lines.next_line() => {
                match line.expect("Error reading stdin") {
                    Some(text) => println!("You: {text}"),
                    None => {
                        println!("stdin EOF, exiting...");
                        break;
                    }
                }
            }

            line = socket_lines.next_line() => {
                match line.expect("ERROR > couldn't read from server") {
                    Some(msg) => println!("Server: {msg}"),
                    None => {
                        println!("Server closed the connection.");
                        break;
                    }
                }
            }
        }
    }
}
