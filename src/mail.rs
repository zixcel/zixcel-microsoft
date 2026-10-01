//! Microsoft Graph mail request mapping. No HAT, semantic inference, token custody or HTTP.
use crate::{ConnectorError, MicrosoftCloud};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub const MAIL_REQUEST_SCHEMA: &str = include_str!("../schemas/mail-request-v1.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailContent {
    Metadata,
    Body,
}

/// References are supplied by an authenticated host; never raw tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailRequest {
    pub schema: String,
    pub account_ref: String,
    pub authorization_ref: String,
    pub cloud: MicrosoftCloud,
    pub folder_id: String,
    pub content: MailContent,
    pub maximum_messages: u16,
    pub max_response_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MailPlan {
    pub schema: &'static str,
    pub plan_id: String,
    pub account_ref: String,
    pub authorization_ref: String,
    pub method: &'static str,
    pub url: String,
    pub prefer: &'static str,
    pub required_delegated_permission: &'static str,
    pub max_response_bytes: u32,
    pub follow_redirects: bool,
    pub network_calls_performed: bool,
}

pub fn parse_mail_request(source: &str) -> Result<MailRequest, ConnectorError> {
    if source.len() > 16_384 {
        return Err(ConnectorError::new("request", "request exceeds 16 KiB"));
    }
    let request: MailRequest = serde_json::from_str(source)
        .map_err(|_| ConnectorError::new("request", "invalid closed mail request"))?;
    validate(&request)?;
    Ok(request)
}

fn validate(value: &MailRequest) -> Result<(), ConnectorError> {
    if value.schema != "zixcel://microsoft/mail/request/v1" {
        return Err(ConnectorError::new(
            "schema",
            "unexpected mail request schema",
        ));
    }
    for (field, value, maximum) in [
        ("account_ref", value.account_ref.as_str(), 160),
        ("authorization_ref", value.authorization_ref.as_str(), 160),
        ("folder_id", value.folder_id.as_str(), 512),
    ] {
        if value.is_empty()
            || value.len() > maximum
            || value.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(ConnectorError::new(field, "invalid bounded reference"));
        }
    }
    if matches!(value.folder_id.as_str(), "." | "..") {
        return Err(ConnectorError::new(
            "folder_id",
            "relative path segments are not mailbox identifiers",
        ));
    }
    if !(1..=100).contains(&value.maximum_messages)
        || !(1..=1_048_576).contains(&value.max_response_bytes)
    {
        return Err(ConnectorError::new(
            "limits",
            "mail page exceeds allowed resource limits",
        ));
    }
    Ok(())
}

/// First-page read proposal only; does not obtain tokens, follow nextLink, mark
/// messages as read, send mail or interpret message content. The transport owner
/// must verify account/cloud/scope and bound response bytes before accumulation.
pub fn build_mail_plan(value: &MailRequest) -> Result<MailPlan, ConnectorError> {
    validate(value)?;
    let host = match value.cloud {
        MicrosoftCloud::Public => "graph.microsoft.com",
        MicrosoftCloud::UsGovernment => "graph.microsoft.us",
    };
    let mut folder = String::new();
    for byte in value.folder_id.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            folder.push(char::from(byte));
        } else {
            write!(folder, "%{byte:02X}")
                .map_err(|_| ConnectorError::new("folder_id", "could not encode folder"))?;
        }
    }
    let body = matches!(value.content, MailContent::Body);
    let fields = "id,changeKey,subject,from,receivedDateTime,sentDateTime,hasAttachments";
    let url = format!(
        "https://{host}/v1.0/me/mailFolders/{folder}/messages?$top={}&$select={fields}{}",
        value.maximum_messages,
        if body { ",body" } else { "" }
    );
    let bytes = serde_json::to_vec(value)
        .map_err(|_| ConnectorError::new("request", "serialization failed"))?;
    Ok(MailPlan {
        schema: "zixcel://microsoft/mail/plan/v1",
        plan_id: format!("{:x}", Sha256::digest(bytes)),
        account_ref: value.account_ref.clone(),
        authorization_ref: value.authorization_ref.clone(),
        method: "GET",
        url,
        prefer: "IdType=\"ImmutableId\", outlook.body-content-type=\"text\"",
        required_delegated_permission: if body { "Mail.Read" } else { "Mail.ReadBasic" },
        max_response_bytes: value.max_response_bytes,
        follow_redirects: false,
        network_calls_performed: false,
    })
}
