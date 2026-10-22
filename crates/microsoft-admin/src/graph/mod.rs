mod continuation;
mod observe;
mod projection;
mod resource;
mod transport;
pub use observe::{Read, observe};
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
}
impl Observation {
    fn failed(reason: &str) -> Self {
        Self {
            state: "unobserved".into(),
            reason: reason.into(),
            data: Value::Null,
            pages: 0,
            complete: false,
        }
    }
}
