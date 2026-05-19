//! XSCP Request PDU parsing and representation.
//!
//! This module defines the logic to parse incoming XSCP request PDUs.

use std::fmt;

/// An XSCP request PDU.
///
/// # Wire Format
///
/// ```text
/// +------------------------------------------------------------------+
/// |   OPCODE (4 Bytes)   |    Source (Min 3 Bytes, Max 32 Bytes)     |
/// |------------------------------------------------------------------|
/// |          Message (Max 472 Bytes) + \r\n (2 Bytes)                |
/// +------------------------------------------------------------------+
/// ```
///
/// Fields are delimited by `|`. Both `Source` and `Message` are UTF-8 encoded.
/// The total PDU size must not exceed **512 bytes** (delimiters included).
///
/// Note: `|` and `\r\n` characters are disallowed in `Source`. `Message` disallows `\r\n`
/// to prevent message smuggling attacks. `Message` may contain `|` characters.
#[derive(Debug)]
pub struct XscpRequest<'a> {
    opcode: OpCode,
    source: &'a str,
    message: &'a str,
}

impl<'a> XscpRequest<'a> {
    /// Creates a new XSCP request.
    ///
    /// This method protects against 'message smuggling' attacks by validating the input parameters and ensuring that the
    /// the source and message do not contain disallowed characters. The source rejects `|` and `\r\n`; the message
    /// rejects only `\r\n` (pipes are allowed in the message).
    ///
    /// # Errors
    /// - `InvalidSource`: The source contains disallowed characters or is of invalid length.
    /// - `InvalidMessage`: The message contains disallowed characters or is of invalid length.
    pub fn try_new(
        opcode: OpCode,
        source: &'a str,
        message: &'a str,
    ) -> Result<Self, RequestError> {
        if source.contains(['|', '\r', '\n']) || source.len() < 3 || source.len() > 32 {
            return Err(RequestError::InvalidSource);
        }

        if message.contains(['\r', '\n']) || message.len() > 472 {
            return Err(RequestError::InvalidMessage);
        }

        Ok(Self {
            opcode,
            source,
            message,
        })
    }

    /// Parses a raw request string into an `XscpRequest` struct.
    ///
    /// # Errors
    /// - `UnknownOpcode`: The opcode is not recognized.
    /// - `MalformedRequest`: The request does not conform to the expected format.
    /// - `InvalidSource`: The source is shorter than 3 bytes or longer than 32 bytes.
    /// - `InvalidMessage`: The message is longer than 472 bytes.
    /// - `MissingCrlf`: The request does not end with `\r\n`.
    pub fn parse(raw_request: &'a str) -> Result<Self, RequestError> {
        if !raw_request.ends_with("\r\n") {
            return Err(RequestError::MissingCrlf);
        }

        let raw_request = raw_request.trim_end_matches("\r\n");
        let mut parts = raw_request.splitn(3, '|');

        let opcode = parts.next().ok_or(RequestError::MalformedRequest)?;
        let source = parts.next().ok_or(RequestError::MalformedRequest)?;
        let message = parts.next().ok_or(RequestError::MalformedRequest)?;

        let opcode = match opcode {
            "LOGN" => OpCode::Login,
            "SEND" => OpCode::Send,
            "EXIT" => OpCode::Exit,
            _ => return Err(RequestError::UnknownOpcode),
        };

        Self::try_new(opcode, source, message)
    }

    /// Returns the opcode of the request.
    pub fn opcode(&self) -> OpCode {
        self.opcode
    }

    /// Returns the source of the request.
    pub fn source(&self) -> &str {
        self.source
    }

    /// Returns the message of the request.
    pub fn message(&self) -> &str {
        self.message
    }
}

impl<'a> fmt::Display for XscpRequest<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}|{}|{}\r\n", self.opcode, self.source, self.message)
    }
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            OpCode::Login => "LOGN",
            OpCode::Send => "SEND",
            OpCode::Exit => "EXIT",
        };
        write!(f, "{}", s)
    }
}

/// Possible OPCODEs in XSCP requests.
///
/// Wire Format Reference:
/// - Login: `LOGN`
/// - Send:  `SEND`
/// - Exit:  `EXIT`
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OpCode {
    /// User registration.
    Login,
    /// Global message broadcast.
    Send,
    /// Graceful disconnection.
    Exit,
}

