// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::ParseDateTimeError;
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike};
use regex::Regex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Add;

/// Parses a date modification string and performs the operation on the given `DateTime<TimeZone>`
/// object and returns a new `DateTime<TimeZone>` as a `Result`.
///
///
/// # Arguments
///
/// * `date` - A `DateTime<TimeZone>` instance representing the base date for the calculation
/// * `s` - A string slice representing the relative time.
///
/// # Errors
///
/// This function will return `Err(ParseDateTimeError::InvalidInput)` if the input string
/// cannot be parsed as a relative time.
pub fn dt_from_relative<Tz: TimeZone>(
    s: &str,
    date: DateTime<Tz>,
) -> Result<DateTime<Tz>, ParseDateTimeError> {
    if s.trim().is_empty() {
        return Ok(date);
    }

    let time_pattern: Regex = Regex::new(
        r"(?ix)
        (?:(?P<value>[-+]?\s*\d*)\s*)?
        (\s*(?P<direction>next|last)?\s*)?
        (?P<absolute>years?|months?|fortnights?|weeks?|days?|hours?|h|minutes?|mins?|m|seconds?|secs?|s)?
        (?P<relative>yesterday|tomorrow|now|today)?
        (\s*(?P<separator>and|,)?\s*)?
        (\s*(?P<ago>ago)?)?",
    ).unwrap();

    let mut is_ago = s.contains(" ago");
    let mut captures_processed = 0;

    let mut chrono_map: HashMap<ChronoUnit, i64> = HashMap::new();

    let mut time: Option<u32> = None;

    for capture in time_pattern.captures_iter(s.trim()) {
        captures_processed += 1;
        let value_str = capture
            .name("value")
            .ok_or(ParseDateTimeError::InvalidInput)?
            .as_str();

        let mut value = if !value_str.is_empty() {
            value_str
                .chars()
                .filter(|char| !char.is_ascii_whitespace())
                .collect::<String>()
                .parse::<i64>()
                .map_err(|_| ParseDateTimeError::InvalidInput)?
        } else {
            1
        };

        if let Some(direction) = capture.name("direction") {
            if direction.as_str() == "last" {
                is_ago = true;
            }
        }

        if capture.name("ago").is_some() {
            is_ago = true;
        }

        if value > 0 && is_ago {
            value *= -1;
        }

        match (capture.name("absolute"), capture.name("relative")) {
            (None, None) => {
                // time cannot be set twice and time cannot be negative
                if value < 0 || time.is_some() {
                    return Err(ParseDateTimeError::InvalidInput);
                }
                // Time values cannot start with '+' or '-' to be consistent with GNU
                if value_str.starts_with('+') || value_str.starts_with('-') {
                    return Err(ParseDateTimeError::InvalidInput);
                }
                time = Some(value as u32);
            }
            (Some(absolute), None) => {
                process_absolute(
                    absolute.as_str().to_ascii_lowercase(),
                    &mut chrono_map,
                    value,
                )?;
            }
            (None, Some(relative)) => {
                // time cannot be set twice and time cannot be negative
                if value < 0 || time.is_some() {
                    return Err(ParseDateTimeError::InvalidInput);
                }
                // Use value_str as a way to check if user passed in a value.
                // If they did not then we should not interpret `value = 1` as a time
                if !value_str.is_empty() {
                    time = Some(value as u32);
                }
                process_relative(relative.as_str().to_string(), &mut chrono_map)?;
            }
            (Some(_), Some(_)) => {
                /* Doesn't appear to be possibly due to the way the
                regular expression is structured, and how the iterator works.
                There is a test case in test_edge_cases() that passes.
                */
            }
        }
    }
    if captures_processed == 0 {
        return Err(ParseDateTimeError::InvalidInput);
    }

    let mut datetime = match time {
        None => date,
        Some(time) => {
            let hour = time / 100;
            let minute = time % 100;
            if hour >= 24 || minute >= 60 {
                return Err(ParseDateTimeError::InvalidInput);
            }
            date.with_hour(hour).unwrap().with_minute(minute).unwrap()
        }
    };

    if let Some(months) = chrono_map.remove(&ChronoUnit::Month) {
        process_months(&mut datetime, months);
    }

    // Not doing things like months/years before other elements leads to improper output.
    let sorted = {
        let mut v = chrono_map.into_iter().collect::<Vec<(ChronoUnit, i64)>>();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    };
    for (chrono, value) in sorted.into_iter() {
        match chrono {
            ChronoUnit::Month => { /* Not possible */ }
            ChronoUnit::Fortnight => {
                datetime = datetime.add(Duration::weeks(value * 2));
            }
            ChronoUnit::Week => {
                datetime = datetime.add(Duration::weeks(value));
            }
            ChronoUnit::Day => {
                datetime = datetime.add(Duration::days(value));
            }
            ChronoUnit::Hour => {
                datetime = datetime.add(Duration::hours(value));
            }
            ChronoUnit::Minute => {
                datetime = datetime.add(Duration::minutes(value));
            }
            ChronoUnit::Second => {
                datetime = datetime.add(Duration::seconds(value));
            }
        }
    }
    Ok(datetime)
}
#[allow(clippy::map_entry)]
fn add_unit(map: &mut HashMap<ChronoUnit, i64>, unit: ChronoUnit, time: i64) {
    if map.contains_key(&unit) {
        *map.get_mut(&unit).unwrap() += time;
    } else {
        map.insert(unit, time);
    }
}

