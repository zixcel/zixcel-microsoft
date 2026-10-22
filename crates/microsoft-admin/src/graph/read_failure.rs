use super::ReadError;

/// Preserve protocol failure classes without provider error messages or headers.
pub(super) fn classify(error: ReadError) -> (&'static str, Option<u16>) {
    let reason = match error {
        ReadError::Http(400) => "request_rejected",
        ReadError::Http(401) => "authentication_rejected",
        ReadError::Http(403) => "permission_denied",
        ReadError::Http(404) => "not_found_or_hidden",
        ReadError::Http(429) => "rate_limited",
        ReadError::Http(500..=599) => "remote_unavailable",
        ReadError::Http(_) => "http_rejected",
        ReadError::Budget => "request_or_byte_limit",
        ReadError::Invalid => "response_invalid",
        ReadError::Transport => "transport_failed",
    };
    let status = match error {
        ReadError::Http(status) => Some(status),
        _ => None,
    };
    (reason, status)
}
