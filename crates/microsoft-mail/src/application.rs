use crate::{Result, mail::PlainMail};
use zeroize::Zeroizing;
/// A secret-bearing request deliberately has no Debug/Serialize implementation.
pub struct Request {
    pub url: String,
    pub content_type: &'static str,
    pub body: Zeroizing<String>,
}
pub struct Application {
    tenant: String,
    client: String,
    sender: String,
}
fn guid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().any(|b| b.is_ascii_hexdigit() && b != b'0')
        && s.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}
impl Application {
    pub fn new(tenant: &str, client: &str, sender: &str) -> Result<Self> {
        if ![tenant, client, sender].into_iter().all(guid) {
            return Err("invalid_graph_identity");
        }
        Ok(Self {
            tenant: tenant.to_ascii_lowercase(),
            client: client.to_ascii_lowercase(),
            sender: sender.to_ascii_lowercase(),
        })
    }
    pub fn identity(&self) -> String {
        format!("{}:{}:{}", self.tenant, self.client, self.sender)
    }
    pub fn client_secret_request(&self, secret: &str) -> Result<Request> {
        if secret.is_empty() || secret.len() > 4096 || secret.chars().any(char::is_control) {
            return Err("invalid_graph_credential");
        }
        let body = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("client_id", &self.client)
            .append_pair("client_secret", secret)
            .append_pair("scope", "https://graph.microsoft.com/.default")
            .append_pair("grant_type", "client_credentials")
            .finish();
        Ok(Request {
            url: format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
                self.tenant
            ),
            content_type: "application/x-www-form-urlencoded",
            body: Zeroizing::new(body),
        })
    }
    pub fn send_request(&self, mail: PlainMail<'_>) -> Result<Request> {
        Ok(Request {
            url: format!(
                "https://graph.microsoft.com/v1.0/users/{}/sendMail",
                self.sender
            ),
            content_type: "application/json",
            body: Zeroizing::new(mail.encode()?),
        })
    }
}
