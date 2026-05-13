//! Manages the XSCP connection state machine.
//!
//! The state transitions are defined in the
//! [protocol spec](https://xscp.ivanamon.dev/state-machine.html).
//!
//! ## States
//!
//! - [`State::Negotiating`] — awaiting successful authentication
//! - [`State::Established`] — session authenticated and active
//!
//! Termination is signaled via [`Action::Close`] /
//! [`Action::ReplyAndClose`]
use std::net::SocketAddr;
use xscp::{XscpRequest, XscpResponse};
use crate::session::auth::auth;
use crate::session::storage::Sessions;

/// All Connection States.
///
/// Note: the LISTEN → NEGOTIATING transition is handled upstream,
/// before a [`Connection`] is created.
#[derive(Debug, PartialEq)]
pub enum State {
    Negotiating { attempts: u8 },
    Established { source: String },
}

/// Represents an active XSCP connection with a remote peer.
///
/// Holds the current [`State`] of the connection, the peer's address,
/// and a reference to the shared session store used for authentication.
///
/// # Lifecycle
///
/// While in [`State::Established`], the connection owns an entry in the
/// shared [`Sessions`] map. The entry is removed automatically on drop,
/// so the map always reflects currently-active sessions.
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
    /// Returns an [`Action`] describing what the I/O layer should do next:
    ///
    /// - [`Action::Reply`] — send the response and keep the connection open
    ///   (normal request while [`State::Negotiating`] or [`State::Established`]).
    /// - [`Action::ReplyAndClose`] — send the response and then close the
    ///   socket (e.g. `402` after exceeding authentication attempts).
    /// - [`Action::Close`] — close the socket without sending anything
    ///   (e.g. the peer issued an `EXIT` and is no longer reading).
    ///
    /// State transitions:
    /// - [`State::Negotiating`] → [`State::Established`] on successful auth.
    pub fn handle(&mut self, request: XscpRequest) -> Action {
        return match &self.state {

            State::Negotiating { attempts } => {
                let attempts = *attempts;
                let response = self.negotiate(&request, attempts);
                match response.status_code() {
                    200 => {
                        println!("{} - Logged in successfully", self.peer_addr);
                        self.state = State::Established { source: request.source().to_string() };
                        Action::Reply(response)
                    }
                    401 => {
                        println!("{} - Invalid Credentials", self.peer_addr);
                        self.state = State::Negotiating { attempts: attempts + 1 };
                        Action::Reply(response)
                    }
                    402 => {
                        // Connection will be closed after this, future state doesn't matter
                        println!("{} - Exceeded auth attempts", self.peer_addr);
                        Action::ReplyAndClose(response)
                    }
                    400 | _ => {
                        println!("{} - Invalid request", self.peer_addr);
                        self.state = State::Negotiating { attempts: attempts + 1 };
                        Action::Reply(response)
                    }
                }
            },

            State::Established { source: _ } => {
                match request.opcode() {
                    xscp::OpCode::Send => todo!(),
                    xscp::OpCode::Exit => Action::Close, // Connection will be closed after this, future state doesn't matter
                    _                  => {
                        let response = XscpResponse::try_new(400, "Invalid Request").unwrap();
                        Action::Reply(response)
                    },
                }
            },
        };
    }

    fn negotiate(&self, request: &XscpRequest, attempts: u8) -> XscpResponse<'static> {
        match request.opcode() {
            xscp::OpCode::Login => auth(request.source().to_string(), self.peer_addr, attempts, &self.sessions),
            _                   => XscpResponse::try_new(400, "Invalid Request").unwrap(),
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

/// Describes what [`run_connection`] should do after [`Connection::handle`]
/// processes a request.
///
/// Returned by [`Connection::handle`] so the I/O layer can decide whether to
/// write a response, close the socket, or both, without inspecting the
/// connection's internal [`State`].
///
/// [`run_connection`]: crate::run_connection
pub enum Action {
    /// Send the response to the peer and keep the connection open.
    Reply(XscpResponse<'static>),
    /// Send the response to the peer and then close the connection.
    ///
    /// Used when the protocol requires a final reply before terminating —
    /// e.g. a `402` after exceeding authentication attempts.
    ReplyAndClose(XscpResponse<'static>),
    /// Close the connection without sending anything.
    ///
    /// Used when the peer signaled it is gone (e.g. an `EXIT` request) and
    /// no response is expected.
    Close,
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn dummy_sessions() -> Sessions {
        Arc::new(Mutex::new(HashMap::new()))
    }

    #[test]
    fn negotiating_to_established() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        let mut connection = Connection::new(peer_addr, sessions);

        assert_eq!(connection.state, State::Negotiating { attempts: 0 });

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let source = request.source().to_string();
        let response = match connection.handle(request) {
            Action::Reply(res) => res,
            _ => panic!("expected Action::Reply"),
        };

        assert_eq!(response.status_code(), 200);
        assert_eq!(connection.state, State::Established { source });
    }

    #[test]
    fn negotiating_to_negotiating() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        sessions.lock().unwrap().insert("test".to_string(), peer_addr);
        let mut connection = Connection::new(peer_addr, sessions);

        assert_eq!(connection.state, State::Negotiating { attempts: 0 });

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let response = match connection.handle(request) {
            Action::Reply(res) => res,
            _ => panic!("expected Action::Reply"),
        };

        assert_eq!(response.status_code(), 401);
        assert_eq!(connection.state, State::Negotiating { attempts: 1 });
    }

    #[test]
    fn attempts_exceeded_replies_and_closes() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        sessions.lock().unwrap().insert("test".to_string(), peer_addr);
        let mut connection = Connection::new(peer_addr, sessions);

        assert_eq!(connection.state, State::Negotiating { attempts: 0 });

        for _ in 0..2 {
            let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
            assert!(
                matches!(connection.handle(request), Action::Reply(_)),
                "expected Action::Reply while attempts remain",
            );
        }

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let response = match connection.handle(request) {
            Action::ReplyAndClose(res) => res,
            _ => panic!("expected Action::ReplyAndClose after exceeding attempts"),
        };

        assert_eq!(response.status_code(), 402);
    }

    #[test]
    fn invalid_login_request() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        sessions.lock().unwrap().insert("test".to_string(), peer_addr);
        let mut connection = Connection::new(peer_addr, sessions);

        let request = XscpRequest::try_new(xscp::OpCode::Send, "invalid", "msg").unwrap();
        let response = match connection.handle(request) {
            Action::Reply(res) => res,
            _ => panic!("expected Action::Reply"),
        };

        assert_eq!(response.status_code(), 400);
        assert_eq!(response.reason_phrase(), "Invalid Request");
        assert_eq!(connection.state, State::Negotiating { attempts: 1 });
    }

    #[test]
    fn exit_from_established_closes_and_cleans_session() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        let mut connection = Connection::new(peer_addr, Arc::clone(&sessions));

        let login = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        assert!(matches!(connection.handle(login), Action::Reply(_)));
        assert_eq!(connection.state, State::Established { source: "test".to_string() });
        assert!(sessions.lock().unwrap().contains_key("test"));

        let exit = XscpRequest::try_new(xscp::OpCode::Exit, "test", "").unwrap();
        assert!(matches!(connection.handle(exit), Action::Close));

        drop(connection);
        assert!(!sessions.lock().unwrap().contains_key("test"));
    }

    #[test]
    fn login_while_established_is_invalid_request() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        let mut connection = Connection::new(peer_addr, Arc::clone(&sessions));

        let login = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        assert!(matches!(connection.handle(login), Action::Reply(_)));
        assert_eq!(connection.state, State::Established { source: "test".to_string() });

        let relogin = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let response = match connection.handle(relogin) {
            Action::Reply(res) => res,
            _ => panic!("expected Action::Reply"),
        };

        assert_eq!(response.status_code(), 400);
        assert_eq!(response.reason_phrase(), "Invalid Request");
        assert_eq!(connection.state, State::Established { source: "test".to_string() });
        assert!(sessions.lock().unwrap().contains_key("test"));
    }
}