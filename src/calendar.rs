use chrono::{DateTime, Datelike, Local, Timelike};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarSystem {
    Jalali,
    Gregorian,
}

impl Default for CalendarSystem {
    fn default() -> Self {
        CalendarSystem::Jalali
    }
}

impl fmt::Display for CalendarSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalendarSystem::Jalali => write!(f, "jalali"),
            CalendarSystem::Gregorian => write!(f, "gregorian"),
        }
    }
}

impl std::str::FromStr for CalendarSystem {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gregorian" | "greg" => Ok(CalendarSystem::Gregorian),
            _ => Ok(CalendarSystem::Jalali),
        }
    }
}

/// Convert Gregorian year, month, day to Jalali (Solar Hijri) year, month, day.
pub fn gregorian_to_jalali(gy: i32, gm: u32, gd: u32) -> (i32, u32, u32) {
    let g_d_m = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let gy2 = if gm > 2 { gy } else { gy - 1 };
    let mut days = 355666
        + (365 * gy)
        + ((gy2 + 3) / 4)
        - ((gy2 + 99) / 100)
        + ((gy2 + 399) / 400)
        + (gd as i32)
        + g_d_m[(gm - 1) as usize];

    let mut jy = -1595 + (33 * (days / 12053));
    days %= 12053;
    jy += 4 * (days / 1461);
    days %= 1461;

    if days > 365 {
        jy += (days - 1) / 365;
        days = (days - 1) % 365;
    }

    let (jm, jd) = if days < 186 {
        (1 + (days / 31) as u32, 1 + (days % 31) as u32)
    } else {
        (7 + ((days - 186) / 30) as u32, 1 + ((days - 186) % 30) as u32)
    };

    (jy, jm, jd)
}

/// Convert Jalali (Solar Hijri) year, month, day to Gregorian year, month, day.
pub fn jalali_to_gregorian(jy: i32, jm: u32, jd: u32) -> (i32, u32, u32) {
    let j_days = if jm <= 6 {
        ((jm - 1) * 31 + (jd - 1)) as i32
    } else {
        (186 + (jm - 7) * 30 + (jd - 1)) as i32
    };

    let jy_offset = jy + 1595;
    let cycle33 = jy_offset / 33;
    let rem33 = jy_offset % 33;
    let cycle4 = rem33 / 4;
    let rem4 = rem33 % 4;

    let mut days = cycle33 * 12053 + cycle4 * 1461 + rem4 * 365 + j_days;
    if rem4 > 0 {
        days += 1;
    }

    let mut sal_g = days - 355666;
    let mut gy = 400 * (sal_g / 146097);
    sal_g %= 146097;

    if sal_g > 36524 {
        sal_g -= 1;
        gy += 100 * (sal_g / 36524);
        sal_g %= 36524;
        if sal_g >= 365 {
            sal_g += 1;
        }
    }

    gy += 4 * (sal_g / 1461);
    sal_g %= 1461;

    if sal_g > 365 {
        gy += (sal_g - 1) / 365;
        sal_g = (sal_g - 1) % 365;
    }

    let g_days_in_month = [
        31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];

    let mut gm = 0;
    while gm < 12 && sal_g > g_days_in_month[gm] {
        sal_g -= g_days_in_month[gm];
        gm += 1;
    }

    (gy, (gm + 1) as u32, sal_g as u32)
}

/// Helper to parse an entry's ID or date string into a normalized Gregorian NaiveDate
pub fn parse_entry_to_naive_date(id: &str, declared_cal: Option<&str>) -> Option<chrono::NaiveDate> {
    let date_str = id.split('_').next()?;
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() < 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;

    let is_jalali = if let Some(cal) = declared_cal {
        cal.to_lowercase() == "jalali"
    } else {
        y < 1900 // Heuristic: Jalali years are ~1300-1500, Gregorian are ~1900-2100
    };

    if is_jalali {
        let (gy, gm, gd) = jalali_to_gregorian(y, m, d);
        chrono::NaiveDate::from_ymd_opt(gy, gm, gd)
    } else {
        chrono::NaiveDate::from_ymd_opt(y, m, d)
    }
}

/// Generate timestamp according to calendar system and time
pub fn format_timestamp(dt: &DateTime<Local>, cal: CalendarSystem) -> String {
    match cal {
        CalendarSystem::Jalali => {
            let (jy, jm, jd) = gregorian_to_jalali(dt.year(), dt.month(), dt.day());
            format!(
                "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
                jy,
                jm,
                jd,
                dt.hour(),
                dt.minute(),
                dt.second()
            )
        }
        CalendarSystem::Gregorian => dt.format("%Y-%m-%d_%H-%M-%S").to_string(),
    }
}

/// Format a Unix epoch timestamp to string
pub fn format_epoch_timestamp(epoch: i64, cal: CalendarSystem) -> String {
    if let Some(naive) = DateTime::from_timestamp(epoch, 0) {
        let local: DateTime<Local> = DateTime::from(naive);
        format_timestamp(&local, cal)
    } else {
        let now = Local::now();
        format_timestamp(&now, cal)
    }
}

/// Get current timestamp formatted for entry directory ID
pub fn get_current_timestamp(cal: CalendarSystem) -> String {
    let now = Local::now();
    format_timestamp(&now, cal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gregorian_to_jalali() {
        let (jy, jm, jd) = gregorian_to_jalali(2026, 8, 22);
        assert_eq!(jy, 1405);
        assert_eq!(jm, 5);
        assert_eq!(jd, 31);

        let (jy, jm, jd) = gregorian_to_jalali(2024, 3, 21);
        assert_eq!(jy, 1403);
        assert_eq!(jm, 1);
        assert_eq!(jd, 1);

        let (jy, jm, jd) = gregorian_to_jalali(2024, 3, 20);
        assert_eq!(jy, 1402);
        assert_eq!(jm, 12);
        assert_eq!(jd, 29);
    }

    #[test]
    fn test_jalali_to_gregorian() {
        let (gy, gm, gd) = jalali_to_gregorian(1405, 5, 31);
        assert_eq!(gy, 2026);
        assert_eq!(gm, 8);
        assert_eq!(gd, 22);

        let (gy, gm, gd) = jalali_to_gregorian(1403, 1, 1);
        assert_eq!(gy, 2024);
        assert_eq!(gm, 3);
        assert_eq!(gd, 21);

        let (gy, gm, gd) = jalali_to_gregorian(1402, 12, 29);
        assert_eq!(gy, 2024);
        assert_eq!(gm, 3);
        assert_eq!(gd, 20);
    }

    #[test]
    fn test_parse_entry_to_naive_date() {
        let d1 = parse_entry_to_naive_date("1405-05-31_12-00-00", Some("jalali")).unwrap();
        assert_eq!(d1, chrono::NaiveDate::from_ymd_opt(2026, 8, 22).unwrap());

        let d2 = parse_entry_to_naive_date("2026-08-22_12-00-00", Some("gregorian")).unwrap();
        assert_eq!(d2, chrono::NaiveDate::from_ymd_opt(2026, 8, 22).unwrap());
    }
}
