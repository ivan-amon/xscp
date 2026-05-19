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
use crate::session::{auth, storage::Sessions};
use std::net::SocketAddr;
use xscp::{XscpNotification, XscpRequest, XscpResponse};

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
/// shared [`Sessions`] set. The entry is removed automatically on drop,
/// so the set always reflects currently-active sessions.
pub struct Connection {
    peer_addr: SocketAddr,
    state: State,
    sessions: Sessions,
}

impl Connection {
    /// Creates a new [`Connection`] in the [`State::Negotiating`] state.
    ///
    /// # Arguments
    /// - `peer_addr` — the remote socket address of the connecting client.
    /// - `sessions` — shared session store used to validate credentials during login.
    pub fn new(peer_addr: SocketAddr, sessions: Sessions) -> Self {
        Self {
            peer_addr,
            state: State::Negotiating { attempts: 0 },
            sessions,
        }
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
    /// - [`Action::Broadcast`] — publish the envelope to all active
    ///   connection writers via the broadcast channel.
    /// State transitions:
    /// - [`State::Negotiating`] → [`State::Established`] on successful auth.
    pub async fn handle(&mut self, request: XscpRequest<'_>) -> Action {
        return match &self.state {
            State::Negotiating { attempts } => {
                let attempts = *attempts;
                let response = self.negotiate(&request, attempts);
                match response.status_code() {
                    200 => {
                        println!("{} - Logged in successfully", self.peer_addr);
                        self.state = State::Established {
                            source: request.source().to_string(),
                        };
                        Action::Reply(response)
                    }
                    401 => {
                        println!("{} - Invalid Credentials", self.peer_addr);
                        self.state = State::Negotiating {
                            attempts: attempts + 1,
                        };
                        Action::Reply(response)
                    }
                    402 => {
                        // Connection will be closed after this, future state doesn't matter
                        println!("{} - Exceeded auth attempts", self.peer_addr);
                        Action::ReplyAndClose(response)
                    }
                    400 | _ => {
                        println!("{} - Invalid request", self.peer_addr);
                        self.state = State::Negotiating {
                            attempts: attempts + 1,
                        };
                        Action::Reply(response)
                    }
                }
            }

            State::Established { source } => {
                match request.opcode() {
                    xscp::OpCode::Send => {
                        println!(
                            "{} ({}) sent: {:?}",
                            self.peer_addr,
                            source,
                            request.message()
                        );
                        let notification = XscpNotification::try_new(
                            xscp::NotificationType::Broadcast,
                            source,
                            request.message(),
                        )
                        .unwrap();

                        Action::Broadcast(BroadcastEnvelope {
                            from: source.to_string(),
                            payload: notification.to_string(),
                        })
                    }
                    xscp::OpCode::Exit => Action::Close, // Connection will be closed after this, future state doesn't matter
                    _ => {
                        let response = XscpResponse::try_new(400, "Invalid Request").unwrap();
                        Action::Reply(response)
                    }
                }
            }
        };
    }

    /// Returns the authenticated source name if the connection is in [`State::Established`].
    ///
    /// Returns `None` if the connection is still negotiating or has no registered source.
    pub fn source(&self) -> Option<&str> {
        match &self.state {
            State::Established { source } => Some(source),
            _ => None,
        }
    }

    /// Processes a request while in [`State::Negotiating`].
    ///
    /// Only [`xscp::OpCode::Login`] requests are valid during negotiation.
    /// Other opcodes return a `400 Bad Request` response.
    fn negotiate(&self, request: &XscpRequest, attempts: u8) -> XscpResponse<'static> {
        match request.opcode() {
            xscp::OpCode::Login => auth(request.source().to_string(), attempts, &self.sessions),
            _ => XscpResponse::try_new(400, "Invalid Request").unwrap(),
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

/// Encapsulates a broadcast message to be delivered to all connected clients.
///
/// Created when a client in [`State::Established`] sends a message via [`xscp::OpCode::Send`].
/// The payload is a serialized [`XscpNotification`] with type [`xscp::NotificationType::Broadcast`].
///
/// [`XscpNotification`]: xscp::XscpNotification
#[derive(Clone, Debug)]
pub struct BroadcastEnvelope {
    pub from: String,
    pub payload: String,
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
    /// Deliver a broadcast message to all connected clients.
    ///
    /// Occurs when a client in [`State::Established`] sends a message.
    /// The I/O layer should forward the [`BroadcastEnvelope`] to all
    /// active connection writers.
    Broadcast(BroadcastEnvelope),
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    fn dummy_sessions() -> Sessions {
        Arc::new(Mutex::new(HashSet::new()))
    }

    #[tokio::test]
    async fn negotiating_to_established() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        let mut connection = Connection::new(peer_addr, sessions);

        assert_eq!(connection.state, State::Negotiating { attempts: 0 });

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let source = request.source().to_string();
        let response = match connection.handle(request).await {
            Action::Reply(res) => res,
            _ => panic!("expected Action::Reply"),
        };

        assert_eq!(response.status_code(), 200);
        assert_eq!(connection.state, State::Established { source });
    }

    #[tokio::test]
    async fn negotiating_to_negotiating() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        sessions.lock().unwrap().insert("test".to_string());
        let mut connection = Connection::new(peer_addr, sessions);

        assert_eq!(connection.state, State::Negotiating { attempts: 0 });

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let response = match connection.handle(request).await {
            Action::Reply(res) => res,
            _ => panic!("expected Action::Reply"),
        };

        assert_eq!(response.status_code(), 401);
        assert_eq!(connection.state, State::Negotiating { attempts: 1 });
    }

    #[tokio::test]
    async fn attempts_exceeded_replies_and_closes() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        sessions.lock().unwrap().insert("test".to_string());
        let mut connection = Connection::new(peer_addr, sessions);

        assert_eq!(connection.state, State::Negotiating { attempts: 0 });

        for _ in 0..2 {
            let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
            assert!(
                matches!(connection.handle(request).await, Action::Reply(_)),
                "expected Action::Reply while attempts remain",
            );
        }

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let response = match connection.handle(request).await {
            Action::ReplyAndClose(res) => res,
            _ => panic!("expected Action::ReplyAndClose after exceeding attempts"),
        };

        assert_eq!(response.status_code(), 402);
    }

