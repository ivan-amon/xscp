use crate::{
    Action, 
    connection::Connection, 
    io::SocketIo, 
    session::storage::Sessions
};
use tokio::net::TcpStream;
use xscp::{XscpRequest, XscpResponse};

/// Runs an XSCP connection, using the connection State Machine
pub async fn run_connection(
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
            Ok(request) => match connection.handle(request).await {
                Action::Reply(response) => socket_io.write(&response.to_string()).await?,
                Action::ReplyAndClose(response) => {
                    socket_io.write(&response.to_string()).await?;
                    return Ok(());
                }
                Action::Close => {
                    return Ok(());
                }
                Action::Broadcast(_broadcast_envelope ) => todo!(),
            },
            Err(_) => {
                println!("Error parsing request from {peer_addr}");
                let response = XscpResponse::try_new(400, "Invalid Request").unwrap();
                socket_io.write(&response.to_string()).await?;
            }
        }
    }
}


