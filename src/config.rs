use serde::{Deserialize, Serialize};

use crate::boundary::{identifier, reference, reject_secrets, secret_ref};
use crate::{CONFIG_SCHEMA, ConnectorError};

/// Supported Microsoft cloud boundaries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MicrosoftCloud {
    Public,
    UsGovernment,
}

/// Workloads that a separately authorized executor may observe.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MicrosoftWorkload {
    OneDrive,
    OutlookCalendar,
    OutlookMail,
    SharePoint,
    Teams,
}

impl MicrosoftWorkload {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::OneDrive => "onedrive",
            Self::OutlookCalendar => "outlook-calendar",
            Self::OutlookMail => "outlook-mail",
            Self::SharePoint => "sharepoint",
            Self::Teams => "teams",
        }
    }
}

/// Closed generic Microsoft observation configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorConfig {
    pub schema: String,
    pub config_id: String,
    pub tenant_ref: String,
    pub secret_ref: String,
    pub cloud: MicrosoftCloud,
    pub workloads: Vec<MicrosoftWorkload>,
}

impl ConnectorConfig {
    /// Checks schema, opaque references, and the bounded workload set.
    pub fn validate(&self) -> Result<(), ConnectorError> {
        if self.schema != CONFIG_SCHEMA {
            return Err(ConnectorError::new(
                "schema",
                "expected zixcel://microsoft/config/v1",
            ));
        }
        identifier("config_id", &self.config_id)?;
        reference("tenant_ref", &self.tenant_ref)?;
        secret_ref(&self.secret_ref)?;
        if self.workloads.is_empty() || self.workloads.len() > 5 {
            return Err(ConnectorError::new(
                "workloads",
                "must contain 1..=5 workloads",
            ));
        }
        Ok(())
    }

    pub(crate) fn normalized(&self) -> Self {
        let mut value = self.clone();
        value.workloads.sort();
        value.workloads.dedup();
        value
    }
}

/// Parses bounded TOML and rejects embedded credentials.
pub fn parse_config(source: &str) -> Result<ConnectorConfig, ConnectorError> {
    reject_secrets(source)?;
    let config: ConnectorConfig = toml::from_str(source).map_err(|_| {
        ConnectorError::new(
            "config",
            "invalid TOML or fields do not match the Microsoft v1 schema",
        )
    })?;
    config.validate()?;
    Ok(config)
}
