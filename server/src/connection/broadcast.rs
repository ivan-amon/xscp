use std::{collections::HashMap, net::SocketAddr, sync::Mutex};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use xscp::XscpNotification;

pub async fn broadcast(
    source: &str,
    source_addr: SocketAddr,
    message: &str,
    sessions: &Mutex<HashMap<String, SocketAddr>>,
) {

    let destinations: Vec<SocketAddr> = {
        let guard = sessions.lock().unwrap();
        guard.values().copied().collect()
    };

    let Ok(notification) = XscpNotification::try_new(
        xscp::NotificationType::Broadcast, source, message
    ) else {
        eprintln!("Invalid broadcast payload from {source_addr}");
        return;
    };

    let payload = notification.to_string();

    for dest_addr in destinations {
        if dest_addr == source_addr { continue }

        let mut stream = match TcpStream::connect(dest_addr).await {
            Ok(stream) => stream,
            Err(err) => {
                eprintln!("Connect failed {source_addr} -> {dest_addr}: {err}");
                continue;
            }
        };

        if let Err(err) = stream.write_all(payload.as_bytes()).await {
            eprintln!("Write failed {source_addr} -> {dest_addr}: {err}");
            continue;
        }
    }
}
