//! Fixed-origin, bounded HTTPS without redirects, proxies or automatic retries.
use crate::{Error, Result};
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    io::Read,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
pub struct Http {
    client: Client,
    origin: &'static str,
    deadline: Instant,
    remaining: usize,
}
pub struct Response {
    pub status: u16,
    pub bytes: Zeroizing<Vec<u8>>,
}
impl Http {
    pub fn new(origin: &'static str, timeout: Duration) -> Result<Self> {
        if ![
            "https://graph.microsoft.com",
            "https://login.microsoftonline.com",
        ]
        .contains(&origin)
        {
            return Err(Error("invalid_origin"));
        }
        Ok(Self {
            client: Client::builder()
                .https_only(true)
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .connect_timeout(Duration::from_secs(5))
                .build()
                .map_err(|_| Error("transport_setup_failed"))?,
            origin,
            deadline: Instant::now() + timeout,
            remaining: 16,
        })
    }
    fn send(
        &mut self,
        method: &str,
        path: &str,
        token: Option<&str>,
        json: Option<&Value>,
        form: Option<&[(&str, &str)]>,
    ) -> Result<Response> {
        if self.remaining == 0 {
            return Err(Error("request_budget"));
        }
        if !path.starts_with('/')
            || path.starts_with("//")
            || path.contains(['#', '\\', '\r', '\n'])
            || path.contains("..")
            || path.len() > 4096
        {
            return Err(Error("invalid_request_path"));
        }
        self.remaining -= 1;
        let mut r = self
            .client
            .request(
                method.parse().map_err(|_| Error("invalid_method"))?,
                format!("{}{path}", self.origin),
            )
            .timeout(
                self.deadline
                    .checked_duration_since(Instant::now())
                    .ok_or(Error("request_deadline"))?,
            );
        if let Some(t) = token {
            r = r.bearer_auth(t);
        }
        if let Some(v) = json {
            r = r.json(v);
        }
        if let Some(v) = form {
            r = r.form(v);
        }
        let r = r.send().map_err(|_| Error("transport_failed"))?;
        let status = r.status().as_u16();
        if r.content_length().is_some_and(|n| n > 1_048_576) {
            return Err(Error("response_limit"));
        }
        let mut bytes = Zeroizing::new(Vec::new());
        r.take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|_| Error("response_unavailable"))?;
        if bytes.len() > 1_048_576 {
            return Err(Error("response_limit"));
        }
        Ok(Response { status, bytes })
    }
    pub fn graph(
        &mut self,
        method: &str,
        path: &str,
        token: &str,
        body: Option<&Value>,
    ) -> Result<Response> {
        if self.origin != "https://graph.microsoft.com"
            || !path.starts_with("/v1.0/")
            || !["GET", "POST", "PATCH"].contains(&method)
        {
            return Err(Error("invalid_graph_route"));
        }
        self.send(method, path, Some(token), body, None)
    }
    pub fn oauth(
        &mut self,
        tenant: &str,
        endpoint: &str,
        form: &[(&str, &str)],
    ) -> Result<Response> {
        crate::uuid(tenant)?;
        if self.origin != "https://login.microsoftonline.com"
            || !["devicecode", "token"].contains(&endpoint)
        {
            return Err(Error("invalid_oauth_route"));
        }
        self.send(
            "POST",
            &format!("/{tenant}/oauth2/v2.0/{endpoint}"),
            None,
            None,
            Some(form),
        )
    }
}
impl Response {
    pub fn json(&self) -> Result<Value> {
        serde_json::from_slice(&self.bytes).map_err(|_| Error("response_decode_failed"))
    }
    pub fn require_success(&self) -> Result<()> {
        match self.status {
            200..=299 => Ok(()),
            401 => Err(Error("authentication_rejected")),
            403 => Err(Error("permission_denied")),
            429 => Err(Error("rate_limited_no_retry")),
            _ => Err(Error("request_rejected")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_origin_methods_and_oauth_tenant_fail_before_network() {
        assert!(Http::new("https://other.example", Duration::from_secs(1)).is_err());
        let mut g = Http::new("https://graph.microsoft.com", Duration::from_secs(1)).unwrap();
        assert!(
            g.graph("DELETE", "/v1.0/users/id", "synthetic", None)
                .is_err()
        );
        assert!(
            g.graph("GET", "//other.example/v1.0/", "synthetic", None)
                .is_err()
        );
        assert!(g.graph("GET", "/v1.0/../users", "synthetic", None).is_err());
        assert!(g.oauth("common", "token", &[]).is_err());
        let r = Response {
            status: 429,
            bytes: Zeroizing::new(Vec::new()),
        };
        assert_eq!(r.require_success().unwrap_err().0, "rate_limited_no_retry");
    }
}
