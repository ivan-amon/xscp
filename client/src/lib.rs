mod auth;
mod prompt;
pub use auth::{AuthError, auth};
pub use prompt::{clear_prompt_line, print_prompt};