#[allow(clippy::match_overlapping_arm, overlapping_range_endpoints)]
fn process_months<Tz: TimeZone>(date: &mut DateTime<Tz>, months: i64) {
    let mut years = months / 12;
    let current_month = date.month() as i64;
    let potential_month = current_month + months % 12;
    const JANUARY: i64 = 1;
    const DECEMBER: i64 = 12;
    let new_month = match potential_month {
        JANUARY..=DECEMBER => potential_month,
        -12..=JANUARY => {
            years -= 1;
            DECEMBER + potential_month
        }
        DECEMBER.. => {
            years += 1;
            potential_month - DECEMBER
        }
        _ => panic!("IMPOSSIBLE!"),
    } as u32;

    *date = date
        .with_day(28)
        .unwrap()
        .with_month(new_month)
        .unwrap()
        .with_year(date.year() + years as i32)
        .unwrap()
        .add(Duration::days(date.day() as i64 - 28));
}

fn process_absolute(
    unit: String,
    chrono_map: &mut HashMap<ChronoUnit, i64>,
    value: i64,
) -> Result<(), ParseDateTimeError> {
    match unit.as_bytes() {
        b"years" | b"year" => add_unit(chrono_map, ChronoUnit::Month, value * 12),
        b"months" | b"month" => add_unit(chrono_map, ChronoUnit::Month, value),
        b"fortnights" | b"fortnight" => add_unit(chrono_map, ChronoUnit::Fortnight, value),
        b"weeks" | b"week" => add_unit(chrono_map, ChronoUnit::Week, value),
        b"days" | b"day" => add_unit(chrono_map, ChronoUnit::Day, value),
        b"hours" | b"hour" | b"h" => add_unit(chrono_map, ChronoUnit::Hour, value),
        b"minutes" | b"minute" | b"mins" | b"min" | b"m" => {
            add_unit(chrono_map, ChronoUnit::Minute, value)
        }
        b"seconds" | b"second" | b"secs" | b"sec" | b"s" => {
            add_unit(chrono_map, ChronoUnit::Second, value)
        }
        _ => return Err(ParseDateTimeError::InvalidInput),
    };
    Ok(())
}

fn process_relative(
    unit: String,
    chrono_map: &mut HashMap<ChronoUnit, i64>,
) -> Result<(), ParseDateTimeError> {
    match unit.as_bytes() {
        b"yesterday" => add_unit(chrono_map, ChronoUnit::Day, -1),
        b"tomorrow" => add_unit(chrono_map, ChronoUnit::Day, 1),
        b"now" | b"today" => { /*No processing needed*/ }
        _ => return Err(ParseDateTimeError::InvalidInput),
    }
    Ok(())
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
enum ChronoUnit {
    Month,
    Fortnight,
    Week,
    Day,
    Hour,
    Minute,
    Second,
}

impl ChronoUnit {
    fn map_to_int(&self) -> u8 {
        match self {
            ChronoUnit::Month => 7,
            ChronoUnit::Fortnight => 6,
            ChronoUnit::Week => 5,
            ChronoUnit::Day => 4,
            ChronoUnit::Hour => 3,
            ChronoUnit::Minute => 2,
            ChronoUnit::Second => 1,
        }
    }
}

impl PartialOrd for ChronoUnit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.map_to_int().cmp(&other.map_to_int()))
    }
}

impl Ord for ChronoUnit {
    fn cmp(&self, other: &Self) -> Ordering {
        self.map_to_int().cmp(&other.map_to_int())
    }
}

#[cfg(test)]
mod tests {

    use super::dt_from_relative;
    use chrono::DateTime;

