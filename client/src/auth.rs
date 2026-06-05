//! This module creates an XSCP Login request and sends it to the server
use std::fmt;

use io::SocketIo;
use xscp::{RequestError, ResponseError, XscpRequest, XscpResponse};

/// Errors that can occur during authentication.
///
/// Each variant maps to a distinct stage of the login exchange that can fail,
/// either for reasons outside the protocol (network, socket, OS) or because of
/// an unexpected server response.
#[derive(Debug)]
pub enum AuthError {
    /// The login request could not be built.
    BuildRequest(RequestError),
    /// Writing the request to the socket failed.
    Write(std::io::Error),
    /// Reading the response from the socket failed.
    Read(Box<dyn std::error::Error + Send + Sync>),
    /// The server closed the connection (EOF) before responding.
    ConnectionClosed,
    /// The server response could not be parsed.
    ParseResponse(ResponseError),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::BuildRequest(e) => write!(f, "failed to build login request: {e:?}"),
            AuthError::Write(e) => write!(f, "failed to write to socket: {e}"),
            AuthError::Read(e) => write!(f, "failed to read from socket: {e}"),
            AuthError::ConnectionClosed => {
                write!(f, "server closed the connection before responding")
            }
            AuthError::ParseResponse(e) => write!(f, "invalid server response: {e:?}"),
        }
    }
}

impl std::error::Error for AuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuthError::Write(e) => Some(e),
            AuthError::Read(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

/// Sends a login request to the server and returns the received status code.
///
/// # Errors
///
/// Returns [`AuthError`] if any step of the exchange fails: building the
/// request, writing/reading on the socket, a connection closed before the
/// response, or a response that cannot be parsed.
pub async fn auth(socket_io: &mut SocketIo, source: &str) -> Result<u16, AuthError> {
    let request = XscpRequest::try_new(xscp::OpCode::Login, source, "")
        .map_err(AuthError::BuildRequest)?;

    socket_io
        .write(&request.to_string())
        .await
        .map_err(AuthError::Write)?;

    let response = socket_io
        .read()
        .await
        .map_err(AuthError::Read)?
        .ok_or(AuthError::ConnectionClosed)?;

    let response = XscpResponse::parse(&response).map_err(AuthError::ParseResponse)?;

    Ok(response.status_code())
}
