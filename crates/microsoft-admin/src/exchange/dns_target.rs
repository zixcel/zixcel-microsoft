//! Observed DNS record targets may contain underscore labels. Tenant domain
//! inputs continue to use the separate, stricter host-domain contract.
pub(super) fn valid(value: &str) -> bool {
    value.len() <= 253
        && value.contains('.')
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_')
                })
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn microsoft_legacy_and_current_dkim_targets_preserve_underscore_labels() {
        for target in [
            "selector1-example-net._domainkey.initial.onmicrosoft.com",
            "selector2-example-net._domainkey.initial.a-v1.dkim.mail.microsoft",
        ] {
            assert!(valid(target));
        }
    }
    #[test]
    fn urls_empty_labels_bad_bytes_and_excessive_names_are_rejected() {
        for target in [
            "https://example.net",
            "a..example.net",
            "-a.example.net",
            "a-.example.net",
            "a.example.net.",
            "a\n.example.net",
            "A.example.net",
            "*.example.net",
        ] {
            assert!(!valid(target), "{target}");
        }
        assert!(!valid(&format!("{}.example.net", "a".repeat(64))));
        assert!(!valid(&format!("{}.net", "a.".repeat(126))));
    }
}
