//! Shared helpers for server integration tests.
use server::{
    connection::{BroadcastEnvelope, run_connection},
    session::storage::Sessions,
};
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        TcpListener, TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::broadcast,
};

/// Spawns an XSCP server on an ephemeral loopback port and returns its address.
///
/// Mirrors the accept loop in `main.rs` but binds to `127.0.0.1:0` so each test
/// gets an isolated server on its own port. The background task is detached and
/// dies when the test process exits.
pub async fn spawn_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let sessions: Sessions = Arc::new(Mutex::new(HashSet::new()));
    let (broadcast_tx, _) = broadcast::channel::<BroadcastEnvelope>(64);

    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let sessions = Arc::clone(&sessions);
            let broadcast_tx = broadcast_tx.clone();
            tokio::spawn(async move {
                let _ = run_connection(socket, sessions, broadcast_tx).await;
            });
        }
    });

    addr
}

/// Test client wrapping a TCP socket with line-based read/write for XSCP PDUs.
pub struct TestClient {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl TestClient {
    pub async fn connect(addr: SocketAddr) -> Self {
        let stream = TcpStream::connect(addr).await.unwrap();
        let (r, w) = stream.into_split();
        Self {
            reader: BufReader::new(r),
            writer: w,
        }
    }

    pub async fn send(&mut self, pdu: &str) {
        self.writer.write_all(pdu.as_bytes()).await.unwrap();
    }

    /// Reads one line from the server. Returns `None` on EOF.
    pub async fn recv(&mut self) -> Option<String> {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf).await.unwrap() {
            0 => None,
            _ => Some(buf),
        }
    }
}
