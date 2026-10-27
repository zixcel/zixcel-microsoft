//! Portable provider protocol only: no network client, credential store or admin operations.
mod address;
mod application;
mod mail;
mod token;
pub use address::mailbox;
pub use application::{Application, Request};
pub use mail::PlainMail;
pub use token::{AccessToken, MAX_TOKEN_RESPONSE, parse_token};
pub type Result<T> = std::result::Result<T, &'static str>;
