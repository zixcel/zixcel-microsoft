//! Conservative ASCII mailbox profile shared by mail producers.
pub fn mailbox(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    if s.len() > 254 || local.is_empty() || local.len() > 64 || !domain.contains('.') {
        return false;
    }
    let atom = |b: u8| b.is_ascii_alphanumeric() || b"!#$%&'*+/=?^_`{|}~-".contains(&b);
    if !local
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(atom))
    {
        return false;
    }
    let labels: Vec<_> = domain.split('.').collect();
    let tld = labels.last().unwrap_or(&"");
    tld.len() >= 2
        && tld.len() <= 63
        && tld.bytes().all(|b| b.is_ascii_alphabetic())
        && labels.iter().all(|p| {
            !p.is_empty()
                && p.len() <= 63
                && p.as_bytes()[0].is_ascii_alphanumeric()
                && p.as_bytes()[p.len() - 1].is_ascii_alphanumeric()
                && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}
