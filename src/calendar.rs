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
}
