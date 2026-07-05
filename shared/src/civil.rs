//! Minimal pure Gregorian-calendar math for the day model. Deliberately
//! dependency-free (Global Constraints: no chrono in `shared`): Howard
//! Hinnant's civil-days algorithms plus total-function helpers. The shell
//! supplies "today"; nothing here reads a clock.

pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
pub const MONTH_ABBREV: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
/// Monday-first, matching the design reference's calendar (§2.3).
pub const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDate {
    pub year: i32,
    pub month: u32, // 1-12
    pub day: u32,   // 1-31, validated by parse/from_days
}

impl CivilDate {
    /// Parse strict `YYYY-MM-DD`; rejects impossible dates.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('-');
        let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
            return None;
        }
        let year: i32 = y.parse().ok()?;
        let month: u32 = m.parse().ok()?;
        let day: u32 = d.parse().ok()?;
        ((1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day))
            .then_some(Self { year, month, day })
    }

    #[must_use]
    pub fn iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Days since 1970-01-01 (negative before). Hinnant's days_from_civil.
    fn to_days(self) -> i64 {
        let y = i64::from(if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        });
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400; // [0, 399]
        let m = i64::from(self.month);
        let d = i64::from(self.day);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Hinnant's civil_from_days.
    fn from_days(z: i64) -> Self {
        let z = z + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        Self {
            year: (if m <= 2 { y + 1 } else { y }) as i32,
            month: m as u32,
            day: d as u32,
        }
    }

    /// 0 = Monday … 6 = Sunday. 1970-01-01 (days = 0) was a Thursday (= 3).
    #[must_use]
    pub fn weekday(&self) -> u32 {
        ((self.to_days() + 3).rem_euclid(7)) as u32
    }

    #[must_use]
    pub fn add_days(&self, delta: i64) -> Self {
        Self::from_days(self.to_days() + delta)
    }

    /// "Thursday, July 2" — the daily-note title (reference §5).
    #[must_use]
    pub fn display_title(&self) -> String {
        format!(
            "{}, {} {}",
            WEEKDAY_NAMES[self.weekday() as usize],
            MONTH_NAMES[(self.month - 1) as usize],
            self.day
        )
    }

    /// "Jul 2" — the sidebar Today-row date (reference §2.2).
    #[must_use]
    pub fn short_label(&self) -> String {
        format!("{} {}", MONTH_ABBREV[(self.month - 1) as usize], self.day)
    }
}

#[must_use]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[must_use]
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

#[must_use]
pub fn prev_month(year: i32, month: u32) -> (i32, u32) {
    if month <= 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

#[must_use]
pub fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month >= 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

/// "July 2026" — the calendar header (reference §2.3).
#[must_use]
pub fn month_label(year: i32, month: u32) -> String {
    format!("{} {}", MONTH_NAMES[(month - 1) as usize], year)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_iso_round_trip() {
        let d = CivilDate::parse("2026-07-04").unwrap();
        assert_eq!((d.year, d.month, d.day), (2026, 7, 4));
        assert_eq!(d.iso(), "2026-07-04");
    }

    #[test]
    fn parse_rejects_garbage() {
        for bad in [
            "",
            "garbage",
            "2026-13-01",
            "2026-02-30",
            "2026-07-04-05",
            "07-04-2026",
        ] {
            assert!(CivilDate::parse(bad).is_none(), "should reject {bad:?}");
        }
    }

    #[test]
    fn weekdays_match_known_dates() {
        // Anchors from the design reference: July 1 2026 is a Wednesday,
        // July 2 a Thursday. Unix epoch day zero was a Thursday.
        assert_eq!(CivilDate::parse("1970-01-01").unwrap().weekday(), 3);
        assert_eq!(CivilDate::parse("2026-07-01").unwrap().weekday(), 2);
        assert_eq!(CivilDate::parse("2026-07-02").unwrap().weekday(), 3);
        assert_eq!(CivilDate::parse("2026-07-04").unwrap().weekday(), 5); // Saturday
    }

    #[test]
    fn add_days_crosses_months_years_and_leap_days() {
        let jump = |s: &str, n: i64| CivilDate::parse(s).unwrap().add_days(n).iso();
        assert_eq!(jump("2026-07-04", 28), "2026-08-01");
        assert_eq!(jump("2025-12-31", 1), "2026-01-01");
        assert_eq!(jump("2024-02-28", 1), "2024-02-29"); // leap
        assert_eq!(jump("2026-02-28", 1), "2026-03-01"); // not leap
        assert_eq!(jump("2026-07-04", -4), "2026-06-30");
    }

    #[test]
    fn display_strings_match_the_design_reference() {
        let d = CivilDate::parse("2026-07-02").unwrap();
        assert_eq!(d.display_title(), "Thursday, July 2"); // reference §5
        assert_eq!(d.short_label(), "Jul 2"); // reference §2.2
        assert_eq!(month_label(2026, 7), "July 2026"); // reference §2.3
    }

    #[test]
    fn month_arithmetic_wraps_years() {
        assert_eq!(prev_month(2026, 1), (2025, 12));
        assert_eq!(next_month(2026, 12), (2027, 1));
        assert_eq!(prev_month(2026, 7), (2026, 6));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 6), 30);
    }

    // --- Additional vectors beyond the brief (see task-5 report §self-review) ---

    #[test]
    fn century_leap_year_rules() {
        // 1900 and 2100 are NOT leap (divisible by 100 but not 400);
        // 2000 IS leap (divisible by 400). Classic Gregorian edge case.
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(2100));
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert!(CivilDate::parse("1900-02-29").is_none());
        assert!(CivilDate::parse("2000-02-29").is_some());
    }

    #[test]
    fn add_days_crosses_a_century_leap_boundary() {
        // Feb 28 2000 -> Feb 29 2000 (2000 is leap) -> Mar 1 2000.
        let jump = |s: &str, n: i64| CivilDate::parse(s).unwrap().add_days(n).iso();
        assert_eq!(jump("2000-02-28", 1), "2000-02-29");
        assert_eq!(jump("2000-02-28", 2), "2000-03-01");
        // 1900 is not leap: Feb 28 1900 -> Mar 1 1900 directly.
        assert_eq!(jump("1900-02-28", 1), "1900-03-01");
    }

    #[test]
    fn year_boundary_weekday_and_add_days_agree() {
        // 2026-01-01 is a Thursday (weekday 3); stepping back one day from
        // 2026-01-01 must land on 2025-12-31, a Wednesday (weekday 2).
        let jan1 = CivilDate::parse("2026-01-01").unwrap();
        assert_eq!(jan1.weekday(), 3);
        let dec31 = jan1.add_days(-1);
        assert_eq!(dec31.iso(), "2025-12-31");
        assert_eq!(dec31.weekday(), 2);
    }
}
