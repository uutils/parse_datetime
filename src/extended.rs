// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::GNU_MAX_YEAR;

const SECONDS_PER_DAY: i64 = 86_400;

/// A date-time representation that supports years beyond Jiff's civil range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedDateTime {
    pub year: u32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
    /// Offset in seconds east of UTC.
    pub offset_seconds: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateParts {
    pub year: u32,
    pub month: u8,
    pub day: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeParts {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
}

impl ExtendedDateTime {
    pub fn new(
        date: DateParts,
        time: TimeParts,
        offset_seconds: i32,
    ) -> Result<Self, &'static str> {
        let DateParts { year, month, day } = date;
        let TimeParts {
            hour,
            minute,
            second,
            nanosecond,
        } = time;
        if year > GNU_MAX_YEAR {
            return Err("year must be no greater than 2147485547");
        }
        if !(1..=12).contains(&month) {
            return Err("month must be between 1 and 12");
        }
        let dim = days_in_month(year, month);
        if day == 0 || day > dim {
            return Err("day is not valid for the given month");
        }
        if hour > 23 {
            return Err("hour must be between 0 and 23");
        }
        if minute > 59 {
            return Err("minute must be between 0 and 59");
        }
        if second > 59 {
            return Err("second must be between 0 and 59");
        }
        if nanosecond >= 1_000_000_000 {
            return Err("nanosecond must be between 0 and 999999999");
        }
        if offset_seconds.unsigned_abs() > 24 * 3600 {
            return Err("offset must be between -24:00 and +24:00");
        }
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond,
            offset_seconds,
        })
    }

    pub fn from_unix_seconds(
        unix_seconds: i64,
        nanosecond: u32,
        offset_seconds: i32,
    ) -> Result<Self, &'static str> {
        if nanosecond >= 1_000_000_000 {
            return Err("nanosecond must be between 0 and 999999999");
        }
        if offset_seconds.unsigned_abs() > 24 * 3600 {
            return Err("offset must be between -24:00 and +24:00");
        }

        let local = unix_seconds
            .checked_add(offset_seconds as i64)
            .ok_or("timestamp overflow")?;
        let days = local.div_euclid(SECONDS_PER_DAY);
        let sod = local.rem_euclid(SECONDS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        let year: u32 = year.try_into().map_err(|_| "year must be non-negative")?;
        let month: u8 = month.try_into().map_err(|_| "month is invalid")?;
        let day: u8 = day.try_into().map_err(|_| "day is invalid")?;
        let hour = (sod / 3600) as u8;
        let minute = ((sod % 3600) / 60) as u8;
        let second = (sod % 60) as u8;

        Self::new(
            DateParts { year, month, day },
            TimeParts {
                hour,
                minute,
                second,
                nanosecond,
            },
            offset_seconds,
        )
    }

    pub fn with_date(self, year: u32, month: u8, day: u8) -> Result<Self, &'static str> {
        Self::new(
            DateParts { year, month, day },
            TimeParts {
                hour: self.hour,
                minute: self.minute,
                second: self.second,
                nanosecond: self.nanosecond,
            },
            self.offset_seconds,
        )
    }

    pub fn with_time(
        self,
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> Result<Self, &'static str> {
        Self::new(
            DateParts {
                year: self.year,
                month: self.month,
                day: self.day,
            },
            TimeParts {
                hour,
                minute,
                second,
                nanosecond,
            },
            self.offset_seconds,
        )
    }

    pub fn with_offset(self, offset_seconds: i32) -> Result<Self, &'static str> {
        Self::new(
            DateParts {
                year: self.year,
                month: self.month,
                day: self.day,
            },
            TimeParts {
                hour: self.hour,
                minute: self.minute,
                second: self.second,
                nanosecond: self.nanosecond,
            },
            offset_seconds,
        )
    }

    pub fn checked_add_days(self, days: i64) -> Result<Self, &'static str> {
        let unix = self.unix_seconds();
        let delta = days
            .checked_mul(SECONDS_PER_DAY)
            .ok_or("seconds overflow")?;
        let unix = unix.checked_add(delta).ok_or("timestamp overflow")?;
        Self::from_unix_seconds(unix, self.nanosecond, self.offset_seconds)
    }

    pub fn checked_add_hours(self, hours: i64) -> Result<Self, &'static str> {
        self.checked_add_seconds(hours.checked_mul(3600).ok_or("seconds overflow")?, 0)
    }

    pub fn checked_add_minutes(self, minutes: i64) -> Result<Self, &'static str> {
        self.checked_add_seconds(minutes.checked_mul(60).ok_or("seconds overflow")?, 0)
    }

    pub fn checked_add_seconds(
        self,
        delta_seconds: i64,
        delta_nanoseconds: u32,
    ) -> Result<Self, &'static str> {
        let mut unix = self.unix_seconds();
        unix = unix
            .checked_add(delta_seconds)
            .ok_or("timestamp overflow")?;
        let mut ns = self
            .nanosecond
            .checked_add(delta_nanoseconds)
            .ok_or("nanosecond overflow")?;
        if ns >= 1_000_000_000 {
            unix = unix.checked_add(1).ok_or("timestamp overflow")?;
            ns -= 1_000_000_000;
        }
        Self::from_unix_seconds(unix, ns, self.offset_seconds)
    }

    pub fn checked_add_years(self, years: i32) -> Result<Self, &'static str> {
        let year = (self.year as i64)
            .checked_add(years as i64)
            .ok_or("year overflow")?;
        let year: u32 = year.try_into().map_err(|_| "year must be non-negative")?;
        if year > GNU_MAX_YEAR {
            return Err("year must be no greater than 2147485547");
        }

        // GNU-compatible clamp for leap day.
        let month = self.month;
        let day = if month == 2 && self.day == 29 && !is_leap_year(year) {
            28
        } else {
            self.day
        };
        self.with_date(year, month, day)
    }

    pub fn day_of_year(&self) -> u16 {
        let mut doy = 0u16;
        let mut month = 1u8;
        while month < self.month {
            doy += days_in_month(self.year, month) as u16;
            month += 1;
        }
        doy + self.day as u16
    }

    /// Returns Unix timestamp seconds for this date-time.
    pub fn unix_seconds(&self) -> i64 {
        let days = days_from_civil(self.year as i64, self.month as i64, self.day as i64);
        let daytime = (self.hour as i64) * 3600 + (self.minute as i64) * 60 + (self.second as i64);
        days * SECONDS_PER_DAY + daytime - self.offset_seconds as i64
    }

    /// Weekday in range 0..=6 where 0=Sunday, 1=Monday, ..., 6=Saturday.
    pub fn weekday_sunday0(&self) -> u8 {
        let days = days_from_civil(self.year as i64, self.month as i64, self.day as i64);
        (((days + 4).rem_euclid(7) + 7).rem_euclid(7)) as u8
    }

    /// Weekday in range 0..=6 where 0=Monday, 1=Tuesday, ..., 6=Sunday.
    pub fn weekday_monday0(&self) -> u8 {
        (self.weekday_sunday0() + 6) % 7
    }
}