    #[tokio::test]
    async fn invalid_login_request() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        sessions.lock().unwrap().insert("test".to_string());
        let mut connection = Connection::new(peer_addr, sessions);

        let request = XscpRequest::try_new(xscp::OpCode::Send, "invalid", "msg").unwrap();
        let response = match connection.handle(request).await {
            Action::Reply(res) => res,
            _ => panic!("expected Action::Reply"),
        };

        assert_eq!(response.status_code(), 400);
        assert_eq!(response.reason_phrase(), "Invalid Request");
        assert_eq!(connection.state, State::Negotiating { attempts: 1 });
    }

    #[tokio::test]
    async fn exit_from_established_closes_and_cleans_session() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        let mut connection = Connection::new(peer_addr, Arc::clone(&sessions));

        let login = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        assert!(matches!(connection.handle(login).await, Action::Reply(_)));
        assert_eq!(
            connection.state,
            State::Established {
                source: "test".to_string()
            }
        );
        assert!(sessions.lock().unwrap().contains("test"));

        let exit = XscpRequest::try_new(xscp::OpCode::Exit, "test", "").unwrap();
        assert!(matches!(connection.handle(exit).await, Action::Close));

        drop(connection);
        assert!(!sessions.lock().unwrap().contains("test"));
    }

    #[tokio::test]
    async fn login_while_established_is_invalid_request() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        let mut connection = Connection::new(peer_addr, Arc::clone(&sessions));

        let login = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        assert!(matches!(connection.handle(login).await, Action::Reply(_)));
        assert_eq!(
            connection.state,
            State::Established {
                source: "test".to_string()
            }
        );

        let relogin = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let response = match connection.handle(relogin).await {
            Action::Reply(res) => res,
            _ => panic!("expected Action::Reply"),
        };

        assert_eq!(response.status_code(), 400);
        assert_eq!(response.reason_phrase(), "Invalid Request");
        assert_eq!(
            connection.state,
            State::Established {
                source: "test".to_string()
            }
        );
        assert!(sessions.lock().unwrap().contains("test"));
    }

    #[tokio::test]
    async fn send_from_established_returns_broadcast_action() {
        let peer_addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
        let sessions = dummy_sessions();
        let mut connection = Connection::new(peer_addr, sessions);

        let login = XscpRequest::try_new(xscp::OpCode::Login, "Bob", "").unwrap();
        let _ = connection.handle(login).await;

        assert_eq!(
            connection.state,
            State::Established {
                source: "Bob".to_string()
            }
        );

        let request = XscpRequest::try_new(xscp::OpCode::Send, "Bob", "Hello World").unwrap();
        match connection.handle(request).await {
            Action::Broadcast(envelope) => {
                assert_eq!(envelope.from, "Bob");
                assert_eq!(envelope.payload, "BRDC|Bob|Hello World\r\n");
            }
            _ => panic!("expected Action::Broadcast"),
        }
    }
}
