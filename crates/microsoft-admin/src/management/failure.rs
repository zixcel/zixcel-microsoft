//! Code-only diagnostics: never serialize remote messages or arbitrary local errors.
use crate::Error;
use serde::Serialize;
use zeroize::Zeroize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailurePhase {
    Identity,
    Prepare,
    Request,
    Response,
    PostRead,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    AuthenticationRejected,
    PermissionDenied,
    RateLimited,
    RequestRejected,
    RemoteUnavailable,
    TransportFailed,
    ResponseUnavailable,
    RequestDeadline,
    PostconditionNotObserved,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ProviderCode {
    #[serde(rename = "Request_BadRequest")]
    BadRequest,
    #[serde(rename = "Authorization_RequestDenied")]
    RequestDenied,
    InvalidAuthenticationToken,
    AuthenticationError,
    #[serde(rename = "Authentication_MSGraphPermissionMissing")]
    PermissionMissing,
    #[serde(rename = "Request_ResourceNotFound")]
    ResourceNotFound,
    TooManyRequests,
    ServiceUnavailable,
    #[serde(rename = "unrecognized")]
    Unrecognized,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MutationFailure {
    pub phase: FailurePhase,
    pub reason: FailureReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_code: Option<ProviderCode>,
}

impl MutationFailure {
    pub fn from_error(phase: FailurePhase, error: Error) -> Self {
        let reason = match error.0 {
            "authentication_rejected" => FailureReason::AuthenticationRejected,
            "permission_denied" => FailureReason::PermissionDenied,
            "rate_limited_no_retry" => FailureReason::RateLimited,
            "request_rejected" => FailureReason::RequestRejected,
            "transport_failed" => FailureReason::TransportFailed,
            "response_unavailable" | "response_limit" | "response_decode_failed" => {
                FailureReason::ResponseUnavailable
            }
            "request_deadline" | "request_budget" => FailureReason::RequestDeadline,
            "postcondition_not_observed" => FailureReason::PostconditionNotObserved,
            _ => FailureReason::Unknown,
        };
        Self {
            phase,
            reason,
            http_status: None,
            provider_code: None,
        }
    }

    pub fn from_http(status: u16, body: &[u8]) -> Self {
        #[derive(serde::Deserialize)]
        struct Envelope {
            error: RemoteError,
        }
        #[derive(serde::Deserialize)]
        struct RemoteError {
            code: String,
        }
        impl Drop for RemoteError {
            fn drop(&mut self) {
                self.code.zeroize();
            }
        }
        let provider_code = if body.len() <= 1_048_576 {
            serde_json::from_slice::<Envelope>(body)
                .ok()
                .map(|v| match v.error.code.as_str() {
                    "Request_BadRequest" => ProviderCode::BadRequest,
                    "Authorization_RequestDenied" => ProviderCode::RequestDenied,
                    "InvalidAuthenticationToken" => ProviderCode::InvalidAuthenticationToken,
                    "AuthenticationError" => ProviderCode::AuthenticationError,
                    "Authentication_MSGraphPermissionMissing" => ProviderCode::PermissionMissing,
                    "Request_ResourceNotFound" => ProviderCode::ResourceNotFound,
                    "TooManyRequests" => ProviderCode::TooManyRequests,
                    "ServiceUnavailable" => ProviderCode::ServiceUnavailable,
                    _ => ProviderCode::Unrecognized,
                })
        } else {
            None
        };
        Self {
            phase: FailurePhase::Response,
            reason: match status {
                401 => FailureReason::AuthenticationRejected,
                403 => FailureReason::PermissionDenied,
                429 => FailureReason::RateLimited,
                500..=599 => FailureReason::RemoteUnavailable,
                _ => FailureReason::RequestRejected,
            },
            http_status: Some(status),
            provider_code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_and_local_secrets_never_become_diagnostics() {
        let body = br#"{"error":{"code":"untrusted-secret","message":"Bearer synthetic-secret","innerError":{"token":"synthetic-secret"}}}"#;
        let d = MutationFailure::from_http(403, body);
        assert_eq!(d.reason, FailureReason::PermissionDenied);
        assert_eq!(d.provider_code, Some(ProviderCode::Unrecognized));
        assert!(!serde_json::to_string(&d).unwrap().contains("secret"));
        let d = MutationFailure::from_error(FailurePhase::Unknown, Error("synthetic-secret"));
        assert_eq!(d.reason, FailureReason::Unknown);
        assert!(!serde_json::to_string(&d).unwrap().contains("secret"));
        for status in [400, 401, 429, 503] {
            assert_eq!(
                MutationFailure::from_http(status, b"malformed").http_status,
                Some(status)
            );
        }
    }
}
