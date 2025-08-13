// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use jiff::{civil::time, ToSpan, Zoned};

use super::{date, epoch, relative, time, timezone, weekday, year};

/// The builder is used to construct a DateTime object from various components.
/// The parser creates a `DateTimeBuilder` object with the parsed components,
/// but without the baseline date and time. So you normally need to set the base
/// date and time using the `set_base()` method before calling `build()`, or
/// leave it unset to use the current date and time as the base.
#[derive(Debug, Default)]
pub(crate) struct DateTimeBuilder {
    base: Option<Zoned>,
    timestamp: Option<epoch::Timestamp>,
    date: Option<date::Date>,
    time: Option<time::Time>,
    weekday: Option<weekday::Weekday>,
    timezone: Option<timezone::Offset>,
    relative: Vec<relative::Relative>,
}

impl DateTimeBuilder {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Sets the base date and time for the builder. If not set, the current
    /// date and time will be used.
    pub(super) fn set_base(mut self, base: Zoned) -> Self {
        self.base = Some(base);
        self
    }

    /// Sets a timestamp value. Timestamp values are exclusive to other date/time
    /// items (date, time, weekday, timezone, relative adjustments).
    pub(super) fn set_timestamp(mut self, ts: epoch::Timestamp) -> Result<Self, &'static str> {
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

    pub(super) fn set_timezone(mut self, timezone: timezone::Offset) -> Result<Self, &'static str> {
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

    fn build_from_timestamp(ts: epoch::Timestamp, tz: jiff::tz::TimeZone) -> Option<Zoned> {
        Some(
            jiff::Timestamp::new(ts.second, ts.nanosecond as i32)
                .ok()?
                .to_zoned(tz),
        )
    }

    pub(super) fn build(self) -> Option<Zoned> {
        let base = self.base.unwrap_or(Zoned::now());

        // If a timestamp is set, we use it to build the Zoned object.
        if let Some(ts) = self.timestamp {
            return Self::build_from_timestamp(ts, base.offset().to_time_zone());
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
            base.with().time(time(0, 0, 0, 0)).build().ok()?
        };

        if let Some(date::Date { year, month, day }) = self.date {
            dt = dt
                .with()
                .year(year.map(|x| x as i16).unwrap_or(dt.year()))
                .month(month as i8)
                .day(day as i8)
                .build()
                .ok()?;
        }

        if let Some(time::Time {
            hour,
            minute,
            second,
            nanosecond,
            ref offset,
        }) = self.time
        {
            let offset = offset
                .clone()
                .and_then(|o| jiff::tz::Offset::try_from(o).ok())
                .unwrap_or(dt.offset());

            dt = dt
                .with()
                .time(time(
                    hour as i8,
                    minute as i8,
                    second as i8,
                    nanosecond as i32,
                ))
                .offset(offset)
                .build()
                .ok()?;
        }

        if let Some(weekday::Weekday { offset, day }) = self.weekday {
            if self.time.is_none() {
                dt = dt
                    .with()
                    .hour(0)
                    .minute(0)
                    .second(0)
                    .nanosecond(0)
                    .build()
                    .ok()?;
            }

            let mut offset = offset;
            let day = day.into();

            // If the current day is not the target day, we need to adjust
            // the x value to ensure we find the correct day.
            //
            // Consider this:
            // Assuming today is Monday, next Friday is actually THIS Friday;
            // but next Monday is indeed NEXT Monday.
            if dt.date().weekday() != day && offset > 0 {
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
            let delta = (day.since(jiff::civil::Weekday::Monday) as i32
                - dt.date().weekday().since(jiff::civil::Weekday::Monday) as i32)
                .rem_euclid(7)
                + offset.checked_mul(7)?;

            dt = dt.checked_add(delta.days()).ok()?;
        }

        for rel in self.relative {
            match rel {
                relative::Relative::Years(x) => {
                    dt = dt.with().year(dt.year() + x as i16).build().ok()?
                }
                relative::Relative::Months(x) => {
                    // *NOTE* This is done in this way to conform to
                    // GNU behavior.
                    let days = dt.date().last_of_month().day();
                    dt = dt.checked_add((days * x as i8).days()).ok()?;
                }
                relative::Relative::Days(x) => dt = dt.checked_add(x.days()).ok()?,
                relative::Relative::Hours(x) => dt = dt.checked_add(x.hours()).ok()?,
                relative::Relative::Minutes(x) => dt = dt.checked_add(x.minutes()).ok()?,
                // Seconds are special because they can be given as a float.
                relative::Relative::Seconds(x) => dt = dt.checked_add((x as i64).seconds()).ok()?,
            }
        }

        if let Some(offset) = self.timezone {
            let tz = jiff::tz::TimeZone::fixed(offset.try_into().ok()?);
            dt = dt.datetime().to_zoned(tz).ok()?;
        }
        Some(dt)
    }
}
