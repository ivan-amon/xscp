use crate::{
    Action,
    connection::{BroadcastEnvelope, Connection},
    io::SocketIo,
    session::storage::Sessions,
};
use std::ops::ControlFlow;
use tokio::{
    net::TcpStream,
    sync::broadcast::{Sender, error::RecvError},
};
use xscp::{XscpRequest, XscpResponse};

/// Runs the I/O loop for an XSCP connection, handling both incoming requests and broadcasts.
///
/// Manages a bidirectional communication loop using [`Connection`] FSM to process client requests.
/// Concurrently listens for:
/// - **Client requests** — incoming XSCP PDUs from the socket, processed via [`Connection::handle`]
/// - **Broadcast messages** — notifications from other connection tasks via the shared channel
///
/// # Arguments
///
/// - `socket` — the TCP connection to the remote peer
/// - `sessions` — shared session store for authentication
/// - `broadcast_tx` — sender side of the broadcast channel; used to register this connection
///   as a listener and forward outgoing broadcasts
///
/// # Termination
///
/// The loop exits when either:
/// - The client closes the socket (EOF)
/// - A request handler signals [`ControlFlow::Break`]
/// - The broadcast channel is closed (all senders dropped)
pub async fn run_connection(
    socket: TcpStream,
    sessions: Sessions,
    broadcast_tx: Sender<BroadcastEnvelope>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let peer_addr = socket.peer_addr()?;
    let mut socket_io = SocketIo::new(socket);
    let mut connection = Connection::new(peer_addr, sessions);
    let mut broadcast_rx = broadcast_tx.subscribe();

    loop {
        let flow = tokio::select! {
            read_result = socket_io.read() => { // Arrives request from socket
                handle_client_request(
                    read_result,
                    &mut socket_io,
                    &mut connection,
                    &broadcast_tx,
                ).await?
            }
            recv_result = broadcast_rx.recv() => { // Arrives notification from channel
                handle_broadcast(
                    recv_result,
                    &mut socket_io,
                    &connection,
                ).await?
            }
        };

        if flow.is_break() {
            println!("Connection closed by {peer_addr}");
            return Ok(());
        }
    }
}

/// Processes a PDU read from the client socket.
///
/// Returns [`ControlFlow::Break`] when the connection should terminate
/// (client EOF, [`Action::Close`], or [`Action::ReplyAndClose`]).
async fn handle_client_request(
    read_result: Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>,
    socket_io: &mut SocketIo,
    connection: &mut Connection,
    broadcast_tx: &Sender<BroadcastEnvelope>,
) -> Result<ControlFlow<()>, Box<dyn std::error::Error + Send + Sync>> {
    let raw_request = match read_result? {
        Some(raw) => raw,
        None => return Ok(ControlFlow::Break(())), //EOF
    };

    let request = match XscpRequest::parse(&raw_request) {
        Ok(req) => req,
        Err(_) => {
            let response = XscpResponse::try_new(400, "Invalid Request").unwrap();
            socket_io.write(&response.to_string()).await?;
            return Ok(ControlFlow::Continue(()));
        }
    };

    match connection.handle(request).await {
        Action::Reply(response) => {
            socket_io.write(&response.to_string()).await?;
            Ok(ControlFlow::Continue(()))
        }
        Action::ReplyAndClose(response) => {
            socket_io.write(&response.to_string()).await?;
            Ok(ControlFlow::Break(()))
        }
        Action::Close => Ok(ControlFlow::Break(())),
        Action::Broadcast(envelope) => {
            let _ = broadcast_tx.send(envelope);
            let response = XscpResponse::try_new(200, "Ok").unwrap();
            socket_io.write(&response.to_string()).await?;
            Ok(ControlFlow::Continue(()))
        }
    }
}

/// Processes a broadcast received from another connection task.
///
/// Filters out broadcasts whose `from` matches this connection's source so a
/// client never receives its own message back.
async fn handle_broadcast(
    recv_result: Result<BroadcastEnvelope, RecvError>,
    socket_io: &mut SocketIo,
    connection: &Connection,
) -> Result<ControlFlow<()>, Box<dyn std::error::Error + Send + Sync>> {
    let envelope = match recv_result {
        Ok(env) => env,
        Err(RecvError::Lagged(n)) => {
            eprintln!("Broadcast lagged by {n} messages");
            return Ok(ControlFlow::Continue(()));
        }
        Err(RecvError::Closed) => {
            return Ok(ControlFlow::Break(()));
        }
    };

    // Don't send Broadcast Notification to sender
    if let Some(my_source) = connection.source() {
        if envelope.from == my_source {
            return Ok(ControlFlow::Continue(()));
        }
    }

    socket_io.write(&envelope.payload).await?;
    Ok(ControlFlow::Continue(()))
}
