// For the full copyright and license information, view the LICENSE file.

use jiff::Zoned;

use crate::{items, ParseDateTimeError, ParsedDateTime, ParsedDateTimeWithTrace};

/// Parses a time string and returns a [`ParsedDateTime`] representing the
/// absolute time of the string.
///
/// # Arguments
///
/// * `input` - A string slice representing the time.
///
/// # Examples
///
/// ```
/// use parse_datetime::{parse_datetime, ParsedDateTime};
///
/// let time = parse_datetime("2023-06-03 12:00:01Z").unwrap();
/// match time {
///     ParsedDateTime::InRange(z) => {
///         assert_eq!(z.strftime("%F %T").to_string(), "2023-06-03 12:00:01");
///     }
///     ParsedDateTime::Extended(_) => unreachable!("unexpected for this input"),
/// }
/// ```
///
///
/// # Returns
///
/// * `Ok(ParsedDateTime)` - If the input string can be parsed as a time
/// * `Err(ParseDateTimeError)` - If the input string cannot be parsed
///
/// # Errors
///
/// This function will return `Err(ParseDateTimeError::InvalidInput)` if the
/// input string cannot be parsed.
pub fn parse_datetime<S: AsRef<str> + Clone>(
    input: S,
) -> Result<ParsedDateTime, ParseDateTimeError> {
    items::parse_at_local(input).map_err(|e| e.into())
}

/// Parses a time string at a specific date and returns a [`ParsedDateTime`]
/// representing the absolute time of the string.
///
/// # Arguments
///
/// * date - The date represented in local time
/// * `input` - A string slice representing the time.
///
/// # Examples
///
/// ```
/// use jiff::Zoned;
/// use parse_datetime::{parse_datetime_at_date, ParsedDateTime};
///
///  let now = Zoned::now();
///  let after = parse_datetime_at_date(now, "2024-09-13UTC +3 days").unwrap();
///
///  match after {
///      ParsedDateTime::InRange(z) => assert_eq!("2024-09-16", z.strftime("%F").to_string()),
///      ParsedDateTime::Extended(_) => unreachable!("unexpected for this input"),
///  }
/// ```
///
/// # Returns
///
/// * `Ok(ParsedDateTime)` - If the input string can be parsed as a time
/// * `Err(ParseDateTimeError)` - If the input string cannot be parsed
///
/// # Errors
///
/// This function will return `Err(ParseDateTimeError::InvalidInput)` if the
/// input string cannot be parsed.
pub fn parse_datetime_at_date<S: AsRef<str> + Clone>(
    date: Zoned,
    input: S,
) -> Result<ParsedDateTime, ParseDateTimeError> {
    items::parse_at_date(date, input).map_err(|e| e.into())
}

/// Parses a time string at a specific date and returns the parsed value and a
/// semantic trace of the parser and builder stages.
pub fn parse_datetime_at_date_with_trace<S: AsRef<str> + Clone>(
    date: Zoned,
    input: S,
) -> Result<ParsedDateTimeWithTrace, ParseDateTimeError> {
    items::parse_at_date_with_trace(date, input).map_err(|e| e.into())
}
