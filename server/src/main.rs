//! # XSCP Server
//!
//! This crate implements an example of an XSCP concurrent server.
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use server::store_session;
use xscp::XscpRequest;

const MAX_AUTH_ATTEMPTS: u8 = 3;

/// Runs an XSCP Server instance on the 7878 port.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:7878";
    let listener = TcpListener::bind(addr).await?;
    println!("XSCP Server is running on port 7878");

    let sessions: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        println!("New connection from {}.", peer_addr);

        let sessions = Arc::clone(&sessions);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, sessions).await {
                eprintln!("Connection {peer_addr} ended with error: {e}");
            }
        });
    }
}

async fn handle_connection(
    socket: TcpStream,
    sessions: Arc<Mutex<HashSet<String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Read the request from the socket.
    // 2. Negotiate (authentication, max 3 attempts).
    // 3. If auth succeeds, process the request and send a response.
    let mut auth_attempts: u8 = 0;
    
    let (reader, mut writer) = socket.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut raw_request = String::new();

    loop {
        raw_request.clear();

        let xscp_request = match buf_reader.read_line(&mut raw_request).await {
            Ok(0) => {
                println!("Connection closed by client.");
                return Ok(());
            },
            Ok(_) => {
                let xscp_request = match XscpRequest::parse(&raw_request) {
                    Ok(req) => req,
                    Err(_) => {
                        return Err("Failed to parse XSCP request".into());
                    }
                };
                xscp_request
            },
            Err(err) => {
                println!("Failed to read from socket: {}. Closing connection.", err);
                return Err(Box::new(err));
            }
        };

        // todo: add the other opcodes and their handling logic
        match xscp_request.opcode() {
            xscp::OpCode::Login => {
                let result = {
                    let mut guard = sessions.lock().unwrap();
                    store_session(&xscp_request, &mut guard)
                };
                match result {
                    Ok(_) => {
                        println!("User '{}' logged in successfully.", xscp_request.source());
                        writer.write_all(b"Login successful\r\n").await?; // todo: send an XscpResponse
                    },
                    Err(err) => {
                        println!("Login failed for host '{}': {}.", xscp_request.source(), err);
                        writer.write_all(format!("Login failed: {}\r\n", err).as_bytes()).await?; // todo: send an XscpResponse
                        auth_attempts += 1;
                    }
                }
            },
            xscp::OpCode::Send => todo!(),
            xscp::OpCode::Exit => todo!(),
        }


        if auth_attempts >= MAX_AUTH_ATTEMPTS {
            println!("Maximum authentication attempts reached. Closing connection.");
            return Err("Maximum authentication attempts reached".into());
        }
    }
}
