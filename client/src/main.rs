//! # XSCP Client CLI
//!
//! This binary provides a CLI to send simple messages to an XSCP server over TCP.

use std::io::Write;
use client::auth;
use ::io::SocketIo;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    let socket = TcpStream::connect("127.0.0.1:7878").await.expect("Failed to connect to server");
    let mut socket_io = SocketIo::new(socket);
    println!("Connected to 127.0.0.1:7878");

    let mut stdin_lines = BufReader::new(io::stdin()).lines();
    let mut username = String::new();

    // Authentication
    loop {

        print!("Username:");
        std::io::stdout().flush().expect("Failed to flush stdout");

        let line = stdin_lines.next_line().await;
        match line.expect("Error reading stdin") {
            Some(text) => {
                let response_code = auth(&mut socket_io, &text).await;

                match  response_code {
                    200 => {
                        username = text;
                        break;
                    },
                    401 => {
                        println!("Invalid Credentials, try again");
                        continue;
                    }
                    _ => std::process::exit(1),
                }
            },
            None => {
                println!("stdin EOF, exiting...");
                break;
            }
        }
    }

    println!("Logged in as {username}!");

    // loop {
    //     tokio::select! {
    //         line = stdin_lines.next_line() => {
    //             todo!()
    //         }
    //         // line = socket_lines.next_line() => {
    //         //     todo!()
    //         // }
    //     }
    // }
}
