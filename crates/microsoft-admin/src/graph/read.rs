use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Read {
    Organization,
    Licenses,
    Domains,
    Users,
    Groups,
    SecurityDefaults,
    ConditionalAccess,
    GroupMembers { group: String },
    UserMemberships { user: String },
    User { user: String },
    DomainDns { domain: String },
    DomainVerification { domain: String },
}
