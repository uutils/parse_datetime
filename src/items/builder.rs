// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use jiff::{civil, Span, ToSpan, Zoned};

use super::{date, epoch, error, offset, relative, time, weekday, year, Item};
use crate::extended::{DateParts, TimeParts};
use crate::{ExtendedDateTime, ParsedDateTime};

/// The builder is used to construct a DateTime object from various components.
/// The parser creates a `DateTimeBuilder` object with the parsed components,
/// but without the baseline date and time. So you normally need to set the base
/// date and time using the `set_base()` method before calling `build()`, or
/// leave it unset to use the current date and time as the base.
#[derive(Debug, Default, Clone)]
pub(crate) struct DateTimeBuilder {
    base: Option<Zoned>,
    timestamp: Option<epoch::Timestamp>,
    date: Option<date::Date>,
    time: Option<time::Time>,
    weekday: Option<weekday::Weekday>,
    offset: Option<offset::Offset>,
    timezone: Option<jiff::tz::TimeZone>,
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

    /// Sets the timezone rule for the builder.
    ///
    /// By default, the builder uses the time zone rules indicated by the `TZ`
    /// environment variable, or the system default rules if `TZ` is not set.
    /// This method allows overriding the time zone rules.
    fn set_timezone(mut self, tz: jiff::tz::TimeZone) -> Result<Self, &'static str> {
        if self.timezone.is_some() {
            return Err("timezone rule cannot appear more than once");
        }

