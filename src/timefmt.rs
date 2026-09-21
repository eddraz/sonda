//! Minimal RFC3339 UTC timestamps from `SystemTime` (std-only, no chrono).

use std::time::{SystemTime, UNIX_EPOCH};

/// Current UTC time formatted as RFC3339, e.g. `2026-02-14T10:00:00Z`.
pub fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_epoch(secs)
}

/// Format seconds since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_epoch(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 to (y, m, d).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + i64::from(m <= 2), m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_epoch_zero() {
        assert_eq!(format_epoch(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn formats_known_timestamp() {
        assert_eq!(format_epoch(1_000_000_000), "2001-09-09T01:46:40Z");
    }

    #[test]
    fn handles_leap_day() {
        assert_eq!(format_epoch(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn now_is_rfc3339_shaped() {
        let now = now_rfc3339();
        assert_eq!(now.len(), 20);
        assert!(now.ends_with('Z'));
        assert_eq!(&now[4..5], "-");
        assert_eq!(&now[10..11], "T");
    }
}
