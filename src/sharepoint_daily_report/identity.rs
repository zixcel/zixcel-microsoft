use std::collections::BTreeSet;

use super::DailyReportError;

pub(crate) fn identifier(field: &'static str, value: &str) -> Result<(), DailyReportError> {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err(DailyReportError::new(
            field,
            "must be a lowercase ASCII identifier",
        ));
    };
    if value.len() > 128
        || !(first.is_ascii_lowercase() || first.is_ascii_digit())
        || !bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
    {
        return Err(DailyReportError::new(
            field,
            "must be a lowercase ASCII identifier",
        ));
    }
    Ok(())
}

pub(crate) fn identifier_list(
    field: &'static str,
    values: &[String],
    non_empty: bool,
) -> Result<(), DailyReportError> {
    if values.len() > 1_024 {
        return Err(DailyReportError::new(
            field,
            "must contain at most 1024 identifiers",
        ));
    }
    if non_empty && values.is_empty() {
        return Err(DailyReportError::new(
            field,
            "must contain at least one identifier",
        ));
    }
    let mut unique = BTreeSet::new();
    for value in values {
        identifier(field, value)?;
        if !unique.insert(value) {
            return Err(DailyReportError::new(
                field,
                "must contain unique identifiers",
            ));
        }
    }
    Ok(())
}

pub(crate) fn opaque_reference(
    field: &'static str,
    value: &str,
    prefix: &'static str,
) -> Result<(), DailyReportError> {
    let Some(identifier_value) = value.strip_prefix(prefix) else {
        return Err(DailyReportError::new(
            field,
            "must use its opaque reference prefix",
        ));
    };
    if value.len() > 160 {
        return Err(DailyReportError::new(field, "opaque reference is too long"));
    }
    identifier(field, identifier_value)
}

pub(crate) fn contract_reference(field: &'static str, value: &str) -> Result<(), DailyReportError> {
    let Some((scheme, identifier_value)) = value.split_once("://") else {
        return Err(DailyReportError::new(
            field,
            "must be a bounded versioned contract reference",
        ));
    };
    let scheme_valid = (2..=32).contains(&scheme.len())
        && scheme
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && scheme.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.')
        });
    let reference_valid = !identifier_value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'?' | b'#' | b'\\'));
    if scheme_valid && reference_valid {
        Ok(())
    } else {
        Err(DailyReportError::new(
            field,
            "must be a bounded versioned contract reference",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{contract_reference, identifier};

    #[test]
    fn identifier_rejects_empty_uppercase_and_overlong_values() {
        assert!(identifier("id", "").is_err());
        assert!(identifier("id", "Upper").is_err());
        assert!(identifier("id", &"x".repeat(129)).is_err());
    }

    #[test]
    fn contract_reference_rejects_network_parameters_and_invalid_schemes() {
        assert!(contract_reference("schema", "example://reports/daily/v1").is_ok());
        assert!(contract_reference("schema", "https://example.test/schema?token=no").is_err());
        assert!(contract_reference("schema", "Upper://reports/v1").is_err());
    }
}
