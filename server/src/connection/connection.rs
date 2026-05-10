//! Manages the XSCP connection state machine.
//!
//! The state transitions are defined in the
//! [protocol spec](https://xscp.ivanamon.dev/state-machine.html).
//!
//! ## States
//!
//! - [`State::Negotiating`] — awaiting successful authentication
//! - [`State::Established`] — session authenticated and active
//! - [`State::Aborted`] — connection terminated
use std::net::SocketAddr;
use xscp::{XscpRequest, XscpResponse};
use crate::session::auth::{Sessions, auth};

/// All Connection States.
///
/// Note: the LISTEN → NEGOTIATING transition is handled upstream,
/// before a [`Connection`] is created.
#[derive(Debug, PartialEq)]
pub enum State {
    Negotiating { attempts: u8 },
    Established { source: String },
    Aborted,
}

/// Represents an active XSCP connection with a remote peer.
///
/// Holds the current [`State`] of the connection, the peer's address,
/// and a reference to the shared session store used for authentication.
///
/// # Lifecycle
///
/// While in [`State::Established`], the connection owns an entry in the
/// shared [`Sessions`] set. The entry is removed automatically on drop,
/// so the set always reflects currently-active sessions.
pub struct Connection {
    peer_addr: SocketAddr,
    state: State,
    sessions: Sessions
}

impl Connection {
    /// Creates a new [`Connection`] in the [`State::Negotiating`] state.
    ///
    /// # Arguments
    /// - `peer_addr` — the remote socket address of the connecting client.
    /// - `sessions` — shared session store used to validate credentials during login.
    pub fn new(peer_addr: SocketAddr, sessions: Sessions) -> Self {
        Self { peer_addr, state: State::Negotiating { attempts: 0 }, sessions }
    }

    /// Processes the next incoming request and advances the connection state machine.
    ///
    /// Returns the [`XscpResponse`] that should be sent back to the peer.
    /// Transitions the connection to [`State::Established`] on successful auth,
    /// or to [`State::Aborted`] on too many failed attempts.
    pub fn handle(&mut self, request: XscpRequest) -> XscpResponse<'static> {

        let response = match &self.state {

            State::Negotiating { attempts } => {
                let attempts = *attempts;
                let response = self.negotiate(&request, attempts);
                match response.status_code() {
                    200 => {
                        println!("{} logged in successfully", self.peer_addr);
                        self.state = State::Established { source: request.source().to_string() };
                    }
                    400 => {
                        println!("Invalid request from: {}", self.peer_addr);
                        self.state = State::Negotiating { attempts: attempts + 1 };
                    }
                    401 => {
                        println!("Invalid Credentials from: {}", self.peer_addr);
                        self.state = State::Negotiating { attempts: attempts + 1 };
                    }
                    402 => {
                        println!("{} exceeded auth attempts", self.peer_addr);
                        self.state = State::Aborted;
                    }
                    _ => {}
                }
                response
            },

            State::Established { source: _ } => todo!(),

            State::Aborted => todo!(),
        };
        response
    }

    fn negotiate(&self, request: &XscpRequest, attempts: u8) -> XscpResponse<'static> {
        match request.opcode() {
            xscp::OpCode::Login => auth(request, attempts, &self.sessions),
            _                   => XscpResponse::try_new(400, "INVALID REQUEST").unwrap(),
        }
    }
}

impl Drop for Connection {
    /// Removes this connection's source from [`Sessions`] when dropped
    /// while in [`State::Established`].
    ///
    /// In any other state no cleanup is performed, since no name was
    /// registered on behalf of this connection.
    fn drop(&mut self) {
        if let State::Established { source } = &self.state {
            if let Ok(mut guard) = self.sessions.lock() {
                guard.remove(source);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};
    use crate::session::auth::Sessions;

    fn dummy_sessions() -> Sessions {
        Arc::new(Mutex::new(HashSet::new()))
    }

    #[test]
    fn negotiating_to_established() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        let mut connection = Connection::new(peer_addr, sessions);

        assert_eq!(connection.state, State::Negotiating { attempts: 0 });

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let source = request.source().to_string();
        let _response = connection.handle(request);

        assert_eq!(connection.state, State::Established { source });
        assert_ne!(connection.state, State::Aborted);
        assert_ne!(connection.state, State::Negotiating { attempts: 0 })
    }

    #[test]
    fn negotiating_to_negotiating() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        sessions.lock().unwrap().insert("test".to_string());
        let mut connection = Connection::new(peer_addr, sessions);

        assert_eq!(connection.state, State::Negotiating { attempts: 0 });

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let source = request.source().to_string();
        let _response = connection.handle(request);

        assert_eq!(connection.state, State::Negotiating { attempts: 1 });
        assert_ne!(connection.state, State::Established { source });
        assert_ne!(connection.state, State::Aborted);
    }

    #[test]
    fn negotiating_to_aborted() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        sessions.lock().unwrap().insert("test".to_string());
        let mut connection = Connection::new(peer_addr, sessions);

        assert_eq!(connection.state, State::Negotiating { attempts: 0 });

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let source = request.source().to_string();
        let _response = connection.handle(request);
        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let _response = connection.handle(request);
        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let _response = connection.handle(request);
        
        assert_eq!(connection.state, State::Aborted);
        assert_ne!(connection.state, State::Negotiating { attempts: 1 });
        assert_ne!(connection.state, State::Established { source });
    }

    #[test]
    fn invlaid_login_request() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        sessions.lock().unwrap().insert("test".to_string());
        let mut connection = Connection::new(peer_addr, sessions);

        let request = XscpRequest::try_new(xscp::OpCode::Send, "invalid", "msg").unwrap();
        let response = connection.handle(request);

        assert_eq!(response.status_code(), 400);
        assert_eq!(response.reason_phrase(), "INVALID REQUEST");
        assert_eq!(connection.state, State::Negotiating { attempts: 1 });
    }
}