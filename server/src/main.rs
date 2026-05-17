//! # XSCP Server.

use server::{
    Action,
    connection::Connection,
    io::SocketIo,
    session::storage::Sessions,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::net::{TcpListener, TcpStream};
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
        println!("New connection from {peer_addr}");

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
    let mut socket_io = SocketIo::new(socket);
    let mut connection = Connection::new(peer_addr, sessions);

    loop {
        let raw_request = match socket_io.read().await? {
            Some(raw) => raw,
            None => {
                println!("Connection closed by {peer_addr}");
                return Ok(());
            }
        };

        match XscpRequest::parse(&raw_request) {
            Ok(request) => {
                match connection.handle(request).await {
                    Action::Reply(response) => {
                        socket_io.write(&response.to_string()).await?
                    },
                    Action::ReplyAndClose(response) => {
                        socket_io.write(&response.to_string()).await?;
                        return Ok(());
                    },
                    Action::Close => {
                        return Ok(());
                    },
                }},
            Err(_) => {
                println!("Error parsing request from {peer_addr}");
                let response =  XscpResponse::try_new(400, "Invalid Request").unwrap();
                socket_io.write(&response.to_string()).await?;
            }
        }
    }
}