    #[test]
    fn test_parse_date_from_modifier_ok() {
        let format = "%Y %b %d %H:%M:%S.%f %z";
        let input = [
            (
                "1000",
                DateTime::parse_from_str("2022 May 15 10:00:00.0 +0000", format).unwrap(),
            ),
            (
                "1000 yesterday",
                DateTime::parse_from_str("2022 May 14 10:00:00.0 +0000", format).unwrap(),
            ),
            (
                "1000 yesterday next month",
                DateTime::parse_from_str("2022 Jun 14 10:00:00.0 +0000", format).unwrap(),
            ),
            (
                "1000 yesterday month",
                DateTime::parse_from_str("2022 Jun 14 10:00:00.0 +0000", format).unwrap(),
            ),
            (
                "last year",
                DateTime::parse_from_str("2021 May 15 00:00:00.0 +0000", format).unwrap(),
            ),
            (
                "yesterday 1223",
                DateTime::parse_from_str("2022 May 14 12:23:00.0 +0000", format).unwrap(),
            ),
            (
                "yesterday month",
                DateTime::parse_from_str("2022 Jun 14 00:00:00.0 +0000", format).unwrap(),
            ),
            (
                "+01MONTH",
                DateTime::parse_from_str("2022 Jun 15 00:00:00.0 +0000", format).unwrap(),
            ),
            (
                "+01MONTH 1545",
                DateTime::parse_from_str("2022 Jun 15 15:45:00.0 +0000", format).unwrap(),
            ),
            (
                "00001year-000000001year+\t12months",
                DateTime::parse_from_str("2023 May 15 00:00:00.0 +0000", format).unwrap(),
            ),
            (
                "",
                DateTime::parse_from_str("2022 May 15 00:00:00.0 +0000", format).unwrap(),
            ),
            (
                "30SecONDS1houR",
                DateTime::parse_from_str("2022 May 15 01:00:30.0 +0000", format).unwrap(),
            ),
            (
                "30     \t\n\t SECONDS000050000houR-10000yearS",
                DateTime::parse_from_str("-7972 Jan 27 08:00:30.0 +0000", format).unwrap(),
            ),
            (
                "+0000111MONTHs -   20    yearS 100000day",
                DateTime::parse_from_str("2285 May 30 00:00:00.0 +0000", format).unwrap(),
            ),
            (
                "100 week + 0024HOUrs - 50 minutes",
                DateTime::parse_from_str("2024 Apr 14 23:10:00.0 +0000", format).unwrap(),
            ),
            (
                "-100 MONTHS 300 days + 20 \t YEARS 130",
                DateTime::parse_from_str("2034 Nov 11 01:30:00.0 +0000", format).unwrap(),
            ),
        ];

        let date0 = DateTime::parse_from_str("2022 May 15 00:00:00.0 +0000", format).unwrap();
        for (modifier, expected) in input {
            assert_eq!(dt_from_relative(modifier, date0).unwrap(), expected);
        }
    }

    #[test]
    fn test_edge_cases() {
        let format = "%Y %b %d %H:%M:%S.%f %z";
        let input = [
            (
                "1 month 1230",
                DateTime::parse_from_str("2022 Aug 31 00:00:00.0 +0000", format).unwrap(),
                DateTime::parse_from_str("2022 Oct 1 12:30:00.0 +0000", format).unwrap(),
            ),
            (
                "2 month 1230",
                DateTime::parse_from_str("2022 Aug 31 00:00:00.0 +0000", format).unwrap(),
                DateTime::parse_from_str("2022 Oct 31 12:30:00.0 +0000", format).unwrap(),
            ),
            (
                "year 1230",
                DateTime::parse_from_str("2020 Feb 29 00:00:00.0 +0000", format).unwrap(),
                DateTime::parse_from_str("2021 Mar 1 12:30:00.0 +0000", format).unwrap(),
            ),
            (
                "100 year 1230",
                DateTime::parse_from_str("2020 Feb 29 00:00:00.0 +0000", format).unwrap(),
                DateTime::parse_from_str("2120 Feb 29 12:30:00.0 +0000", format).unwrap(),
            ),
            (
                "101 year 1230",
                DateTime::parse_from_str("2020 Feb 29 00:00:00.0 +0000", format).unwrap(),
                DateTime::parse_from_str("2121 Mar 1 12:30:00.0 +0000", format).unwrap(),
            ),
            (
                " month yesterday",
                DateTime::parse_from_str("2020 Feb 29 00:00:00.0 +1000", format).unwrap(),
                DateTime::parse_from_str("2020 Mar 28 00:00:00.0 +1000", format).unwrap(),
            ),
        ];

        for (modifier, input_dt, expected_dt) in input {
            assert_eq!(dt_from_relative(modifier, input_dt).unwrap(), expected_dt);
        }
    }

    #[test]
    fn test_parse_date_from_modifier_err() {
        let format = "%Y %b %d %H:%M:%S.%f %z";

        let input = [
            "100000000000000000000000000000000000000 Years",
            "1000 days 1000 100",
            "1000 1200",
            "1000 yesterday 1200",
        ];

        let date0 = DateTime::parse_from_str("2022 May 15 00:00:00.0 +0000", format).unwrap();
        for modifier in input.into_iter() {
            assert!(dt_from_relative(modifier, date0).is_err());
        }
    }
}
