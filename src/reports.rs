use serde::Serialize;

use crate::{CONNECTOR, ConnectorConfig, ConnectorError, PROVIDER};

const CAPABILITIES: &[&str] = &[
    "onedrive-observe",
    "outlook-calendar-observe",
    "outlook-mail-observe",
    "sharepoint-department-daily-report-offline-accept",
    "sharepoint-department-daily-report-plan",
    "sharepoint-observe",
    "teams-observe",
];

/// Evidence that generic planning cannot cause an external action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorReport {
    pub schema: &'static str,
    pub connector: &'static str,
    pub status: &'static str,
    pub network_client_linked: bool,
    pub secret_resolution_enabled: bool,
    pub execution_enabled: bool,
    pub network_calls_performed: bool,
    pub redirects_followed: bool,
    pub credentials_resolved: bool,
}

/// Capabilities advertised to a separate orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityReport {
    pub schema: &'static str,
    pub connector: &'static str,
    pub provider: &'static str,
    pub planning_only: bool,
    pub capabilities: Vec<&'static str>,
}

/// Positive generic configuration validation receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationReport {
    pub schema: &'static str,
    pub connector: &'static str,
    pub config_id: String,
    pub valid: bool,
}

/// Reports deliberately unavailable effects and dependencies.
#[must_use]
pub const fn doctor() -> DoctorReport {
    DoctorReport {
        schema: "zixcel://doctor/v1",
        connector: CONNECTOR,
        status: "healthy",
        network_client_linked: false,
        secret_resolution_enabled: false,
        execution_enabled: false,
        network_calls_performed: false,
        redirects_followed: false,
        credentials_resolved: false,
    }
}

/// Returns planning and offline-accept capabilities.
#[must_use]
pub fn capabilities() -> CapabilityReport {
    CapabilityReport {
        schema: "zixcel://capabilities/v1",
        connector: CONNECTOR,
        provider: PROVIDER,
        planning_only: true,
        capabilities: CAPABILITIES.to_vec(),
    }
}

/// Revalidates a typed configuration and emits a receipt.
pub fn validation_report(config: &ConnectorConfig) -> Result<ValidationReport, ConnectorError> {
    config.validate()?;
    Ok(ValidationReport {
        schema: "zixcel://validation-result/v1",
        connector: CONNECTOR,
        config_id: config.config_id.clone(),
        valid: true,
    })
}
