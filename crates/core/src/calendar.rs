//! Dates and the monthly model calendar.
//!
//! Dates are proleptic Gregorian, held as a day number (days since
//! 1970-01-01) so ranges and day counts are plain integer arithmetic.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub y: i32,
    pub m: u32,
    pub d: u32,
}

/// Days in a month of a year.
pub fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

impl Date {
    /// A validated date.
    pub fn new(y: i32, m: u32, d: u32) -> Result<Date, String> {
        if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
            return Err(format!("{y:04}-{m:02}-{d:02} is not a valid date"));
        }
        Ok(Date { y, m, d })
    }

    /// Parses `YYYY-MM-DD`.
    pub fn parse(s: &str) -> Result<Date, String> {
        let bad = || format!("\"{s}\" is not a date in YYYY-MM-DD form");
        let p: Vec<&str> = s.split('-').collect();
        if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 {
            return Err(bad());
        }
        let y = p[0].parse().map_err(|_| bad())?;
        let m = p[1].parse().map_err(|_| bad())?;
        let d = p[2].parse().map_err(|_| bad())?;
        Date::new(y, m, d)
    }

    /// Days since 1970-01-01 (Howard Hinnant's days_from_civil).
    pub fn day(self) -> i64 {
        let y = i64::from(self.y) - i64::from(self.m <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = (i64::from(self.m) + 9) % 12;
        let doy = (153 * mp + 2) / 5 + i64::from(self.d) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date of a day number (civil_from_days).
    pub fn from_day(z: i64) -> Date {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = (yoe + era * 400 + i64::from(m <= 2)) as i32;
        Date { y, m, d }
    }

    /// This date plus `n` months, clamped to the end of the month
    /// (Jan 31 + 1 month = Feb 28 or 29).
    pub fn add_months(self, n: i64) -> Date {
        let idx = i64::from(self.y) * 12 + i64::from(self.m) - 1 + n;
        let y = idx.div_euclid(12) as i32;
        let m = (idx.rem_euclid(12) + 1) as u32;
        Date {
            y,
            m,
            d: self.d.min(days_in_month(y, m)),
        }
    }

    /// The next day.
    pub fn next_day(self) -> Date {
        Date::from_day(self.day() + 1)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Date, D::Error> {
        let s = String::deserialize(d)?;
        Date::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// The model's months: month 1 is the calendar month of the start date.
#[derive(Clone, Debug)]
pub struct Calendar {
    pub start: Date,
    /// Number of model months.
    pub months: usize,
}

impl Calendar {
    /// The first day of model month `m` (any integer, 1-based).
    pub fn month_start(&self, m: i64) -> Date {
        self.start.add_months(m - 1)
    }

    /// The day range `[first, last + 1)` of model month `m`.
    pub fn month_days(&self, m: i64) -> (i64, i64) {
        (self.month_start(m).day(), self.month_start(m + 1).day())
    }

    /// The model month containing a date (may be <= 0 or past the horizon).
    pub fn month_of(&self, d: Date) -> i64 {
        (i64::from(d.y) - i64::from(self.start.y)) * 12 + i64::from(d.m) - i64::from(self.start.m)
            + 1
    }

    /// Analysis year of model month `m`: floor((m - 1) / 12), 0-based.
    pub fn year_of(m: i64) -> i64 {
        (m - 1).div_euclid(12)
    }

    /// The first day after the model horizon.
    pub fn end_day(&self) -> i64 {
        self.month_start(self.months as i64 + 1).day()
    }

    /// Number of analysis years covering `months` months.
    pub fn years(months: usize) -> usize {
        months.div_ceil(12)
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Date {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Date".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}$",
            "description": "A calendar date, YYYY-MM-DD."
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_numbers_round_trip() {
        assert_eq!(Date::new(1970, 1, 1).unwrap().day(), 0);
        assert_eq!(Date::new(2000, 3, 1).unwrap().day(), 11_017);
        for z in -800_000..800_000 {
            if z % 997 == 0 {
                assert_eq!(Date::from_day(z).day(), z);
            }
        }
    }

    #[test]
    fn add_months_clamps() {
        let d = Date::parse("2024-01-31").unwrap();
        assert_eq!(d.add_months(1).to_string(), "2024-02-29");
        assert_eq!(d.add_months(13).to_string(), "2025-02-28");
        assert_eq!(d.add_months(-2).to_string(), "2023-11-30");
    }

    #[test]
    fn parse_rejects_bad_dates() {
        assert!(Date::parse("2023-02-29").is_err());
        assert!(Date::parse("2024-2-01").is_err());
        assert!(Date::parse("2024-02-29").is_ok());
    }

    #[test]
    fn calendar_months_and_years() {
        let c = Calendar {
            start: Date::parse("2026-07-01").unwrap(),
            months: 30,
        };
        assert_eq!(c.month_of(Date::parse("2026-07-15").unwrap()), 1);
        assert_eq!(c.month_of(Date::parse("2027-06-30").unwrap()), 12);
        assert_eq!(c.month_of(Date::parse("2026-06-30").unwrap()), 0);
        assert_eq!(Calendar::year_of(12), 0);
        assert_eq!(Calendar::year_of(13), 1);
        assert_eq!(Calendar::year_of(0), -1);
        assert_eq!(Calendar::year_of(-11), -1);
        assert_eq!(Calendar::year_of(-12), -2);
        let (a, b) = c.month_days(2);
        assert_eq!(b - a, 31);
        assert_eq!(Calendar::years(30), 3);
    }
}
