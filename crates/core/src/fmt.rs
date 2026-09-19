//! Small display helpers shared by the UI (no locale dependencies).

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// (year, month 1-12, day) of a unix timestamp, UTC. Howard Hinnant's civil-from-days.
pub fn ymd(unix_seconds: u64) -> (i64, u32, u32) {
    let z = (unix_seconds / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

/// "Sep 18, 2026"; empty for 0.
pub fn format_date(unix_seconds: u64) -> String {
    if unix_seconds == 0 {
        return String::new();
    }
    let (y, m, d) = ymd(unix_seconds);
    format!("{} {}, {}", MONTHS[(m - 1) as usize], d, y)
}

/// Unix seconds of midnight UTC on y-m-d (inverse of `ymd`).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> u64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (days.max(0) as u64) * 86_400
}

pub fn format_bytes(n: u64) -> String {
    if n < 1024 {
        return format!("{n} B");
    }
    let units = ["KB", "MB", "GB", "TB"];
    let mut v = n as f64 / 1024.0;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if v < 10.0 {
        format!("{v:.1} {}", units[i])
    } else {
        format!("{} {}", v.round() as u64, units[i])
    }
}

/// Case-insensitive compare, the order the CA launcher uses for pack file names.
pub fn compare_pack_names(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(format_date(0), "");
        assert_eq!(format_date(1_789_661_561), "Sep 17, 2026");
        assert_eq!(format_date(951_782_400), "Feb 29, 2000");
        assert_eq!(ymd(days_from_civil(2026, 4, 2)), (2026, 4, 2));
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn bytes() {
        assert_eq!(format_bytes(587), "587 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(386 * 1024 * 1024), "386 MB");
        assert_eq!(format_bytes(3_650_000_000), "3.4 GB");
    }
}