        self.timezone = Some(tz);
        Ok(self)
    }

    /// Sets a timestamp value. Timestamp values are exclusive to other date/time
    /// items (date, time, weekday, timezone, relative adjustments).
    pub(super) fn set_timestamp(mut self, ts: epoch::Timestamp) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot appear more than once");
        } else if self.date.is_some()
            || self.time.is_some()
            || self.weekday.is_some()
            || self.offset.is_some()
            || !self.relative.is_empty()
        {
            return Err("timestamp cannot be combined with other date/time items");
        }

        self.timestamp = Some(ts);
        Ok(self)
    }

    fn set_date(mut self, date: date::Date) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.date.is_some() {
            return Err("date cannot appear more than once");
        }

        self.date = Some(date);
        Ok(self)
    }

    fn set_time(mut self, time: time::Time) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.time.is_some() {
            return Err("time cannot appear more than once");
        } else if self.offset.is_some() && time.offset.is_some() {
            return Err("time offset and timezone are mutually exclusive");
        }

        self.time = Some(time);
        Ok(self)
    }

    fn set_weekday(mut self, weekday: weekday::Weekday) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.weekday.is_some() {
            return Err("weekday cannot appear more than once");
        }

        self.weekday = Some(weekday);
        Ok(self)
    }

    fn set_offset(mut self, timezone: offset::Offset) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.offset.is_some()
            || self.time.as_ref().and_then(|t| t.offset.as_ref()).is_some()
        {
            return Err("time offset cannot appear more than once");
        }

        self.offset = Some(timezone);
        Ok(self)
    }

    fn push_relative(mut self, relative: relative::Relative) -> Result<Self, &'static str> {
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
    fn set_pure(mut self, pure: String) -> Result<Self, &'static str> {
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

    /// Build a parsed datetime result from the pieces accumulated in this builder.
    ///
    /// Returns [`ParsedDateTime::InRange`] when the result is representable as a
    /// [`jiff::Zoned`], or [`ParsedDateTime::Extended`] for large/out-of-range
    /// year scenarios.
    ///
    /// Resolution order (mirrors GNU `date` semantics):
    ///
    /// 1. Base instant.
    ///   - a. If `self.base` is provided, start with it.
    ///   - b. Else if a `timezone` rule is present, start with "now" in that
    ///     timezone.
    ///   - c. Else start with current system local time.
    ///
    /// 2. Absolute timestamp override.
    ///   - a. If `self.timestamp` is set, it fully determines the result.
    ///
    /// 3. Time of day truncation.
    ///   - a. If any of date, time, weekday, offset, timezone is set, zero the
    ///     time of day to 00:00:00 before applying fields.
    ///
    /// 4. Fieldwise resolution (applied to the base instant).
    ///   - a. Apply date. If year is absent in the parsed date, inherit the year
    ///     from the base instant.
    ///   - b. Apply time. If time carries an explicit numeric offset, apply the
    ///     offset before setting time.
    ///   - c. Apply weekday (e.g., "next Friday" or "last Monday").
    ///   - d. Apply relative adjustments (e.g., "+3 days", "-2 months").
    ///   - e. Apply final fixed offset if present.
    pub(super) fn build(self) -> Result<ParsedDateTime, error::Error> {
        if let Some(date) = self.date.as_ref() {
            if date.year.unwrap_or(0) > 9999 {
                return self.build_extended();
            }
        }

        if !self.should_try_extended_fallback() {
            return self.build_in_range().map(ParsedDateTime::InRange);
        }

        // Near the year-9999 boundary, otherwise valid inputs can fail during
        // in-range resolution due to transient timestamp/offset conversions.
        // Retry in extended mode for boundary-risk contexts.
        match self.clone().build_in_range() {
            Ok(dt) => Ok(ParsedDateTime::InRange(dt)),
            Err(in_range_err) => self.build_extended().or(Err(in_range_err)),
        }
    }

    fn should_try_extended_fallback(&self) -> bool {
        if self.timestamp.is_some() {
            return false;
        }

        self.date
            .as_ref()
            .is_some_and(|d| d.year.unwrap_or(0) >= 9999)
            || self.base.as_ref().is_some_and(|b| b.year() >= 9999)
    }

    fn build_in_range(self) -> Result<Zoned, error::Error> {
        // 1. Choose the base instant.
        // If a TZ="..." prefix was parsed, it should override the base's timezone
        // while keeping the base's timestamp for relative date calculations.
        let has_timezone = self.timezone.is_some();
        let base = match (self.base, self.timezone) {
            (Some(b), Some(tz)) => b.timestamp().to_zoned(tz),
            (Some(b), None) => b,
            (None, Some(tz)) => jiff::Timestamp::now().to_zoned(tz),
            (None, None) => Zoned::now(),
        };

        // 2. Absolute timestamp override everything else.
        if let Some(ts) = self.timestamp {
            let ts = jiff::Timestamp::try_from(ts)?;
            return Ok(ts.to_zoned(base.offset().to_time_zone()));
        }

        // 3. Determine whether to truncate the time of day.
        let need_midnight = self.date.is_some()
            || self.time.is_some()
            || self.weekday.is_some()
            || self.offset.is_some()
            || has_timezone;

        let mut dt = if need_midnight {
            base.with().time(civil::time(0, 0, 0, 0)).build()?
        } else {
            base
        };

        // 4a. Apply date.
        if let Some(date) = self.date {
            let d: civil::Date = if date.year.is_some() {
                date.try_into()?
            } else {
                let base_year = u32::try_from(dt.date().year())
                    .map_err(|_| "base year must be non-negative")?;
                date.with_year(base_year).try_into()?
            };
            dt = dt.with().date(d).build()?;
        }

        // 4b. Apply time.
        if let Some(time) = self.time.clone() {
            if let Some(offset) = &time.offset {
                dt = dt.datetime().to_zoned(offset.try_into()?)?;
            }

            let t: civil::Time = time.try_into()?;
            dt = dt.with().time(t).build()?;
        }

        // 4c. Apply weekday.
        if let Some(weekday::Weekday { mut offset, day }) = self.weekday {
            if self.time.is_none() {
                dt = dt.with().time(civil::time(0, 0, 0, 0)).build()?;
            }

            let target = day.into();

            // If the current day is not the target day, we need to adjust
            // the x value to ensure we find the correct day.
            //
            // Consider this:
            // Assuming today is Monday, next Friday is actually THIS Friday;
            // but next Monday is indeed NEXT Monday.
            if dt.date().weekday() != target && offset > 0 {
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
            let delta = (target.since(civil::Weekday::Monday) as i32
                - dt.date().weekday().since(civil::Weekday::Monday) as i32)
                .rem_euclid(7)
                + offset.checked_mul(7).ok_or("multiplication overflow")?;

            dt = dt.checked_add(Span::new().try_days(delta)?)?;
        }

        // 4d. Apply relative adjustments.
        for rel in self.relative {
            dt = match rel {
                relative::Relative::Years(_) | relative::Relative::Months(_) => {
                    // GNU way of calculating relative months and years.
                    // GNU changes the month/year and then checks if the target month has
                    // this day. If this day does not exist in the target month it overflows
                    // the difference.
                    let original_day_of_month = dt.day();
                    dt = dt.checked_add::<Span>(rel.try_into()?)?;
                    if original_day_of_month != dt.day() {
                        dt = dt.checked_add(
                            (original_day_of_month.checked_sub(dt.day()).unwrap_or(0)).days(),
                        )?;
                    }
                    dt
                }
                _ => dt.checked_add::<Span>(rel.try_into()?)?,
            };
        }

        // 4e. Apply final fixed offset.
        if let Some(offset) = self.offset {
            let (offset, hour_adjustment) = offset.normalize();
            dt = dt.checked_add(Span::new().hours(hour_adjustment))?;
            dt = dt.datetime().to_zoned((&offset).try_into()?)?;
        }

        Ok(dt)
    }

    fn build_extended(self) -> Result<ParsedDateTime, error::Error> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with large years".into());
        }
        let DateTimeBuilder {
            base,
            timestamp: _,
            date,
            time,
            weekday,
            offset,
            timezone,
            relative,
        } = self;

        let has_timezone = timezone.is_some();
        let base = match (base, timezone) {
            (Some(b), Some(tz)) => b.timestamp().to_zoned(tz),
            (Some(b), None) => b,
            (None, Some(tz)) => jiff::Timestamp::now().to_zoned(tz),
            (None, None) => Zoned::now(),
        };
        let rule_tz = base.time_zone().clone();

        let need_midnight = date.is_some()
            || time.is_some()
            || weekday.is_some()
            || offset.is_some()
            || has_timezone;
        let mut dt = ExtendedDateTime::new(
            DateParts {
                year: u32::try_from(base.year()).map_err(|_| "year must be non-negative")?,
                month: base.month() as u8,
                day: base.day() as u8,
            },
            TimeParts {
                hour: if need_midnight { 0 } else { base.hour() as u8 },
                minute: if need_midnight {
                    0
                } else {
                    base.minute() as u8
                },
                second: if need_midnight {
                    0
                } else {
                    base.second() as u8
                },
                nanosecond: if need_midnight {
                    0
                } else {
                    base.subsec_nanosecond() as u32
                },
            },
            base.offset().seconds(),
        )?;

        if let Some(date) = date {
            let year = date.year.unwrap_or(dt.year);
            dt = dt.with_date(year, date.month, date.day)?;
        }

        let had_time_item = time.is_some();
        let has_time_offset = time.as_ref().and_then(|t| t.offset.as_ref()).is_some();
        if let Some(time) = time {
            if let Some(offset) = time.offset {
                dt = dt.with_offset(offset.total_seconds())?;
            }
            dt = dt.with_time(time.hour, time.minute, time.second, time.nanosecond)?;
        }

        if let Some(weekday::Weekday {
            mut offset,
            day: target_day,
        }) = weekday
        {
            if !had_time_item {
                dt = dt.with_time(0, 0, 0, 0)?;
            }

            let target = weekday_monday0(target_day);
            if dt.weekday_monday0() != target && offset > 0 {
                offset -= 1;
            }

            let delta = (target as i32 - dt.weekday_monday0() as i32).rem_euclid(7)
                + offset.checked_mul(7).ok_or("multiplication overflow")?;
            dt = dt.checked_add_days(delta as i64)?;
        }

        for rel in relative {
            dt = match rel {
                relative::Relative::Years(years) => dt.checked_add_years(years)?,
                relative::Relative::Months(months) => dt.checked_add_months(months)?,
                relative::Relative::Days(days) => dt.checked_add_days(days as i64)?,
                relative::Relative::Hours(hours) => dt.checked_add_hours(hours as i64)?,
                relative::Relative::Minutes(minutes) => dt.checked_add_minutes(minutes as i64)?,
                relative::Relative::Seconds(seconds, nanos) => {
                    dt.checked_add_seconds(seconds, nanos)?
                }
            };
        }

        if !has_time_offset && offset.is_none() {
            let offset_seconds = resolve_rule_offset_for_extended(&rule_tz, &dt)?;
            dt = dt.with_offset(offset_seconds)?;
        }

        if let Some(offset) = offset {
            let (offset, hour_adjustment) = offset.normalize();
            dt = dt.checked_add_hours(hour_adjustment as i64)?;
            dt = dt.with_offset(offset.total_seconds())?;
        }

        if dt.year <= 9999 {
            if let (Ok(ts), Ok(offset)) = (
                jiff::Timestamp::new(dt.unix_seconds(), dt.nanosecond as i32),
                jiff::tz::Offset::from_seconds(dt.offset_seconds),
            ) {
                return Ok(ParsedDateTime::InRange(ts.to_zoned(offset.to_time_zone())));
            }
        }

        Ok(ParsedDateTime::Extended(dt))
    }
}

