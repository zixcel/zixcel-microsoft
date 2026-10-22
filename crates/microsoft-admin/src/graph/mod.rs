mod continuation;
mod identity;
mod identity_failure;
mod observe;
mod projection;
mod read;
mod read_failure;
mod resource;
mod transport;
pub use observe::observe;
pub use read::Read;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use transport::{LiveRead, ReadError, Transport};
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub state: String,
    pub reason: String,
    pub data: Value,
    pub pages: usize,
    pub complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
}
impl Observation {
    fn failed(reason: &str) -> Self {
        Self {
            state: "unobserved".into(),
            reason: reason.into(),
            data: Value::Null,
            pages: 0,
            complete: false,
            http_status: None,
        }
    }
}
