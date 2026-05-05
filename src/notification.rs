//! XSCP Notification PDU parsing and representation.
//!
//! This module defines the logic to parse incoming XSCP notification PDUs.

/// An XSCP notification PDU.
///
/// # Wire Format
///
/// ```text
/// +---------------------------------------------------------------------------+
/// |   Notification Type (4 Bytes)   |   Source (Min 3 Bytes, Max 32 Bytes)    |
/// |---------------------------------------------------------------------------|
/// |                 Message (Max 472 Bytes) + \r\n (2 Bytes)                  |
/// +---------------------------------------------------------------------------+
/// ```
///
/// Fields are delimited by `|`. Both `Source` and `Message` are UTF-8 encoded.
/// The total PDU size must not exceed **512 bytes** (delimiters included).
/// Source can be:
/// - A nickname of an user (for `BRDC` notifications).
/// - The string `XSCP_SERVER` (from server for notifications).
///
/// Note: `|` and `\r\n` characters are disallowed in `Source`. `Message` disallows `\r\n`
/// to prevent message smuggling attacks. `Message` may contain `|` characters.
#[derive(Debug)]
pub struct XscpNotification<'a> {
    notification_type: NotificationType,
    source: &'a str,
    message: &'a str,
}

impl<'a> XscpNotification<'a> {
    /// Creates a new XSCP notification.
    ///
    /// This method protects against 'message smuggling' attacks by validating the input parameters and ensuring that the
    /// source and message do not contain disallowed characters. The source rejects `|` and `\r\n`; the message rejects
    /// only `\r\n` (pipes are allowed in the message).
    ///
    /// # Errors
    /// - `InvalidSource`: The source contains disallowed characters or is of invalid length.
    /// - `InvalidMessage`: The message contains disallowed characters or is of invalid length.
    pub fn try_new(notification_type: NotificationType, source: &'a str, message: &'a str) -> Result<Self, NotificationError> {
        if source.contains(['|', '\r', '\n']) || source.len() < 3 || source.len() > 32 {
            return Err(NotificationError::InvalidSource);
        }

        if message.contains(['\r', '\n']) || message.len() > 472 {
            return Err(NotificationError::InvalidMessage);
        }

        Ok(Self { notification_type, source, message })
    }

    /// Parses a raw notification string into an `XscpNotification` struct.
    /// 
    /// # Errors
    /// - `UnknownNotificationType`: The notification type is not recognized.
    /// - `MalformedNotification`: The notification does not conform to the expected format.
    /// - `InvalidSource`: The source is shorter than 3 bytes or longer than 32 bytes.
    /// - `InvalidMessage`: The message is longer than 472 bytes.
    /// - `MissingCrlf`: The notification does not end with `\r\n`.
    pub fn parse(raw_notification: &'a str) -> Result<Self, NotificationError> {
        if !raw_notification.ends_with("\r\n") {
            return Err(NotificationError::MissingCrlf);
        }

        let raw_notification = raw_notification.trim_end_matches("\r\n");
        let mut parts = raw_notification.splitn(3, '|');

        let notification_type = parts.next().ok_or(NotificationError::MalformedNotification)?;
        let source = parts.next().ok_or(NotificationError::MalformedNotification)?;
        let message = parts.next().ok_or(NotificationError::MalformedNotification)?;

        let notification_type = match notification_type {
            "BRDC" => NotificationType::Broadcast,
            _ => return Err(NotificationError::UnknownNotificationType),
        };

        Self::try_new(notification_type, source, message)
    }

    /// Returns the notification type.
    pub fn notification_type(&self) -> NotificationType {
        self.notification_type
    }

    /// Returns the source of the notification.
    pub fn source(&self) -> &str {
        self.source
    }

    /// Returns the message of the notification.
    pub fn message(&self) -> &str {
        self.message
    }
}

/// Possible notification types in XSCP.
/// 
/// Wire Format Reference:
/// - Broadcast: `BRDC`
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NotificationType {
    /// Broadcast message from a user.
    Broadcast,
    // Future notification types can be added here.
}

/// Errors that can occur during notification creation and parsing.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NotificationError {
    InvalidSource,
    InvalidMessage,
    UnknownNotificationType,
    MalformedNotification,
    MissingCrlf,
}

