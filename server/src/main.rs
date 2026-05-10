//! # XSCP Server.
use server::connection::connection::Connection;
use server::session::auth::Sessions;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use xscp::XscpRequest;

/// Runs an XSCP Server instance on the 7878 port.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let addr = "0.0.0.0:7878";
    let listener = TcpListener::bind(addr).await?;
    println!("XSCP Server is running on port 7878");

    let sessions: Sessions = Arc::new(Mutex::new(HashSet::new()));

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
    sessions: Sessions,
) -> Result<(), Box<dyn std::error::Error>> {

    let peer_addr = socket.peer_addr().unwrap();
    let (reader, mut writer) = socket.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut raw_request = String::new();
    let mut connection = Connection::new(peer_addr, sessions.clone());

    loop {
        raw_request.clear();

        let request = match buf_reader.read_line(&mut raw_request).await {
            Ok(0) => {
                println!("Connection closed by client.");
                return Ok(());
            }
            Ok(_) => {
                let request = match XscpRequest::parse(&raw_request) {
                    Ok(req) => req,
                    Err(_) => {
                        return Err("Failed to parse XSCP request".into());
                    }
                };
                request
            }
            Err(err) => {
                println!("Failed to read from socket: {}. Closing connection.", err);
                return Err(Box::new(err));
            }
        };

        let response = connection.handle(request);
        writer.write_all(response.reason_phrase().as_bytes()).await?; 
    }
}
