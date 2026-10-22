use crowsi_provider_read_http::{Error, HttpsRead};
use serde_json::Value;
use std::time::Duration;
#[derive(Debug, Clone, Copy)]
pub enum ReadError {
    Http(u16),
    Budget,
    Invalid,
    Transport,
}
pub trait Transport {
    fn get(&mut self, path: &str) -> Result<Value, ReadError>;
}
pub struct LiveRead {
    http: HttpsRead,
    remaining_bytes: usize,
}
impl LiveRead {
    pub fn new(token: &str, timeout: Duration) -> Result<Self, ReadError> {
        Ok(Self {
            http: HttpsRead::new("https://graph.microsoft.com", token, timeout, 8)
                .map_err(convert)?,
            remaining_bytes: 2 * 1024 * 1024,
        })
    }
}
fn convert(e: Error) -> ReadError {
    match e {
        Error::Status(s) => ReadError::Http(s),
        Error::Budget | Error::BodyLimit => ReadError::Budget,
        _ => ReadError::Transport,
    }
}
impl Transport for LiveRead {
    fn get(&mut self, path: &str) -> Result<Value, ReadError> {
        if !path.starts_with("/v1.0/")
            || path.contains(['#', '\\', '\r', '\n'])
            || path.contains("..")
        {
            return Err(ReadError::Invalid);
        }
        let limit = self.remaining_bytes.min(512 * 1024);
        if limit == 0 {
            return Err(ReadError::Budget);
        }
        let b = self
            .http
            .get(&format!("https://graph.microsoft.com{path}"), &[], limit)
            .map_err(convert)?;
        self.remaining_bytes -= b.len();
        serde_json::from_slice(&b).map_err(|_| ReadError::Invalid)
    }
}
