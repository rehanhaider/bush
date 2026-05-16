//! Hand-rolled size + mtime formatting (no chrono/time/humansize deps).

use std::time::{SystemTime, UNIX_EPOCH};

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["", "K", "M", "G", "T", "P"];
    if bytes < 1024 {
        return format!("{bytes}");
    }
    let mut value = bytes as f64;
    let mut unit_idx = 0;
    while value >= 1024.0 && unit_idx + 1 < UNITS.len() {
        value /= 1024.0;
        unit_idx += 1;
    }
    if value >= 100.0 {
        format!("{value:.0}{}", UNITS[unit_idx])
    } else {
        format!("{value:.1}{}", UNITS[unit_idx])
    }
}

pub fn format_mtime(t: SystemTime) -> String {
    let secs = match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => return "----".to_string(),
    };
    let days = secs.div_euclid(86_400);
    let time_of_day = secs.rem_euclid(86_400);
    let (y, mo, d) = days_to_ymd(days);
    let hour = (time_of_day / 3600) as u32;
    let min = ((time_of_day / 60) % 60) as u32;
    format!("{y:04}-{mo:02}-{d:02} {hour:02}:{min:02}")
}

/// Howard Hinnant's days_from_civil inverse — convert days since 1970-01-01 to (year, month, day).
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(unix)]
pub fn format_mode(mode: u32) -> String {
    let mut s = String::with_capacity(9);
    for shift in [6, 3, 0] {
        let r = (mode >> (shift + 2)) & 1;
        let w = (mode >> (shift + 1)) & 1;
        let x = (mode >> shift) & 1;
        s.push(if r == 1 { 'r' } else { '-' });
        s.push(if w == 1 { 'w' } else { '-' });
        s.push(if x == 1 { 'x' } else { '-' });
    }
    s
}

#[cfg(not(unix))]
pub fn format_mode(_mode: u32) -> String {
    "---------".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_bytes() {
        assert_eq!(format_size(0), "0");
        assert_eq!(format_size(1), "1");
        assert_eq!(format_size(999), "999");
        assert_eq!(format_size(1023), "1023");
    }

    #[test]
    fn size_kilobytes() {
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1500), "1.5K");
        assert_eq!(format_size(10 * 1024), "10.0K");
        assert_eq!(format_size(100 * 1024), "100K");
    }

    #[test]
    fn size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0M");
        assert_eq!(format_size(2_500_000), "2.4M");
    }

    #[test]
    fn size_gigabytes() {
        assert_eq!(format_size(1024_u64.pow(3)), "1.0G");
        assert_eq!(format_size(5 * 1024_u64.pow(3)), "5.0G");
    }

    #[test]
    fn size_terabytes() {
        assert_eq!(format_size(1024_u64.pow(4)), "1.0T");
    }

    #[test]
    fn mtime_unix_epoch_is_1970() {
        assert_eq!(format_mtime(UNIX_EPOCH), "1970-01-01 00:00");
    }

    #[test]
    fn mtime_at_y2k() {
        // 2000-01-01 00:00:00 UTC = 946_684_800
        let t = UNIX_EPOCH + std::time::Duration::from_secs(946_684_800);
        assert_eq!(format_mtime(t), "2000-01-01 00:00");
    }

    #[test]
    fn mtime_random_2024() {
        // 2024-03-15 14:30:00 UTC = 1_710_513_000
        let t = UNIX_EPOCH + std::time::Duration::from_secs(1_710_513_000);
        assert_eq!(format_mtime(t), "2024-03-15 14:30");
    }

    #[test]
    fn mtime_leap_day() {
        // 2024-02-29 00:00:00 UTC = 1_709_164_800
        let t = UNIX_EPOCH + std::time::Duration::from_secs(1_709_164_800);
        assert_eq!(format_mtime(t), "2024-02-29 00:00");
    }

    #[cfg(unix)]
    #[test]
    fn mode_644() {
        assert_eq!(format_mode(0o644), "rw-r--r--");
    }

    #[cfg(unix)]
    #[test]
    fn mode_755() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
    }

    #[cfg(unix)]
    #[test]
    fn mode_777() {
        assert_eq!(format_mode(0o777), "rwxrwxrwx");
    }

    #[cfg(unix)]
    #[test]
    fn mode_000() {
        assert_eq!(format_mode(0o000), "---------");
    }

    #[cfg(unix)]
    #[test]
    fn mode_with_file_type_bits_ignored() {
        // 0o100644 has S_IFREG=0o100000 plus 0o644
        assert_eq!(format_mode(0o100644), "rw-r--r--");
    }
}