#[cfg(test)]
mod tests {

    use super::*;

    // Creation tests
    #[test]
    fn correct_notification() {
        let notification = XscpNotification::try_new(NotificationType::Broadcast, "Alice", "Hello, World!").unwrap();
        assert_eq!(notification.notification_type(), NotificationType::Broadcast);
        assert_eq!(notification.source(), "Alice");
        assert_eq!(notification.message(), "Hello, World!");
    }

    #[test]
    fn source_with_pipe() {
        let result = XscpNotification::try_new(NotificationType::Broadcast, "Alice|Bob", "Hello!").unwrap_err();
        assert_eq!(result, NotificationError::InvalidSource);
    }

    #[test]
    fn source_with_crlf() {
        let result = XscpNotification::try_new(NotificationType::Broadcast, "Alice\r\nBob", "Hello!").unwrap_err();
        assert_eq!(result, NotificationError::InvalidSource);
    }

    #[test]
    fn source_empty() {
        let result = XscpNotification::try_new(NotificationType::Broadcast, "", "Hello!").unwrap_err();
        assert_eq!(result, NotificationError::InvalidSource);
    }

    #[test]
    fn source_below_min() {
        let result = XscpNotification::try_new(NotificationType::Broadcast, "Al", "Hello!").unwrap_err();
        assert_eq!(result, NotificationError::InvalidSource);
    }

    #[test]
    fn source_above_max() {
        let long_source = "A".repeat(33);
        let result = XscpNotification::try_new(NotificationType::Broadcast, &long_source, "Hello!").unwrap_err();
        assert_eq!(result, NotificationError::InvalidSource);
    }

    #[test]
    fn message_with_pipe() {
        let result = XscpNotification::try_new(NotificationType::Broadcast, "Bob", "Hello|World!").unwrap();
        assert_eq!(result.notification_type(), NotificationType::Broadcast);
        assert_eq!(result.source(), "Bob");
        assert_eq!(result.message(), "Hello|World!");
    }

    #[test]
    fn message_with_crlf() {
        let result = XscpNotification::try_new(NotificationType::Broadcast, "Bob", "Hello\r\nWorld!").unwrap_err();
        assert_eq!(result, NotificationError::InvalidMessage);
    }

    #[test]
    fn message_above_max() {
        let long_message = "A".repeat(473);
        let result = XscpNotification::try_new(NotificationType::Broadcast, "Bob", &long_message).unwrap_err();
        assert_eq!(result, NotificationError::InvalidMessage);
    }

    // Parsing tests
    #[test]
    fn correct_parsing() {
        let raw_notification = "BRDC|Alice|Hello, World!\r\n";
        let notification = XscpNotification::parse(raw_notification).unwrap();
        assert_eq!(notification.notification_type(), NotificationType::Broadcast);
        assert_eq!(notification.source(), "Alice");
        assert_eq!(notification.message(), "Hello, World!");
    }

    #[test]
    fn invalid_notification_type() {
        let raw_notification = "AAAA|Alice|Hello!\r\n";
        let result = XscpNotification::parse(raw_notification).unwrap_err();
        assert_eq!(result, NotificationError::UnknownNotificationType);
    }

    #[test]
    fn malformed_notification() {
        let raw_notification = "snfk4n33nkj\r\n"; // Missing delimiter
        let result = XscpNotification::parse(raw_notification).unwrap_err();
        assert_eq!(result, NotificationError::MalformedNotification);
    }

    #[test]
    fn missing_crlf() {
        let raw_notification = "BRDC|Alice|Hello, World!";
        let result = XscpNotification::parse(raw_notification).unwrap_err();
        assert_eq!(result, NotificationError::MissingCrlf);
    }

    #[test]
    fn parse_message_with_pipe() {
        let raw_notification = "BRDC|Alice|Hello|ExtraField\r\n";
        let result = XscpNotification::parse(raw_notification).unwrap();
        assert_eq!(result.notification_type(), NotificationType::Broadcast);
        assert_eq!(result.source(), "Alice");
        assert_eq!(result.message(), "Hello|ExtraField");
    }
}