/// Possible errors when creating or parsing an XSCP request.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RequestError {
    UnknownOpcode,
    MalformedRequest,
    InvalidSource,
    InvalidMessage,
    MissingCrlf,
}

#[cfg(test)]
mod tests {

    use super::*;

    // Creation tests
    #[test]
    fn correct_request() {
        let request = XscpRequest::try_new(OpCode::Send, "source", "message").unwrap();
        assert_eq!(OpCode::Send, request.opcode());
        assert_eq!("source", request.source());
        assert_eq!("message", request.message());
    }

    #[test]
    fn source_with_pipe() {
        let request = XscpRequest::try_new(OpCode::Send, "source|name", "message").unwrap_err();
        assert_eq!(RequestError::InvalidSource, request);
    }

    #[test]
    fn source_with_crlf() {
        let request = XscpRequest::try_new(OpCode::Send, "source\r\nname", "message").unwrap_err();
        assert_eq!(RequestError::InvalidSource, request);
    }

    #[test]
    fn source_empty() {
        let err = XscpRequest::try_new(OpCode::Send, "", "message").unwrap_err();
        assert_eq!(RequestError::InvalidSource, err);
    }

    #[test]
    fn source_below_min() {
        let err = XscpRequest::try_new(OpCode::Send, "ab", "message").unwrap_err();
        assert_eq!(RequestError::InvalidSource, err);
    }

    #[test]
    fn source_above_max() {
        let source = "a".repeat(33);
        let err = XscpRequest::try_new(OpCode::Send, &source, "message").unwrap_err();
        assert_eq!(RequestError::InvalidSource, err);
    }

    #[test]
    fn message_with_crlf() {
        let request =
            XscpRequest::try_new(OpCode::Send, "source", "message with \r\n (CRLF)").unwrap_err();
        assert_eq!(RequestError::InvalidMessage, request);
    }

    #[test]
    fn message_with_pipe() {
        let request =
            XscpRequest::try_new(OpCode::Send, "source", "message with | (pipe)").unwrap();
        assert_eq!(OpCode::Send, request.opcode());
        assert_eq!("source", request.source());
        assert_eq!("message with | (pipe)", request.message());
    }

    #[test]
    fn message_above_max() {
        let message = "a".repeat(473);
        let err = XscpRequest::try_new(OpCode::Send, "source", &message).unwrap_err();
        assert_eq!(RequestError::InvalidMessage, err);
    }

    // Parsing tests
    #[test]
    fn correct_parsing() {
        let raw_request = "SEND|source|message\r\n";
        let request = XscpRequest::parse(raw_request).unwrap();
        assert_eq!(OpCode::Send, request.opcode());
        assert_eq!("source", request.source());
        assert_eq!("message", request.message());
    }

    #[test]
    fn invalid_opcode() {
        let raw_request = "AAAA|source|message\r\n";
        let error = XscpRequest::parse(raw_request).unwrap_err();
        assert_eq!(RequestError::UnknownOpcode, error);
    }

    #[test]
    fn invalid_format() {
        let raw_request = "vfw9f8i9v\r\n";
        let error = XscpRequest::parse(raw_request).unwrap_err();
        assert_eq!(RequestError::MalformedRequest, error);
    }

    #[test]
    fn missing_crlf() {
        let raw_request = "SEND|source|message";
        let error = XscpRequest::parse(raw_request).unwrap_err();
        assert_eq!(RequestError::MissingCrlf, error);
    }

    #[test]
    fn parse_message_with_pipe() {
        let raw_request = "SEND|source|message with | (pipe)\r\n";
        let request = XscpRequest::parse(raw_request).unwrap();
        assert_eq!(OpCode::Send, request.opcode());
        assert_eq!("source", request.source());
        assert_eq!("message with | (pipe)", request.message());
    }

    // Other tests
    #[test]
    fn to_string_format() {
        let request = XscpRequest::try_new(OpCode::Send, "Test", "Msg").unwrap();
        let request = request.to_string();
        assert_eq!(request, "SEND|Test|Msg\r\n".to_string());
    }
}
