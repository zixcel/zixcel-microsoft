use serde::{Deserialize, Serialize};

pub(super) const REQUEST_SCHEMA: &str = "zixcel://microsoft/provider-plan-request/v1";
pub(super) const PLAN_SCHEMA: &str = "zixcel://microsoft/authorized-connector-plan/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowAuthorization {
    pub schema: String,
    pub receipt_id: String,
    pub receipt_digest_sha256: String,
    pub workflow_id: String,
    pub workflow_version: String,
    pub workflow_plan_digest_sha256: String,
    pub decision_id: String,
    pub decision_digest_sha256: String,
    pub official_hat_id: String,
    pub hat_manifest_digest_sha256: String,
    pub recommended_option_id: String,
    pub executable: bool,
    pub external_actions: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderPlanRequest {
    pub schema: String,
    pub request_id: String,
    pub expected_provider: String,
    pub workflow_authorization: WorkflowAuthorization,
    pub network_access: bool,
    pub secret_resolution: bool,
    pub external_actions: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuthorizedConnectorPlan {
    pub schema: &'static str,
    pub request_id: String,
    pub request_digest_sha256: String,
    pub provider_plan_id: String,
    pub provider: &'static str,
    pub mode: &'static str,
    pub workflow_receipt_id: String,
    pub workflow_receipt_digest_sha256: String,
    pub decision_id: String,
    pub decision_digest_sha256: String,
    pub network_calls_performed: bool,
    pub credentials_resolved: bool,
    pub secret_resolution: bool,
    pub external_actions: bool,
}
