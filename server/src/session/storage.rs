use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

pub type Sessions = Arc<Mutex<HashMap<String, SocketAddr>>>;

/// Registers a host session by storing its source name and peer address.
///
/// # Arguments
/// - `source`: source name to register.
/// - `peer`: remote socket address associated with the session.
/// - `sessions`: map of active source names to their peer addresses.
///
/// # Errors
/// Returns `Err` if the source name is already registered.
pub fn store_session(
    source: String,
    peer: SocketAddr,
    sessions: &mut HashMap<String, SocketAddr>,
) -> Result<(), &'static str> {
    
    if sessions.contains_key(&source) {
        return Err("Source name is already in use.");
    }
    sessions.insert(source, peer);
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    #[test]
    fn correct_session() {
        let mut sessions = HashMap::<String, SocketAddr>::new();
        let peer = addr(8001);

        store_session("Test".to_string(), peer, &mut sessions).unwrap();

        assert_eq!(sessions.get("Test"), Some(&peer));
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn multiple_correct_sessions() {
        let mut sessions = HashMap::<String, SocketAddr>::new();

        let peer_1 = addr(8001);
        store_session("Test 1".to_string(), peer_1, &mut sessions).unwrap();

        let peer_2 = addr(8002);
        store_session("Test 2".to_string(), peer_2, &mut sessions).unwrap();

        assert_eq!(sessions.get("Test 1"), Some(&peer_1));
        assert_eq!(sessions.get("Test 2"), Some(&peer_2));
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn source_taken() {
        let mut sessions = HashMap::<String, SocketAddr>::new();

        let peer_1 = addr(8001);
        store_session("Test".to_string(), peer_1, &mut sessions).unwrap();

        let peer_2 = addr(8002);
        let err = store_session("Test".to_string(), peer_2, &mut sessions).unwrap_err();

        assert_eq!("Source name is already in use.", err);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions.get("Test"), Some(&peer_1));
    }
}
