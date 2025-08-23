// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use jiff::{civil, Span, Zoned};

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

    pub(super) fn build(self) -> Option<Zoned> {
        let base = self.base.unwrap_or(Zoned::now());

        // If a timestamp is set, we use it to build the `Zoned` object.
        if let Some(ts) = self.timestamp {
            return Some(
                jiff::Timestamp::try_from(ts)
                    .ok()?
                    .to_zoned(base.offset().to_time_zone()),
            );
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
            base.with().time(civil::time(0, 0, 0, 0)).build().ok()?
        };

        if let Some(date) = self.date {
            let d: civil::Date = if date.year.is_some() {
                date.try_into().ok()?
            } else {
                date.with_year(dt.date().year() as u16).try_into().ok()?
            };
            dt = dt.with().date(d).build().ok()?;
        }

        if let Some(time) = self.time.clone() {
            if let Some(offset) = &time.offset {
                dt = dt.datetime().to_zoned(offset.try_into().ok()?).ok()?;
            }

            let t: civil::Time = time.try_into().ok()?;
            dt = dt.with().time(t).build().ok()?;
        }

        if let Some(weekday::Weekday { offset, day }) = self.weekday {
            if self.time.is_none() {
                dt = dt.with().time(civil::time(0, 0, 0, 0)).build().ok()?;
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
            let delta = (day.since(civil::Weekday::Monday) as i32
                - dt.date().weekday().since(civil::Weekday::Monday) as i32)
                .rem_euclid(7)
                + offset.checked_mul(7)?;

            dt = dt.checked_add(Span::new().try_days(delta).ok()?).ok()?;
        }

        for rel in self.relative {
            dt = dt
                .checked_add::<Span>(if let relative::Relative::Months(x) = rel {
                    // *NOTE* This is done in this way to conform to GNU behavior.
                    let days = dt.date().last_of_month().day() as i32;
                    Span::new().try_days(days.checked_mul(x)?).ok()?
                } else {
                    rel.try_into().ok()?
                })
                .ok()?;
        }

        if let Some(offset) = self.timezone {
            let (offset, hour_adjustment) = offset.normalize();
            dt = dt.checked_add(Span::new().hours(hour_adjustment)).ok()?;
            dt = dt.datetime().to_zoned((&offset).try_into().ok()?).ok()?;
        }

        Some(dt)
    }
}
