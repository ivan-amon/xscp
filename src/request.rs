//! XSCP Request PDU parsing and representation.
//!
//! This module defines the logic to parse incoming XSCP request PDUs.

/// An XSCP request PDU.
///
/// # Wire Format
///
/// ```text
/// +------------------------------------------------------------------+
/// |   OPCODE (4 Bytes)   |   Nickname (Min 3 Bytes, Max 32 Bytes)    |
/// |------------------------------------------------------------------|
/// |          Message (Max 472 Bytes) + \r\n (2 Bytes)                |
/// +------------------------------------------------------------------+
/// ```
///
/// Fields are delimited by `|`. Both `Nickname` and `Message` are UTF-8 encoded.
/// The total PDU size must not exceed **512 bytes** (delimiters included).
///
/// Note: `|` and `\r\n` characters are disallowed in `Nickname`. `Message` disallows `\r\n`
/// to prevent message smuggling attacks. `Message` may contain `|` characters.
#[derive(Debug)]
pub struct XscpRequest<'a> {
    opcode: OpCode,
    nickname: &'a str,
    message: &'a str,
}

impl<'a> XscpRequest<'a> {
    /// Creates a new XSCP request.
    ///
    /// This method protects against 'message smuggling' attacks by validating the input parameters and ensuring that the
    /// the nickname and message do not contain disallowed characters. The nickname rejects `|` and `\r\n`; the message
    /// rejects only `\r\n` (pipes are allowed in the message).
    ///
    /// # Errors
    /// - `InvalidNickname`: The nickname contains disallowed characters or is of invalid length.
    /// - `InvalidMessage`: The message contains disallowed characters or is of invalid length.
    pub fn try_new(opcode: OpCode, nickname: &'a str, message: &'a str) -> Result<Self, RequestError> {
        if nickname.contains(['|', '\r', '\n']) || nickname.len() < 3 || nickname.len() > 32 {
            return Err(RequestError::InvalidNickname);
        }

        if message.contains(['\r', '\n']) || message.len() > 472 {
            return Err(RequestError::InvalidMessage);
        }

        Ok(Self { opcode, nickname, message })
    }

    /// Parses a raw request string into an `XscpRequest` struct.
    /// 
    /// # Errors
    /// - `UnknownOpcode`: The opcode is not recognized.
    /// - `MalformedRequest`: The request does not conform to the expected format.
    /// - `InvalidNickname`: The nickname is shorter than 3 bytes or longer than 32 bytes.
    /// - `InvalidMessage`: The message is longer than 472 bytes.
    /// - `MissingCrlf`: The request does not end with `\r\n`.
    pub fn parse(raw_request: &'a str) -> Result<Self, RequestError> {
        if !raw_request.ends_with("\r\n") {
            return Err(RequestError::MissingCrlf);
        }

        let raw_request = raw_request.trim_end_matches("\r\n");
        let mut parts = raw_request.splitn(3, '|');

        let opcode = parts.next().ok_or(RequestError::MalformedRequest)?;
        let nickname = parts.next().ok_or(RequestError::MalformedRequest)?;
        let message = parts.next().ok_or(RequestError::MalformedRequest)?;

        let opcode = match opcode {
            "LOGN" => OpCode::Login,
            "CHAT" => OpCode::Chat,
            "EXIT" => OpCode::Exit,
            _ => return Err(RequestError::UnknownOpcode),
        };

        Self::try_new(opcode, nickname, message)
    }

    /// Returns the opcode of the request.
    pub fn opcode(&self) -> OpCode {
        self.opcode
    }

    /// Returns the nickname of the request.
    pub fn nickname(&self) -> &str {
        self.nickname
    }

    /// Returns the message of the request.
    pub fn message(&self) -> &str {
        self.message
    }
}

/// Possible OPCODEs in XSCP requests.
/// 
/// Wire Format Reference:
/// - Login: `LOGN`
/// - Chat:  `CHAT`
/// - Exit:  `EXIT`
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OpCode {
    /// User registration.
    Login,
    /// Global message broadcast.
    Chat,
    /// Graceful disconnection.
    Exit,
}

/// Possible errors when creating or parsing an XSCP request.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RequestError {
    UnknownOpcode,
    MalformedRequest,
    InvalidNickname,
    InvalidMessage,
    MissingCrlf,
}

#[cfg(test)]
mod tests {

    use super::*;

    // Creation tests
    #[test]
    fn correct_request() {
        let request = XscpRequest::try_new(OpCode::Chat, "nickname", "message").unwrap();
        assert_eq!(OpCode::Chat, request.opcode());
        assert_eq!("nickname", request.nickname());
        assert_eq!("message", request.message());
    }

    #[test]
    fn nickname_with_pipe() {
        let request = XscpRequest::try_new(OpCode::Chat, "nick|name", "message").unwrap_err();
        assert_eq!(RequestError::InvalidNickname, request);
    }

    #[test]
    fn nickname_with_crlf() {
        let request = XscpRequest::try_new(OpCode::Chat, "nick\r\nname", "message").unwrap_err();
        assert_eq!(RequestError::InvalidNickname, request);
    }

    #[test]
    fn nickname_empty() {
        let err = XscpRequest::try_new(OpCode::Chat, "", "message").unwrap_err();
        assert_eq!(RequestError::InvalidNickname, err);
    }

    #[test]
    fn nickname_below_min() {
        let err = XscpRequest::try_new(OpCode::Chat, "ab", "message").unwrap_err();
        assert_eq!(RequestError::InvalidNickname, err);
    }

    #[test]
    fn nickname_above_max() {
        let nickname = "a".repeat(33);
        let err = XscpRequest::try_new(OpCode::Chat, &nickname, "message").unwrap_err();
        assert_eq!(RequestError::InvalidNickname, err);
    }

    #[test]
    fn message_with_crlf() {
        let request = XscpRequest::try_new(OpCode::Chat, "nickname", "message with \r\n (CRLF)").unwrap_err();
        assert_eq!(RequestError::InvalidMessage, request);
    }

    #[test]
    fn message_with_pipe() {
        let request = XscpRequest::try_new(OpCode::Chat, "nickname", "message with | (pipe)").unwrap();
        assert_eq!(OpCode::Chat, request.opcode());
        assert_eq!("nickname", request.nickname());
        assert_eq!("message with | (pipe)", request.message());
    }

    #[test]
    fn message_above_max() {
        let message = "a".repeat(473);
        let err = XscpRequest::try_new(OpCode::Chat, "nickname", &message).unwrap_err();
        assert_eq!(RequestError::InvalidMessage, err);
    }

    // Parsing tests
    #[test]
    fn correct_parsing() {
        let raw_request = "CHAT|nickname|message\r\n";
        let request = XscpRequest::parse(raw_request).unwrap();
        assert_eq!(OpCode::Chat, request.opcode());
        assert_eq!("nickname", request.nickname());
        assert_eq!("message", request.message());
    }

    #[test]
    fn invalid_opcode() {
        let raw_request = "AAAA|nickname|message\r\n";
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
        let raw_request = "CHAT|nickname|message";
        let error = XscpRequest::parse(raw_request).unwrap_err();
        assert_eq!(RequestError::MissingCrlf, error);
    }

    #[test]
    fn parse_message_with_pipe() {
        let raw_request = "CHAT|nickname|message with | (pipe)\r\n";
        let request = XscpRequest::parse(raw_request).unwrap();
        assert_eq!(OpCode::Chat, request.opcode());
        assert_eq!("nickname", request.nickname());
        assert_eq!("message with | (pipe)", request.message());
    }
}
