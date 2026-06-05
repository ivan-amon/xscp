//! # XSCP Client CLI
//!
//! This binary provides a CLI to send simple messages to an XSCP server over TCP.

use std::io::Write;
use client::auth;
use ::io::SocketIo;
use tokio::io::{self, AsyncBufReadExt, BufReader, Lines, Stdin};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    let socket = TcpStream::connect("127.0.0.1:7878").await.expect("Failed to connect to server");
    let mut socket_io = SocketIo::new(socket);
    println!("Connected to 127.0.0.1:7878");

    let mut stdin_lines = BufReader::new(io::stdin()).lines();

    let username = run_auth(&mut stdin_lines, &mut socket_io).await;

    println!("Logged in as {username}!");
}

/// Prompts for a username and authenticates against the server until it
/// succeeds, returning the authenticated username.
///
/// Borrows `stdin_lines` and `socket_io` so the caller can keep using them
/// afterwards. Terminates the process on a fatal outcome (auth error, EOF, or
/// an unexpected status code).
async fn run_auth(
    stdin_lines: &mut Lines<BufReader<Stdin>>,
    socket_io: &mut SocketIo,
) -> String {
    loop {
        print!("Username:");
        std::io::stdout().flush().expect("Failed to flush stdout");

        let line = stdin_lines.next_line().await;
        match line.expect("Error reading stdin") {
            Some(text) => {
                match auth(socket_io, &text).await {
                    Ok(200) => return text,
                    Ok(401) => {
                        println!("Invalid Credentials, try again");
                        continue;
                    }
                    Ok(402) => {
                        println!("Exceeded auth attempts");
                        std::process::exit(1);
                    }
                    Ok(code) => {
                        eprintln!("Unexpected server response (status {code}), exiting...");
                        std::process::exit(1);
                    }
                    Err(err) => {
                        eprintln!("Authentication failed: {err}");
                        std::process::exit(1);
                    }
                }
            }
            None => {
                println!("\nExiting...");
                std::process::exit(0);
            }
        }
    }
}
