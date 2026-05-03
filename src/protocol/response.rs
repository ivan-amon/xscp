//! XSCP Response PDU parsing and representation.
//!
//! This module defines the logic to create and parse an XSCP response.

/// An XSCP response PDU.
///
/// # Wire Format
///
/// ```text
/// +-----------------------------------------------------------------------+
/// |   Status Code (1-3 ASCII digits)   |   Reason Phrase (Max 32 Bytes)   |
/// +-----------------------------------------------------------------------+
/// ```
///
/// Fields are delimited by `|`. The total PDU size must not exceed
/// **36 bytes** (delimiter and CRLF included).
#[derive(Debug)]
pub struct XscpResponse<'a> {
    status_code: u16,
    reason_phrase: &'a str,
}

impl<'a> XscpResponse<'a> {
    /// Creates a new XSCP response.
    ///
    /// # Errors
    /// - `InvalidStatusCode`: The status code is greater than 599.
    /// - `InvalidReasonPhrase`: The reason phrase is longer than 32 bytes or contains invalid characters.
    pub fn try_new(status_code: u16, reason_phrase: &'a str) -> Result<Self, ResponseError> {
        if status_code > 599 {
            return Err(ResponseError::InvalidStatusCode);
        }

        if reason_phrase.contains(['|', '\r', '\n']) || reason_phrase.len() > 32 {
            return Err(ResponseError::InvalidReasonPhrase);
        }

        Ok(Self { status_code, reason_phrase, })
    }

    /// Parses a raw response string into an `XscpResponse`.
    ///
    /// # Errors
    /// - `MissingCrlf`: The raw response does not end with CRLF.
    /// - `MalformedResponse`: The raw response does not contain exactly one delimiter.
    /// - `InvalidStatusCode`: The status code is not a valid number or is greater than 599.
    /// - `InvalidReasonPhrase`: The reason phrase is longer than 32 bytes or contains invalid characters.
    pub fn parse(raw_response: &'a str) -> Result<Self, ResponseError> {
        if !raw_response.ends_with("\r\n") {
            return Err(ResponseError::MissingCrlf);
        }

        let raw_response = raw_response.trim_end_matches("\r\n");
        let raw_response: Vec<&str> = raw_response.split('|').collect();

        if raw_response.len() != 2 {
            return Err(ResponseError::MalformedResponse);
        }

        let status_code = match raw_response[0].parse() {
            Ok(code) => code,
            Err(_) => return Err(ResponseError::InvalidStatusCode),
        };
        let reason_phrase = raw_response[1];

        Self::try_new(status_code, reason_phrase)
    }

    /// Returns the status code.
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Returns the reason phrase.
    pub fn reason_phrase(&self) -> &'a str {
        self.reason_phrase
    }
}

/// Possible errors when creating or parsing an XSCP response.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ResponseError {
    InvalidStatusCode,
    InvalidReasonPhrase,
    MalformedResponse,
    MissingCrlf,
}

#[cfg(test)]
mod tests {

    use super::*;

    // Creation tests
    #[test]
    fn correct_response() {
        let response = XscpResponse::try_new(200, "OK").unwrap();
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.reason_phrase(), "OK");
    }

    #[test]
    fn invalid_status_code() {
        let response = XscpResponse::try_new(600, "Invalid").unwrap_err();
        assert_eq!(ResponseError::InvalidStatusCode, response);
    }

    #[test]
    fn invalid_reason_phrase_length() {
        let response =
            XscpResponse::try_new(200, "Very very long reason phrase......").unwrap_err();
        assert_eq!(ResponseError::InvalidReasonPhrase, response);
    }

    #[test]
    fn try_new_rejects_crlf_in_reason_phrase() {
        let err = XscpResponse::try_new(200, "OK\r\nhack").unwrap_err();
        assert_eq!(ResponseError::InvalidReasonPhrase, err);
    }

    #[test]
    fn try_new_rejects_pipe_in_reason_phrase() {
        let err = XscpResponse::try_new(200, "OK|extra").unwrap_err();
        assert_eq!(ResponseError::InvalidReasonPhrase, err);
    }

    // Parsing tests
    #[test]
    fn correct_parse() {
        let raw_response = "200|OK\r\n";
        let response = XscpResponse::parse(raw_response).unwrap();
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.reason_phrase(), "OK");
    }

    #[test]
    fn malformed_response() {
        let raw_response = "df43vdfjvnk\r\n";
        let response = XscpResponse::parse(raw_response).unwrap_err();
        assert_eq!(ResponseError::MalformedResponse, response);
    }

    #[test]
    fn missing_crlf() {
        let raw_response = "200|OK";
        let response = XscpResponse::parse(raw_response).unwrap_err();
        assert_eq!(ResponseError::MissingCrlf, response);
    }

    #[test]
    fn parsing_invalid_status_code() {
        let raw_response = "600|Invalid\r\n";
        let response = XscpResponse::parse(raw_response).unwrap_err();
        assert_eq!(ResponseError::InvalidStatusCode, response);
    }

    #[test]
    fn parsing_invalid_reason_phrase_length() {
        let raw_response = "200|Very very long reason phrase......\r\n";
        let response = XscpResponse::parse(raw_response).unwrap_err();
        assert_eq!(ResponseError::InvalidReasonPhrase, response);
    }

    #[test]
    fn parse_rejects_pipe_in_reason_phrase() {
        let raw_response = "200|Reason with | (pipe)\r\n";
        let response = XscpResponse::parse(raw_response).unwrap_err();
        assert_eq!(ResponseError::MalformedResponse, response);
    }
}
