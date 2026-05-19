//! # XSCP Server
//!
//! A multi-client message broadcasting server using the XSCP protocol.
//!
//! ## Protocol
//!
//! See the [XSCP specification](https://xscp.ivanamon.dev/) for details on the wire format
//! and state transitions.

use server::{
    connection::{BroadcastEnvelope, run_connection},
    session::storage::Sessions,
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, sync::broadcast};

/// Starts the XSCP server and listens for incoming connections.
///
/// Binds to `0.0.0.0:7878` and enters an infinite accept loop. For each incoming connection:
/// - Creates a new task via [`tokio::spawn`]
/// - Passes the socket, shared session store, and broadcast channel to [`run_connection`]
/// - The task runs until the connection closes or an error occurs
///
/// # Shared State
///
/// - [`Sessions`] — protected by a mutex; tracks currently authenticated client names
/// - **Broadcast channel** — with capacity 64; messages broadcast from one connection to all others
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:7878";
    let listener = TcpListener::bind(addr).await?;
    println!("XSCP Server is running on port 7878");

    let sessions: Sessions = Arc::new(Mutex::new(HashSet::new()));
    let (broadcast_tx, _) = broadcast::channel::<BroadcastEnvelope>(64);

    // Accept loop: listen for incoming connections indefinitely
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
