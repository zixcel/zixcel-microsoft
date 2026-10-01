use super::DailyReportError;

pub(crate) fn date(field: &'static str, value: &str) -> Result<(), DailyReportError> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 4 | 7) && !byte.is_ascii_digit())
    {
        return Err(invalid_date(field));
    }
    let year = decimal(&bytes[..4]);
    let month = decimal(&bytes[5..7]);
    let day = decimal(&bytes[8..10]);
    let Some(limit) = days_in_month(year, month) else {
        return Err(invalid_date(field));
    };
    if day == 0 || day > limit {
        return Err(invalid_date(field));
    }
    Ok(())
}

pub(crate) fn date_time(field: &'static str, value: &str) -> Result<(), DailyReportError> {
    if value.len() < 20 || value.as_bytes().get(10) != Some(&b'T') {
        return Err(invalid_date_time(field));
    }
    date(field, &value[..10])?;
    let suffix = &value[11..];
    let (time, zone) = if let Some(time) = suffix.strip_suffix('Z') {
        (time, "Z")
    } else {
        let Some(index) = suffix.rfind(['+', '-']) else {
            return Err(invalid_date_time(field));
        };
        (&suffix[..index], &suffix[index..])
    };
    let (whole, fraction) = time
        .split_once('.')
        .map_or((time, None), |(whole, fraction)| (whole, Some(fraction)));
    let bytes = whole.as_bytes();
    if bytes.len() != 8
        || bytes[2] != b':'
        || bytes[5] != b':'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 2 | 5) && !byte.is_ascii_digit())
        || decimal(&bytes[..2]) > 23
        || decimal(&bytes[3..5]) > 59
        || decimal(&bytes[6..8]) > 59
        || fraction.is_some_and(|value| {
            value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return Err(invalid_date_time(field));
    }
    if zone != "Z" {
        let bytes = zone.as_bytes();
        if bytes.len() != 6
            || !matches!(bytes[0], b'+' | b'-')
            || bytes[3] != b':'
            || !bytes[1..3].iter().all(u8::is_ascii_digit)
            || !bytes[4..6].iter().all(u8::is_ascii_digit)
            || decimal(&bytes[1..3]) > 23
            || decimal(&bytes[4..6]) > 59
        {
            return Err(invalid_date_time(field));
        }
    }
    Ok(())
}

fn decimal(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(0, |value, byte| value * 10 + u32::from(byte - b'0'))
}

const fn days_in_month(year: u32, month: u32) -> Option<u32> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            Some(29)
        }
        2 => Some(28),
        _ => None,
    }
}

const fn invalid_date(field: &'static str) -> DailyReportError {
    DailyReportError::new(field, "must be an RFC 3339 full-date")
}

const fn invalid_date_time(field: &'static str) -> DailyReportError {
    DailyReportError::new(field, "must be an RFC 3339 date-time with a time-zone")
}

#[cfg(test)]
mod tests {
    use super::{date, date_time};

    #[test]
    fn calendar_and_time_zone_boundaries_are_checked() {
        assert!(date("date", "2024-02-29").is_ok());
        assert!(date("date", "2026-02-29").is_err());
        assert!(date_time("time", "2026-07-24T17:00:00+09:00").is_ok());
        assert!(date_time("time", "2026-07-24T24:00:00Z").is_err());
    }
}
