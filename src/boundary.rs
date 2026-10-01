use std::fmt;

/// Structured validation failure for generic Microsoft planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorError {
    pub field: &'static str,
    pub message: &'static str,
}

impl ConnectorError {
    pub(crate) const fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }
}

impl fmt::Display for ConnectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ConnectorError {}

pub(crate) fn identifier(field: &'static str, value: &str) -> Result<(), ConnectorError> {
    if value.is_empty()
        || value.len() > 96
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
    {
        return Err(ConnectorError::new(
            field,
            "must be a lowercase ASCII identifier",
        ));
    }
    Ok(())
}

pub(crate) fn reference(field: &'static str, value: &str) -> Result<(), ConnectorError> {
    if value.trim().is_empty()
        || value.len() > 160
        || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(ConnectorError::new(
            field,
            "must be a non-empty opaque reference",
        ));
    }
    Ok(())
}

pub(crate) fn secret_ref(value: &str) -> Result<(), ConnectorError> {
    let Some(path) = value.strip_prefix("secret://") else {
        return Err(ConnectorError::new(
            "secret_ref",
            "must use a secret:// reference",
        ));
    };
    if path.len() < 3
        || path.len() > 240
        || !path.contains('/')
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains("//")
        || path.contains(['?', '#'])
        || path.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(ConnectorError::new(
            "secret_ref",
            "contains an invalid reference path",
        ));
    }
    Ok(())
}

pub(crate) fn reject_secrets(source: &str) -> Result<(), ConnectorError> {
    if source.len() > 1_048_576 {
        return Err(ConnectorError::new("config", "configuration exceeds 1 MiB"));
    }
    let value: toml::Value = toml::from_str(source)
        .map_err(|_| ConnectorError::new("config", "configuration is not valid TOML"))?;
    reject_value(&value, 0)
}

pub(crate) fn sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn reject_value(value: &toml::Value, depth: usize) -> Result<(), ConnectorError> {
    if depth > 32 {
        return Err(ConnectorError::new(
            "config",
            "configuration nesting exceeds 32 levels",
        ));
    }
    match value {
        toml::Value::Table(table) => {
            for (key, nested) in table {
                let key: String = key
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .flat_map(char::to_lowercase)
                    .collect();
                if matches!(
                    key.as_str(),
                    "password"
                        | "token"
                        | "accesstoken"
                        | "refreshtoken"
                        | "clientsecret"
                        | "privatekey"
                        | "apikey"
                        | "credential"
                        | "credentials"
                        | "secret"
                ) {
                    return Err(ConnectorError::new(
                        "config",
                        "embedded credential fields are forbidden; use secret_ref",
                    ));
                }
                reject_value(nested, depth + 1)?;
            }
        }
        toml::Value::Array(items) => {
            for item in items {
                reject_value(item, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}
