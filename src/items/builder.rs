// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, TimeZone, Timelike};

use super::{date, relative, time, weekday, year};

/// The builder is used to construct a DateTime object from various components.
/// The parser creates a `DateTimeBuilder` object with the parsed components,
/// but without the baseline date and time. So you normally need to set the base
/// date and time using the `set_base()` method before calling `build()`, or
/// leave it unset to use the current date and time as the base.
#[derive(Debug, Default)]
pub(crate) struct DateTimeBuilder {
    base: Option<DateTime<FixedOffset>>,
    timestamp: Option<f64>,
    date: Option<date::Date>,
    time: Option<time::Time>,
    weekday: Option<weekday::Weekday>,
    timezone: Option<time::Offset>,
    relative: Vec<relative::Relative>,
}

impl DateTimeBuilder {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Sets the base date and time for the builder. If not set, the current
    /// date and time will be used.
    pub(super) fn set_base(mut self, base: DateTime<FixedOffset>) -> Self {
        self.base = Some(base);
        self
    }

    /// Sets a timestamp value. Timestamp values are exclusive to other date/time
    /// items (date, time, weekday, timezone, relative adjustments).
    pub(super) fn set_timestamp(mut self, ts: f64) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot appear more than once");
        } else if self.date.is_some()
            || self.time.is_some()
            || self.weekday.is_some()
            || self.timezone.is_some()
            || !self.relative.is_empty()
        {
            return Err("timestamp cannot be combined with other date/time items");
        }

        self.timestamp = Some(ts);
        Ok(self)
    }

    pub(super) fn set_date(mut self, date: date::Date) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.date.is_some() {
            return Err("date cannot appear more than once");
        }

        self.date = Some(date);
        Ok(self)
    }

    pub(super) fn set_time(mut self, time: time::Time) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.time.is_some() {
            return Err("time cannot appear more than once");
        } else if self.timezone.is_some() && time.offset.is_some() {
            return Err("time offset and timezone are mutually exclusive");
        }

        self.time = Some(time);
        Ok(self)
    }

    pub(super) fn set_weekday(mut self, weekday: weekday::Weekday) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.weekday.is_some() {
            return Err("weekday cannot appear more than once");
        }

        self.weekday = Some(weekday);
        Ok(self)
    }

    pub(super) fn set_timezone(mut self, timezone: time::Offset) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.timezone.is_some() {
            return Err("timezone cannot appear more than once");
        } else if self.time.as_ref().and_then(|t| t.offset.as_ref()).is_some() {
            return Err("time offset and timezone are mutually exclusive");
        }

        self.timezone = Some(timezone);
        Ok(self)
    }

    pub(super) fn push_relative(
        mut self,
        relative: relative::Relative,
    ) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        }

        self.relative.push(relative);
        Ok(self)
    }

    /// Sets a pure number that can be interpreted as either a year or time
    /// depending on the current state of the builder.
    ///
    /// If a date is already set but lacks a year, the number is interpreted as
    /// a year. Otherwise, it's interpreted as a time in HHMM, HMM, HH, or H
    /// format.
    pub(super) fn set_pure(mut self, pure: String) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        }

        if let Some(date) = self.date.as_mut() {
            if date.year.is_none() {
                date.year = Some(year::year_from_str(&pure)?);
                return Ok(self);
            }
        }

        let (mut hour_str, mut minute_str) = match pure.len() {
            1..=2 => (pure.as_str(), "0"),
            3..=4 => pure.split_at(pure.len() - 2),
            _ => {
                return Err("pure number must be 1-4 digits when interpreted as time");
            }
        };

        let hour = time::hour24(&mut hour_str).map_err(|_| "invalid hour in pure number")?;
        let minute = time::minute(&mut minute_str).map_err(|_| "invalid minute in pure number")?;

        let time = time::Time {
            hour,
            minute,
            ..Default::default()
        };
        self.set_time(time)
    }

    fn build_from_timestamp(ts: f64, tz: &FixedOffset) -> Option<DateTime<FixedOffset>> {
        // TODO: How to make the fract -> nanosecond conversion more precise?
        // Maybe considering using the
        // [rust_decimal](https://crates.io/crates/rust_decimal) crate?
        match chrono::Utc.timestamp_opt(ts as i64, (ts.fract() * 10f64.powi(9)).round() as u32) {
            chrono::MappedLocalTime::Single(t) => Some(t.with_timezone(tz)),
            chrono::MappedLocalTime::Ambiguous(earliest, _latest) => {
                // TODO: When there is a fold in the local time, which value
                // do we choose? For now, we use the earliest one.
                Some(earliest.with_timezone(tz))
            }
            chrono::MappedLocalTime::None => None, // Invalid timestamp
        }
    }

    pub(super) fn build(self) -> Option<DateTime<FixedOffset>> {
        let base = self.base.unwrap_or_else(|| chrono::Local::now().into());

        // If a timestamp is set, we use it to build the DateTime object.
        if let Some(ts) = self.timestamp {
            return Self::build_from_timestamp(ts, base.offset());
        }

        // If any of the following items are set, we truncate the time portion
        // of the base date to zero; otherwise, we use the base date as is.
        let mut dt = if self.timestamp.is_none()
            && self.date.is_none()
            && self.time.is_none()
            && self.weekday.is_none()
            && self.timezone.is_none()
        {
            base
        } else {
            new_date(
                base.year(),
                base.month(),
                base.day(),
                0,
                0,
                0,
                0,
                *base.offset(),
            )?
        };

        if let Some(date::Date { year, month, day }) = self.date {
            dt = new_date(
                year.map(|x| x as i32).unwrap_or(dt.year()),
                month,
                day,
                dt.hour(),
                dt.minute(),
                dt.second(),
                dt.nanosecond(),
                *dt.offset(),
            )?;
        }

        if let Some(time::Time {
            hour,
            minute,
            second,
            ref offset,
        }) = self.time
        {
            let offset = offset
                .clone()
                .and_then(|o| chrono::FixedOffset::try_from(o).ok())
                .unwrap_or(*dt.offset());

            dt = new_date(
                dt.year(),
                dt.month(),
                dt.day(),
                hour,
                minute,
                second as u32,
                (second.fract() * 10f64.powi(9)).round() as u32,
                offset,
            )?;
        }

        if let Some(weekday::Weekday { offset, day }) = self.weekday {
            if self.time.is_none() {
                dt = new_date(dt.year(), dt.month(), dt.day(), 0, 0, 0, 0, *dt.offset())?;
            }

            let mut offset = offset;
            let day = day.into();

            // If the current day is not the target day, we need to adjust
            // the x value to ensure we find the correct day.
            //
            // Consider this:
            // Assuming today is Monday, next Friday is actually THIS Friday;
            // but next Monday is indeed NEXT Monday.
            if dt.weekday() != day && offset > 0 {
                offset -= 1;
            }

            // Calculate the delta to the target day.
            //
            // Assuming today is Thursday, here are some examples:
            //
            // Example 1: last Thursday (x = -1, day = Thursday)
            //            delta = (3 - 3) % 7 + (-1) * 7 = -7
            //
            // Example 2: last Monday (x = -1, day = Monday)
            //            delta = (0 - 3) % 7 + (-1) * 7 = -3
            //
            // Example 3: next Monday (x = 1, day = Monday)
            //            delta = (0 - 3) % 7 + (0) * 7 = 4
            // (Note that we have adjusted the x value above)
            //
            // Example 4: next Thursday (x = 1, day = Thursday)
            //            delta = (3 - 3) % 7 + (1) * 7 = 7
            let delta = (day.num_days_from_monday() as i32
                - dt.weekday().num_days_from_monday() as i32)
                .rem_euclid(7)
                + offset.checked_mul(7)?;

            dt = if delta < 0 {
                dt.checked_sub_days(chrono::Days::new((-delta) as u64))?
            } else {
                dt.checked_add_days(chrono::Days::new(delta as u64))?
            }
        }

        for rel in self.relative {
            // TODO: Handle potential overflows in the addition operations.
            match rel {
                relative::Relative::Years(x) => {
                    dt = dt.with_year(dt.year() + x)?;
                }
                relative::Relative::Months(x) => {
                    // *NOTE* This is done in this way to conform to
                    // GNU behavior.
                    let days = last_day_of_month(dt.year(), dt.month());
                    if x >= 0 {
                        dt += dt
                            .date_naive()
                            .checked_add_days(chrono::Days::new((days * x as u32) as u64))?
                            .signed_duration_since(dt.date_naive());
                    } else {
                        dt += dt
                            .date_naive()
                            .checked_sub_days(chrono::Days::new((days * -x as u32) as u64))?
                            .signed_duration_since(dt.date_naive());
                    }
                }
                relative::Relative::Days(x) => dt += chrono::Duration::days(x.into()),
                relative::Relative::Hours(x) => dt += chrono::Duration::hours(x.into()),
                relative::Relative::Minutes(x) => {
                    dt += chrono::Duration::try_minutes(x.into())?;
                }
                // Seconds are special because they can be given as a float.
                relative::Relative::Seconds(x) => {
                    dt += chrono::Duration::try_seconds(x as i64)?;
                }
            }
        }

        if let Some(offset) = self.timezone {
            dt = with_timezone_restore(offset, dt)?;
        }

        Some(dt)
    }
}

#[allow(clippy::too_many_arguments, deprecated)]
fn new_date(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    nano: u32,
    offset: FixedOffset,
) -> Option<DateTime<FixedOffset>> {
    let newdate = NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|naive| naive.and_hms_nano_opt(hour, minute, second, nano))?;

    Some(DateTime::<FixedOffset>::from_local(newdate, offset))
}

/// Restores year, month, day, etc after applying the timezone
/// returns None if timezone overflows the date
fn with_timezone_restore(
    offset: time::Offset,
    at: DateTime<FixedOffset>,
) -> Option<DateTime<FixedOffset>> {
    let offset: FixedOffset = chrono::FixedOffset::try_from(offset).ok()?;
    let copy = at;
    let x = at
        .with_timezone(&offset)
        .with_day(copy.day())?
        .with_month(copy.month())?
        .with_year(copy.year())?
        .with_hour(copy.hour())?
        .with_minute(copy.minute())?
        .with_second(copy.second())?
        .with_nanosecond(copy.nanosecond())?;
    Some(x)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(year, month + 1, 1)
        .unwrap_or(NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
        .pred_opt()
        .unwrap()
        .day()
}
