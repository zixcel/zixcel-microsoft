#![forbid(unsafe_code)]
#![doc = "Planning and offline-accept boundaries for Microsoft services."]

mod authorization;
mod boundary;
mod config;
mod oidc;
mod plan;
mod reports;

pub mod sharepoint_daily_report;
pub mod mail;

pub use authorization::{
    AuthorizedConnectorPlan, ProviderPlanRequest, WorkflowAuthorization, build_authorized_plan,
    parse_provider_plan_request,
};
pub use boundary::ConnectorError;
pub use config::{ConnectorConfig, MicrosoftCloud, MicrosoftWorkload, parse_config};
pub use oidc::{
    MicrosoftAuthenticationMethod, OidcLoginPlan, OidcLoginRequest, OidcProviderProfile,
    build_oidc_login_plan,
};
pub use plan::{ConnectorPlan, PlanStep, build_plan};
pub use reports::{
    CapabilityReport, DoctorReport, ValidationReport, capabilities, doctor, validation_report,
};

/// Stable connector identity.
pub const CONNECTOR: &str = "zixcel-microsoft";
/// Provider family represented by this connector.
pub const PROVIDER: &str = "microsoft";
/// Accepted generic configuration schema.
pub const CONFIG_SCHEMA: &str = "zixcel://microsoft/config/v1";
/// Emitted generic planning contract schema.
pub const PLAN_SCHEMA: &str = "zixcel://contracts/connector-plan/v1";