fn surrogate_year_for_rules(year: u32) -> i16 {
    // Keep weekday/leap-year parity by preserving `year mod 400`, but avoid
    // upper-bound years that can overflow when resolving a zoned instant.
    const BASE: i64 = 9_599;
    let mapped = BASE + (i64::from(year) - BASE).rem_euclid(400);
    mapped as i16
}

fn resolve_rule_offset_for_extended(
    tz: &jiff::tz::TimeZone,
    dt: &ExtendedDateTime,
) -> Result<i32, error::Error> {
    let surrogate_year = surrogate_year_for_rules(dt.year);
    let surrogate_dt = civil::DateTime::new(
        surrogate_year,
        dt.month as i8,
        dt.day as i8,
        dt.hour as i8,
        dt.minute as i8,
        dt.second as i8,
        dt.nanosecond as i32,
    )?;
    let zoned = tz.to_ambiguous_zoned(surrogate_dt).compatible()?;
    Ok(zoned.offset().seconds())
}

fn weekday_monday0(day: weekday::Day) -> u8 {
    match day {
        weekday::Day::Monday => 0,
        weekday::Day::Tuesday => 1,
        weekday::Day::Wednesday => 2,
        weekday::Day::Thursday => 3,
        weekday::Day::Friday => 4,
        weekday::Day::Saturday => 5,
        weekday::Day::Sunday => 6,
    }
}

