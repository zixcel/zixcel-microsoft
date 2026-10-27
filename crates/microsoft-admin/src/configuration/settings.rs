use super::Read;
use crate::{Error, Result};
use serde_json::Value;
use std::collections::BTreeMap;
pub fn validate_patch(
    target: &Read,
    identity: Option<&str>,
    values: &BTreeMap<String, Value>,
) -> Result<()> {
    if values.is_empty() || values.len() > 12 {
        return Err(Error("invalid_settings_count"));
    }
    let fields: &[&str] = match target {
        Read::Mailbox { .. } => &[
            "MessageCopyForSentAsEnabled",
            "MessageCopyForSendOnBehalfEnabled",
            "HiddenFromAddressListsEnabled",
            "RetentionPolicy",
            "SharingPolicy",
            "AddressBookPolicy",
        ],
        Read::ClientAccess { .. } => &[
            "OWAEnabled",
            "MAPIEnabled",
            "ActiveSyncEnabled",
            "PopEnabled",
            "ImapEnabled",
            "SmtpClientAuthenticationDisabled",
            "OwaMailboxPolicy",
            "ActiveSyncMailboxPolicy",
        ],
        Read::UserPolicy { .. } => &["AuthenticationPolicy"],
        Read::Organization { .. } => &["SmtpClientAuthenticationDisabled"],
        Read::RemoteDomains { .. } => &["AutoForwardEnabled"],
        Read::Policies { kind } => match kind.as_str() {
            "mobile" => &[
                "PasswordEnabled",
                "MinPasswordLength",
                "DeviceEncryptionEnabled",
            ],
            "authentication" => &[
                "AllowBasicAuthPop",
                "AllowBasicAuthImap",
                "AllowBasicAuthSmtp",
            ],
            "outbound_spam" => &["AutoForwardingMode"],
            _ => return Err(Error("policy_write_not_supported")),
        },
        _ => return Err(Error("configuration_write_not_supported")),
    };
    if matches!(target, Read::Policies { .. } | Read::RemoteDomains { .. }) {
        let id = identity.ok_or(Error("configuration_identity_required"))?;
        super::read::text(id, 256)?;
        if id.contains(['*', '?', '[', ']']) {
            return Err(Error("wildcard_identity_refused"));
        }
    } else if identity.is_some() {
        return Err(Error("unexpected_configuration_identity"));
    }
    for (k, v) in values {
        if !fields.contains(&k.as_str()) {
            return Err(Error("unknown_configuration_property"));
        }
        let valid = if k.ends_with("Policy") {
            v.is_null()
                || v.as_str().is_some_and(|s| {
                    super::read::text(s, 256).is_ok() && !s.contains(['*', '?', '[', ']'])
                })
        } else if k == "MinPasswordLength" {
            v.as_u64().is_some_and(|n| (4..=16).contains(&n))
        } else if k == "AutoForwardingMode" {
            v.as_str()
                .is_some_and(|s| ["Off", "On", "Automatic"].contains(&s))
        } else {
            v.is_boolean()
                || (k == "SmtpClientAuthenticationDisabled"
                    && matches!(target, Read::ClientAccess { .. })
                    && v.is_null())
        };
        if !valid {
            return Err(Error("invalid_configuration_value"));
        }
    }
    Ok(())
}
pub(super) fn target_row<'a>(b: &'a Value, id: Option<&str>) -> Result<&'a Value> {
    if let Some(id) = id {
        let rows = b["data"]
            .as_array()
            .ok_or(Error("configuration_collection_required"))?;
        let matches: Vec<_> = rows
            .iter()
            .filter(|r| r["Identity"] == id || r["Name"] == id)
            .collect();
        if matches.len() != 1 {
            return Err(Error("configuration_identity_ambiguous"));
        }
        Ok(matches[0])
    } else {
        Ok(&b["data"])
    }
}
