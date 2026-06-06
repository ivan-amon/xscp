//! Terminal input-prompt helpers for the chat CLI.
//!
//! Keeps a `You > ` prompt visible while waiting for the user to type, and lets
//! asynchronous server messages be printed without colliding with it.

use std::io::Write;

/// The text shown while waiting for the user to type a message.
const PROMPT: &str = "You > ";

/// Prints the input prompt and flushes so it shows up before any input arrives.
pub fn print_prompt() {
    print!("{PROMPT}");
    let _ = std::io::stdout().flush();
}

/// Erases the current prompt line (carriage return + clear-to-end-of-line) so an
/// asynchronous message from the server can be printed without colliding with it.
pub fn clear_prompt_line() {
    print!("\r\x1b[K");
    let _ = std::io::stdout().flush();
}
