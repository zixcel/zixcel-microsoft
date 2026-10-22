use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Read {
    Organization,
    Licenses,
    Domains,
    Users,
    User { user: String },
    DomainDns { domain: String },
}