pub fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn days_in_month(year: u32, month: u8) -> u8 {
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

/// Howard Hinnant's civil date to days algorithm.
///
/// Returns the number of days since 1970-01-01 in the proleptic Gregorian
/// calendar with astronomical year numbering.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of `days_from_civil`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::{is_leap_year, DateParts, ExtendedDateTime, TimeParts};

    #[test]
    fn leap_year_rules() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(10000));
    }

    #[test]
    fn unix_seconds_large_year() {
        let dt = ExtendedDateTime::new(
            DateParts {
                year: 10000,
                month: 1,
                day: 1,
            },
            TimeParts {
                hour: 0,
                minute: 0,
                second: 0,
                nanosecond: 0,
            },
            0,
        )
        .unwrap();
        assert_eq!(dt.unix_seconds(), 253402300800);
    }

    #[test]
    fn unix_roundtrip() {
        let dt = ExtendedDateTime::new(
            DateParts {
                year: 2147485547,
                month: 12,
                day: 31,
            },
            TimeParts {
                hour: 23,
                minute: 59,
                second: 59,
                nanosecond: 123_456_789,
            },
            5 * 3600,
        )
        .unwrap();
        let unix = dt.unix_seconds();
        let rt =
            ExtendedDateTime::from_unix_seconds(unix, dt.nanosecond, dt.offset_seconds).unwrap();
        assert_eq!(dt, rt);
    }
}
