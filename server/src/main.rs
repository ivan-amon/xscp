//! # XSCP Server.
use server::connection::connection::Connection;
use server::session::storage::Sessions;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream, tcp::OwnedReadHalf};
use xscp::{XscpRequest, XscpResponse};

/// Runs an XSCP Server instance on the 7878 port.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let addr = "0.0.0.0:7878";
    let listener = TcpListener::bind(addr).await?;
    println!("XSCP Server is running on port 7878");

    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        println!("New connection from {}.", peer_addr);

        let sessions = Arc::clone(&sessions);

        tokio::spawn(async move {
            if let Err(e) = run_connection(socket, sessions).await {
                eprintln!("Connection {peer_addr} ended with error: {e}");
            }
        });
    }
}

async fn run_connection(
    socket: TcpStream,
    sessions: Sessions,
) -> Result<(), Box<dyn std::error::Error>> {

    let peer_addr = socket.peer_addr().unwrap();
    let (reader, mut writer) = socket.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut connection = Connection::new(peer_addr, sessions);

    loop {
        let raw_request = match read_socket(&mut buf_reader).await? {
            Some(raw) => raw,
            None => {
                println!("Connection closed by client");
                return Ok(());
            }
        };

        let response = match XscpRequest::parse(&raw_request) {
            Ok(request) => { connection.handle(request) },
            Err(_) => { XscpResponse::try_new(400, "INVALID REQUEST").unwrap() }
        };

        writer.write_all(response.reason_phrase().as_bytes()).await?; 
    }
}

async fn read_socket(
    reader: &mut BufReader<OwnedReadHalf>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {

    let mut buf = String::new();
    match reader.read_line(&mut buf).await {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(buf)),
        Err(err) => Err(Box::new(err)),
    }
}
