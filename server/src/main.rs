//! # XSCP Server.

use server::{
    connection::{BroadcastEnvelope, run_connection}, 
    session::storage::Sessions
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, sync::broadcast};

/// Runs an XSCP Server instance on the 7878 port.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:7878";
    let listener = TcpListener::bind(addr).await?;
    println!("XSCP Server is running on port 7878");

    let sessions: Sessions = Arc::new(Mutex::new(HashSet::new()));
    let (broadcast_tx, _) = broadcast::channel::<BroadcastEnvelope>(64);

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        println!("New connection from {peer_addr}");

        let sessions = Arc::clone(&sessions);
        let broadcast_tx = broadcast_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = run_connection(socket, sessions, broadcast_tx).await {
                eprintln!("Connection {peer_addr} ended with error: {e}");
            }
        });
    }
}
