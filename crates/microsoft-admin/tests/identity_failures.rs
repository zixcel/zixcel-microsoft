mod support;
use serde_json::{Value, json};
use support::*;
use zixcel_microsoft_admin::graph::{Read, ReadError, Transport, observe};

struct Failure {
    phase: usize,
    calls: usize,
    error: ReadError,
}
impl Transport for Failure {
    fn get(&mut self, _: &str) -> Result<Value, ReadError> {
        let call = self.calls;
        self.calls += 1;
        if call == self.phase {
            Err(self.error)
        } else {
            Ok(json!({"id": if call == 0 { SUBJECT } else { TENANT }}))
        }
    }
}

#[test]
fn identity_failures_preserve_phase_and_class_without_continuing_reads() {
    for (phase, prefix) in [(0, "subject"), (1, "tenant"), (2, "domain")] {
        for (error, suffix) in [
            (ReadError::Http(401), "authentication_rejected"),
            (ReadError::Http(403), "permission_denied"),
            (ReadError::Http(404), "not_found_or_hidden"),
            (ReadError::Http(429), "rate_limited"),
            (ReadError::Http(503), "remote_unavailable"),
            (ReadError::Http(302), "http_rejected"),
            (ReadError::Http(400), "http_rejected"),
            (ReadError::Budget, "read_limit"),
            (ReadError::Invalid, "response_invalid"),
            (ReadError::Transport, "transport_failed"),
        ] {
            let mut http = Failure {
                phase,
                calls: 0,
                error,
            };
            let read = Read::DomainDns {
                domain: "example.org".into(),
            };
            let failure = observe(&mut http, &scope(), read).unwrap_err();
            assert_eq!(failure.0, format!("{prefix}_{suffix}"));
            assert_eq!(http.calls, phase + 1, "no further read or automatic retry");
        }
    }
}
