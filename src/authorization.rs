//! Provider planning is accepted only with an intact Constillo workflow receipt.

mod model;

pub use model::{AuthorizedConnectorPlan, ProviderPlanRequest, WorkflowAuthorization};
use model::{PLAN_SCHEMA, REQUEST_SCHEMA};

use crate::{ConnectorConfig, ConnectorError, PROVIDER, boundary::sha256_digest, build_plan};
use sha2::{Digest, Sha256};

pub fn parse_provider_plan_request(source: &str) -> Result<ProviderPlanRequest, ConnectorError> {
    if source.len() > 1_048_576 {
        return Err(error("request exceeds 1 MiB"));
    }
    serde_json::from_str(source).map_err(|_| error("request is not a closed v1 document"))
}

pub fn build_authorized_plan(
    config: &ConnectorConfig,
    request: &ProviderPlanRequest,
) -> Result<AuthorizedConnectorPlan, ConnectorError> {
    validate_request(request)?;
    let plan = build_plan(config)?;
    let request_bytes =
        serde_json::to_vec(request).map_err(|_| error("request encoding failed"))?;
    Ok(AuthorizedConnectorPlan {
        schema: PLAN_SCHEMA,
        request_id: request.request_id.clone(),
        request_digest_sha256: digest(&request_bytes),
        provider_plan_id: plan.plan_id,
        provider: PROVIDER,
        mode: "propose",
        workflow_receipt_id: request.workflow_authorization.receipt_id.clone(),
        workflow_receipt_digest_sha256: request
            .workflow_authorization
            .receipt_digest_sha256
            .clone(),
        decision_id: request.workflow_authorization.decision_id.clone(),
        decision_digest_sha256: request
            .workflow_authorization
            .decision_digest_sha256
            .clone(),
        network_calls_performed: false,
        credentials_resolved: false,
        secret_resolution: false,
        external_actions: false,
    })
}

fn validate_request(value: &ProviderPlanRequest) -> Result<(), ConnectorError> {
    let receipt = &value.workflow_authorization;
    let material = serde_json::json!({
        "workflow_id": receipt.workflow_id,
        "workflow_version": receipt.workflow_version,
        "workflow_plan_digest_sha256": receipt.workflow_plan_digest_sha256,
        "decision_id": receipt.decision_id,
        "decision_digest_sha256": receipt.decision_digest_sha256,
        "official_hat_id": receipt.official_hat_id,
        "hat_manifest_digest_sha256": receipt.hat_manifest_digest_sha256,
        "recommended_option_id": receipt.recommended_option_id,
        "executable": false,
        "external_actions": false
    });
    let expected =
        digest(&serde_json::to_vec(&material).map_err(|_| error("receipt encoding failed"))?);
    let valid = value.schema == REQUEST_SCHEMA
        && valid_id(&value.request_id)
        && value.expected_provider == PROVIDER
        && !value.network_access
        && !value.secret_resolution
        && !value.external_actions
        && receipt.schema == "constillo.workflow-authorization-receipt.v1"
        && receipt.receipt_digest_sha256 == expected
        && receipt.receipt_id == format!("workflow-receipt-{}", &expected[7..31])
        && sha256_digest(&receipt.decision_digest_sha256)
        && receipt.decision_id == format!("decision-{}", &receipt.decision_digest_sha256[7..31])
        && [
            &receipt.workflow_id,
            &receipt.workflow_version,
            &receipt.decision_id,
            &receipt.official_hat_id,
            &receipt.recommended_option_id,
        ]
        .iter()
        .all(|value| valid_id(value))
        && sha256_digest(&receipt.workflow_plan_digest_sha256)
        && sha256_digest(&receipt.hat_manifest_digest_sha256)
        && !receipt.executable
        && !receipt.external_actions;
    valid
        .then_some(())
        .ok_or_else(|| error("workflow authorization failed integrity validation"))
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'/' | b'-' | b'_'))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn error(message: &'static str) -> ConnectorError {
    ConnectorError::new("provider_plan_request", message)
}
