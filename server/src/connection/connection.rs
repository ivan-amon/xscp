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
pub enum State {
    Negotiating { attempts: u8 },
    Established { source: String },
    Aborted,
}

/// Represents an active XSCP connection with a remote peer.
///
/// Holds the current [`State`] of the connection, the peer's address,
/// and a reference to the shared session store used for authentication.
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
                let response = negotiate(request, attempts, self.sessions.clone());
                if response.status_code() == 401 { 
                    println!("Invalid Credentials from {}", self.peer_addr);
                    self.state = State::Negotiating { attempts: *attempts + 1 }
                };
                response
            },

            State::Established { source: _ } => todo!(),

            State::Aborted => todo!(),
        };
        response
    }
}

fn negotiate(request: XscpRequest, attempts: &u8, session: Sessions) -> XscpResponse<'static> {
    let response = match request.opcode() {
        xscp::OpCode::Login => auth(request, *attempts, session.clone()),
        _                   => XscpResponse::try_new(400, "INVALID REQUEST").unwrap(),
    };
    response
}