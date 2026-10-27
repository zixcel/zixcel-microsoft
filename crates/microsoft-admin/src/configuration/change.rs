use super::{Read, Rule};
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    Configure {
        target: Read,
        identity: Option<String>,
        values: BTreeMap<String, Value>,
    },
    CreateInboxRule {
        mailbox: String,
        rule: Rule,
        acknowledge_outlook_rules_blob_removal: bool,
    },
    UpdateInboxRule {
        mailbox: String,
        identity: String,
        rule: Rule,
        acknowledge_outlook_rules_blob_removal: bool,
    },
    DeleteInboxRule {
        mailbox: String,
        identity: String,
        acknowledge_outlook_rules_blob_removal: bool,
    },
    SetInboxRuleEnabled {
        mailbox: String,
        identity: String,
        enabled: bool,
        #[serde(default)]
        allow_external: bool,
        acknowledge_outlook_rules_blob_removal: bool,
    },
    SetAutoMapping {
        mailbox: String,
        trustee: String,
        enabled: bool,
    },
}
impl Change {
    pub fn read(&self) -> Read {
        match self {
            Self::Configure { target, .. } => target.clone(),
            Self::CreateInboxRule { mailbox, .. }
            | Self::UpdateInboxRule { mailbox, .. }
            | Self::DeleteInboxRule { mailbox, .. }
            | Self::SetInboxRuleEnabled { mailbox, .. } => Read::InboxRules {
                mailbox: mailbox.clone(),
            },
            Self::SetAutoMapping {
                mailbox, trustee, ..
            } => Read::AutoMapping {
                mailbox: mailbox.clone(),
                trustee: trustee.clone(),
            },
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.read().validate()?;
        match self {
            Self::Configure {
                target,
                identity,
                values,
            } => super::rules::validate_patch(target, identity.as_deref(), values),
            Self::CreateInboxRule { rule, .. } => rule.validate(),
            Self::UpdateInboxRule { identity, rule, .. } => {
                super::read::text(identity, 256)?;
                rule.validate()
            }
            Self::DeleteInboxRule { identity, .. } | Self::SetInboxRuleEnabled { identity, .. } => {
                super::read::text(identity, 256)
            }
            Self::SetAutoMapping { .. } => Ok(()),
        }
    }
}
