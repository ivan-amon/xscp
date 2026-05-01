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
/// Note: currently `|` cannot be send on messages (the server will return an error).
#[derive(Debug)]
pub struct XscpRequest<'a> {
    opcode: OpCode,
    nickname: &'a str,
    message: &'a str,
}

impl<'a> XscpRequest<'a> {
    pub fn parse(raw_request: &'a str) -> Result<Self, RequestError> {

        if !raw_request.ends_with("\r\n") { 
            return Err(RequestError::MissingCrlf); 
        }

        let raw_request = raw_request.trim_end_matches("\r\n");
        let raw_request: Vec<&str> = raw_request.split('|').collect();

        if raw_request.len() != 3 {
            return Err(RequestError::MalformedRequest);
        }

        // Opcode: must be 4 bytes and should exist
        let opcode = raw_request[0];
        if opcode.len() != 4 {
            return Err(RequestError::UnknownOpcode);
        }
        let opcode = match opcode {
            "LOGN" => OpCode::Login,
            "CHAT" => OpCode::Chat,
            "EXIT" => OpCode::Exit,
                _  => return  Err(RequestError::UnknownOpcode),
        };

        // Nickname: must be between 3 and 32 bytes
        let nickname = raw_request[1];
        if nickname.len() < 3 || nickname.len() > 32 {
            return Err(RequestError::InvalidNicknameLength);
        }

        // Message: must occupy no more than 472 bytes 
        let message = raw_request[2];
        if message.len() > 472 {
            return Err(RequestError::InvalidMessageLength);
        }

        Ok(Self { opcode, nickname, message })
    }

    pub fn opcode(&self) -> OpCode { self.opcode }
    pub fn nickname(&self) -> &str { self.nickname }
    pub fn message(&self) -> &str { self.message }
}

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
    Exit
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RequestError {
    UnknownOpcode,
    MalformedRequest,
    InvalidNicknameLength,
    InvalidMessageLength,
    MissingCrlf,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn correct_parsing() {
        let raw_request = "CHAT|nickname|message\r\n";
        let request = XscpRequest::parse(&raw_request).unwrap();
        assert_eq!(OpCode::Chat, request.opcode());
        assert_eq!("nickname", request.nickname());
        assert_eq!("message", request.message());
    }

    #[test]
    fn invalid_opcode() {
        let raw_request = "AAAA|nickname|message\r\n";
        let error = XscpRequest::parse(&raw_request).unwrap_err();
        assert_eq!(RequestError::UnknownOpcode, error);
    }

    #[test]
    fn invalid_format() {
        let raw_request = "vfw9f8i9v\r\n";
        let error = XscpRequest::parse(&raw_request).unwrap_err();
        assert_eq!(RequestError::MalformedRequest, error);
    }

    #[test]
    fn invalid_length() {
        let raw_request = "CHAT|nickname_with_invalid_length_test|message\r\n";
        let error = XscpRequest::parse(&raw_request).unwrap_err();
        assert_eq!(RequestError::InvalidNicknameLength, error);
    }

    #[test]
    fn missing_crlf() {
        let raw_request = "CHAT|nickname|message";
        let error = XscpRequest::parse(&raw_request).unwrap_err();
        assert_eq!(RequestError::MissingCrlf, error);
    }

    #[test]
    fn pipe_in_message() {
        let raw_request = "CHAT|nickname|message with | (pipe)\r\n";
        let error = XscpRequest::parse(&raw_request).unwrap_err();
        assert_eq!(RequestError::MalformedRequest, error);
    }
}