use super::ReadError;
use crate::Error;

pub(super) enum Phase {
    Subject,
    Tenant,
    Domain,
}

/// Preserve failure class and phase without exposing headers, tokens or response bodies.
pub(super) fn failure(phase: Phase, error: ReadError) -> Error {
    let (subject, tenant, domain) = match error {
        ReadError::Http(401) => (
            "subject_authentication_rejected",
            "tenant_authentication_rejected",
            "domain_authentication_rejected",
        ),
        ReadError::Http(403) => (
            "subject_permission_denied",
            "tenant_permission_denied",
            "domain_permission_denied",
        ),
        ReadError::Http(404) => (
            "subject_not_found_or_hidden",
            "tenant_not_found_or_hidden",
            "domain_not_found_or_hidden",
        ),
        ReadError::Http(429) => (
            "subject_rate_limited",
            "tenant_rate_limited",
            "domain_rate_limited",
        ),
        ReadError::Http(500..=599) => (
            "subject_remote_unavailable",
            "tenant_remote_unavailable",
            "domain_remote_unavailable",
        ),
        ReadError::Http(_) => (
            "subject_http_rejected",
            "tenant_http_rejected",
            "domain_http_rejected",
        ),
        ReadError::Budget => (
            "subject_read_limit",
            "tenant_read_limit",
            "domain_read_limit",
        ),
        ReadError::Invalid => (
            "subject_response_invalid",
            "tenant_response_invalid",
            "domain_response_invalid",
        ),
        ReadError::Transport => (
            "subject_transport_failed",
            "tenant_transport_failed",
            "domain_transport_failed",
        ),
    };
    Error(match phase {
        Phase::Subject => subject,
        Phase::Tenant => tenant,
        Phase::Domain => domain,
    })
}