impl TryFrom<Vec<Item>> for DateTimeBuilder {
    type Error = &'static str;

    fn try_from(items: Vec<Item>) -> Result<Self, Self::Error> {
        let mut builder = DateTimeBuilder::new();

        for item in items {
            builder = match item {
                #[cfg(test)]
                Item::Timestamp(ts) => builder.set_timestamp(ts)?,
                Item::DateTime(dt) => builder.set_date(dt.date)?.set_time(dt.time)?,
                Item::Date(d) => builder.set_date(d)?,
                Item::Time(t) => builder.set_time(t)?,
                Item::Weekday(weekday) => builder.set_weekday(weekday)?,
                Item::Offset(offset) => builder.set_offset(offset)?,
                Item::Relative(rel) => builder.push_relative(rel)?,
                Item::TimeZone(tz) => builder.set_timezone(tz)?,
                Item::Pure(pure) => builder.set_pure(pure)?,
            }
        }

        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::{civil::DateTime, tz::TimeZone, Zoned};

    fn timestamp() -> epoch::Timestamp {
        let mut input = "@1234567890";
        epoch::parse(&mut input).unwrap()
    }

    fn date() -> date::Date {
        let mut input = "2023-06-15";
        date::parse(&mut input).unwrap()
    }

    fn date_large(mut input: &str) -> date::Date {
        date::parse(&mut input).unwrap()
    }

    fn time() -> time::Time {
        let mut input = "12:30:00";
        time::parse(&mut input).unwrap()
    }

    fn time_with_offset() -> time::Time {
        let mut input = "12:30:00+05:00";
        time::parse(&mut input).unwrap()
    }

    fn time_with_small_offset() -> time::Time {
        let mut input = "12:00:00+01:00";
        time::parse(&mut input).unwrap()
    }

    fn weekday() -> weekday::Weekday {
        let mut input = "monday";
        weekday::parse(&mut input).unwrap()
    }

    fn offset() -> offset::Offset {
        let mut input = "+05:00";
        offset::timezone_offset(&mut input).unwrap()
    }

    fn offset_large() -> offset::Offset {
        let mut input = "m+24";
        offset::parse(&mut input).unwrap()
    }

    fn relative_day() -> relative::Relative {
        let mut input = "1 day";
        relative::parse(&mut input).unwrap()
    }

    fn relative_month() -> relative::Relative {
        let mut input = "1 month";
        relative::parse(&mut input).unwrap()
    }

    fn relative_hours() -> relative::Relative {
        let mut input = "2 hours";
        relative::parse(&mut input).unwrap()
    }

    fn relative_minutes() -> relative::Relative {
        let mut input = "3 minutes";
        relative::parse(&mut input).unwrap()
    }

    fn relative_seconds() -> relative::Relative {
        let mut input = "4 seconds";
        relative::parse(&mut input).unwrap()
    }

    fn relative_day_ago() -> relative::Relative {
        let mut input = "1 day ago";
        relative::parse(&mut input).unwrap()
    }

    fn weekday_next_monday() -> weekday::Weekday {
        let mut input = "next monday";
        weekday::parse(&mut input).unwrap()
    }

    fn timezone() -> jiff::tz::TimeZone {
        jiff::tz::TimeZone::UTC
    }

    fn expect_extended_datetime(parsed: ParsedDateTime) -> ExtendedDateTime {
        match parsed {
            ParsedDateTime::Extended(dt) => dt,
            ParsedDateTime::InRange(z) => panic!("expected extended datetime, got in-range: {z}"),
        }
    }

    fn expect_in_range_datetime(parsed: ParsedDateTime) -> Zoned {
        match parsed {
            ParsedDateTime::InRange(z) => z,
            ParsedDateTime::Extended(dt) => panic!("expected in-range datetime, got {dt:?}"),
        }
    }

    #[test]
    fn duplicate_items_error() {
        let test_cases = vec![
            (
                vec![Item::TimeZone(timezone()), Item::TimeZone(timezone())],
                "timezone rule cannot appear more than once",
            ),
            (
                vec![Item::Timestamp(timestamp()), Item::Timestamp(timestamp())],
                "timestamp cannot appear more than once",
            ),
            (
                vec![Item::Date(date()), Item::Date(date())],
                "date cannot appear more than once",
            ),
            (
                vec![Item::Time(time()), Item::Time(time())],
                "time cannot appear more than once",
            ),
            (
                vec![Item::Weekday(weekday()), Item::Weekday(weekday())],
                "weekday cannot appear more than once",
            ),
            (
                vec![Item::Offset(offset()), Item::Offset(offset())],
                "time offset cannot appear more than once",
            ),
        ];

        for (items, expected_err) in test_cases {
            let result = DateTimeBuilder::try_from(items);
            assert_eq!(result.unwrap_err(), expected_err);
        }
    }

    #[test]
    fn timestamp_cannot_be_combined_with_other_items() {
        let test_cases = vec![
            vec![Item::Date(date()), Item::Timestamp(timestamp())],
            vec![Item::Time(time()), Item::Timestamp(timestamp())],
            vec![Item::Weekday(weekday()), Item::Timestamp(timestamp())],
            vec![Item::Offset(offset()), Item::Timestamp(timestamp())],
            vec![Item::Relative(relative_day()), Item::Timestamp(timestamp())],
            vec![Item::Timestamp(timestamp()), Item::Date(date())],
            vec![Item::Timestamp(timestamp()), Item::Time(time())],
            vec![Item::Timestamp(timestamp()), Item::Weekday(weekday())],
            vec![Item::Timestamp(timestamp()), Item::Relative(relative_day())],
            vec![Item::Timestamp(timestamp()), Item::Offset(offset())],
            vec![Item::Timestamp(timestamp()), Item::Pure("2023".to_string())],
        ];

        for items in test_cases {
            let result = DateTimeBuilder::try_from(items);
            assert_eq!(
                result.unwrap_err(),
                "timestamp cannot be combined with other date/time items"
            );
        }
    }

    #[test]
    fn time_offset_conflicts() {
        let items1 = vec![Item::Time(time_with_offset()), Item::Offset(offset())];
        assert_eq!(
            DateTimeBuilder::try_from(items1).unwrap_err(),
            "time offset cannot appear more than once"
        );

        let items2 = vec![Item::Offset(offset()), Item::Time(time_with_offset())];
        assert_eq!(
            DateTimeBuilder::try_from(items2).unwrap_err(),
            "time offset and timezone are mutually exclusive"
        );
    }

    #[test]
    fn helper_mappings() {
        let mapped = surrogate_year_for_rules(2024);
        assert!((9599..=9998).contains(&mapped));
        assert_eq!((mapped as i32).rem_euclid(400), (2024_i32).rem_euclid(400));
        assert_eq!(surrogate_year_for_rules(10000), 9600);
        assert_eq!(weekday_monday0(weekday::Day::Monday), 0);
        assert_eq!(weekday_monday0(weekday::Day::Tuesday), 1);
        assert_eq!(weekday_monday0(weekday::Day::Wednesday), 2);
        assert_eq!(weekday_monday0(weekday::Day::Thursday), 3);
        assert_eq!(weekday_monday0(weekday::Day::Friday), 4);
        assert_eq!(weekday_monday0(weekday::Day::Saturday), 5);
        assert_eq!(weekday_monday0(weekday::Day::Sunday), 6);
    }

    #[test]
    fn build_rejects_timestamp_in_extended_path() {
        let mut builder = DateTimeBuilder::new();
        builder.timestamp = Some(timestamp());
        assert!(builder.build_extended().is_err());
    }

    #[test]
    fn build_falls_back_to_extended_for_9999_rollover() {
        let base = "2000-01-01 00:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("9999-12-31")),
            Item::Relative(relative_day()),
        ])
        .unwrap();
        let dt = expect_extended_datetime(builder.set_base(base).build().unwrap());
        assert_eq!((dt.year, dt.month, dt.day), (10000, 1, 1));
    }

    #[test]
    fn build_extended_keeps_9999_values_when_timestamp_overflows() {
        let base = "2000-01-01 00:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("10000-01-01")),
            Item::Relative(relative_day_ago()),
        ])
        .unwrap();
        let dt = expect_extended_datetime(builder.set_base(base).build().unwrap());
        assert_eq!((dt.year, dt.month, dt.day), (9999, 12, 31));
    }

    #[test]
    fn resolve_rule_offset_for_extended_uses_surrogate_year() {
        let dt = ExtendedDateTime::new(
            DateParts {
                year: 10000,
                month: 7,
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
        let offset = resolve_rule_offset_for_extended(&jiff::tz::TimeZone::UTC, &dt).unwrap();
        assert_eq!(offset, 0);
    }

    #[test]
    fn resolve_rule_offset_for_extended_non_utc_rule() {
        let dt = ExtendedDateTime::new(
            DateParts {
                year: 10000,
                month: 7,
                day: 1,
            },
            TimeParts {
                hour: 12,
                minute: 0,
                second: 0,
                nanosecond: 0,
            },
            0,
        )
        .unwrap();
        let tz = jiff::tz::TimeZone::get("Europe/Paris").unwrap();
        let _ = resolve_rule_offset_for_extended(&tz, &dt).unwrap();
    }

    #[test]
    fn resolve_rule_offset_for_extended_handles_upper_bound_dates() {
        let dt = ExtendedDateTime::new(
            DateParts {
                year: 9999,
                month: 12,
                day: 31,
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
        assert!(resolve_rule_offset_for_extended(&jiff::tz::TimeZone::UTC, &dt).is_ok());
    }

    #[test]
    fn should_try_extended_fallback_conditions() {
        let mut builder = DateTimeBuilder::new();
        assert!(!builder.should_try_extended_fallback());

        builder.relative.push(relative_day());
        builder.date = Some(date_large("9999-12-31"));
        assert!(builder.should_try_extended_fallback());

        builder.timestamp = Some(timestamp());
        assert!(!builder.should_try_extended_fallback());
    }

    #[test]
    fn build_extended_applies_relative_units_and_weekday() {
        let base = "2000-01-01 00:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("10000-01-01")),
            Item::Weekday(weekday_next_monday()),
            Item::Relative(relative_day()),
            Item::Relative(relative_hours()),
            Item::Relative(relative_minutes()),
            Item::Relative(relative_seconds()),
        ])
        .unwrap();
        let dt = expect_extended_datetime(builder.set_base(base).build().unwrap());
        assert_eq!((dt.year, dt.month, dt.day), (10000, 1, 4));
        assert_eq!((dt.hour, dt.minute, dt.second), (2, 3, 4));
    }

    #[test]
    fn build_extended_applies_normalized_final_offset() {
        let base = "2000-01-01 00:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("10000-06-01")),
            Item::Time(time()),
            Item::Offset(offset_large()),
        ])
        .unwrap();
        let dt = expect_extended_datetime(builder.set_base(base).build().unwrap());
        assert_eq!(dt.offset_seconds, 23 * 3600);
    }

    #[test]
    fn build_prefers_in_range_when_fallback_succeeds() {
        let base = "2000-01-01 00:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("9999-01-31")),
            Item::Relative(relative_day()),
        ])
        .unwrap();
        let z = expect_in_range_datetime(builder.set_base(base).build().unwrap());
        assert_eq!(z.strftime("%Y-%m-%d").to_string(), "9999-02-01");
    }

    #[test]
    fn build_falls_back_to_extended_for_non_relative_boundary_time_offset() {
        let base = "2000-01-01 00:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("9999-12-31")),
            Item::Time(time_with_small_offset()),
        ])
        .unwrap();
        let dt = expect_extended_datetime(builder.set_base(base).build().unwrap());
        assert_eq!((dt.year, dt.month, dt.day), (9999, 12, 31));
        assert_eq!((dt.hour, dt.minute, dt.second), (12, 0, 0));
        assert_eq!(dt.offset_seconds, 3600);
    }

    #[test]
    fn build_in_range_month_overflow_adjustment_path() {
        let base = "2000-01-01 00:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("2021-01-31")),
            Item::Relative(relative_month()),
        ])
        .unwrap();
        let z = expect_in_range_datetime(builder.set_base(base).build().unwrap());
        assert_eq!(z.strftime("%Y-%m-%d").to_string(), "2021-03-03");
    }

    #[test]
    fn build_extended_uses_base_and_timezone_rule() {
        let base = "2000-01-01 10:11:12"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let tz = jiff::tz::TimeZone::get("Europe/Paris").unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("10000-01-01")),
            Item::TimeZone(tz),
        ])
        .unwrap();
        let dt = expect_extended_datetime(builder.set_base(base).build().unwrap());
        assert_eq!(dt.year, 10000);
    }

    #[test]
    fn build_extended_supports_timezone_without_base() {
        let tz = jiff::tz::TimeZone::get("Europe/Paris").unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("10000-01-01")),
            Item::TimeZone(tz),
        ])
        .unwrap();
        let dt = expect_extended_datetime(builder.build().unwrap());
        assert_eq!(dt.year, 10000);
    }

    #[test]
    fn build_extended_preserves_base_time_when_not_truncated() {
        let base = "2000-01-01 10:11:12.123456789"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let mut builder = DateTimeBuilder::new();
        builder.base = Some(base);
        let z = expect_in_range_datetime(builder.build_extended().unwrap());
        assert_eq!(
            z.strftime("%Y-%m-%d %H:%M:%S%.9f").to_string(),
            "2000-01-01 10:11:12.123456789"
        );
    }

    #[test]
    fn build_extended_applies_date_and_weekday_without_time_item() {
        let base = "2000-01-01 10:11:12"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();
        let builder = DateTimeBuilder::try_from(vec![
            Item::Date(date_large("10000-01-01")),
            Item::Weekday(weekday_next_monday()),
        ])
        .unwrap();
        let dt = expect_extended_datetime(builder.set_base(base).build().unwrap());
        assert_eq!((dt.hour, dt.minute, dt.second), (0, 0, 0));
    }

    #[test]
    #[should_panic(expected = "expected in-range datetime")]
    fn expect_in_range_datetime_panics_for_extended_input() {
        let parsed = DateTimeBuilder::try_from(vec![Item::Date(date_large("10000-01-01"))])
            .unwrap()
            .build()
            .unwrap();
        let _ = expect_in_range_datetime(parsed);
    }
}
