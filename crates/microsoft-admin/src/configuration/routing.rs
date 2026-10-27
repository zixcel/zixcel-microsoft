use super::read::text;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub name: String,
    pub priority: u16,
    #[serde(default)]
    pub from: Vec<String>,
    #[serde(default)]
    pub subject_contains: Vec<String>,
    #[serde(default)]
    pub body_contains: Vec<String>,
    #[serde(default)]
    pub subject_or_body_contains: Vec<String>,
    pub delivery: String,
    pub recipients: Vec<String>,
    pub keep_original: bool,
    pub stop_processing: bool,
    #[serde(default)]
    pub allow_external: bool,
}
impl Rule {
    pub fn validate(&self) -> Result<()> {
        text(&self.name, 100)?;
        if !self.name.starts_with("zix:")
            || self.priority > 99
            || !["redirect", "forward"].contains(&self.delivery.as_str())
            || self.recipients.is_empty()
            || self.recipients.len() > 8
        {
            return Err(Error("invalid_routing_rule"));
        }
        for list in [
            &self.from,
            &self.subject_contains,
            &self.body_contains,
            &self.subject_or_body_contains,
        ] {
            if list.len() > 16 {
                return Err(Error("rule_condition_limit"));
            }
            for item in list {
                text(item, 256)?;
            }
        }
        if self.from.is_empty()
            && self.subject_contains.is_empty()
            && self.body_contains.is_empty()
            && self.subject_or_body_contains.is_empty()
        {
            return Err(Error("unconditional_route_refused"));
        }
        for a in self.from.iter().chain(&self.recipients) {
            crate::email(a)?;
        }
        if self
            .recipients
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != self.recipients.len()
        {
            return Err(Error("duplicate_route_destination"));
        }
        Ok(())
    }
}
