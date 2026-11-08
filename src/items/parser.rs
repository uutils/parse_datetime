// For the full copyright and license information, view the LICENSE file.

use super::{
    builder, combined, date, epoch, error, offset, primitive, pure, relative, time, timezone,
    weekday, year,
};

use std::borrow::Cow;

use crate::{ParseTrace, ParsedDateTime, ParsedDateTimeWithTrace, ParsedItem};
use jiff::Zoned;
use primitive::space;
use winnow::{
    combinator::{alt, eof, preceded, repeat_till, terminated, trace},
    error::{AddContext, ContextError, ErrMode, StrContext, StrContextValue},
    stream::Stream,
    ModalResult, Parser,
};

use builder::DateTimeBuilder;
use error::Error;

#[derive(PartialEq, Debug)]
pub(super) enum Item {
    DateTime(combined::DateTime, Option<RawYear>),
    Date(date::Date, DateKind, Option<RawYear>),
    Time(time::Time),
    Weekday(weekday::Weekday),
    Relative(relative::Relative),
    Offset(offset::Offset),
    /// The military timezone letter `j`, which GNU `date` defines as local
    /// time. It is a timezone item like [`Item::Offset`], but carries no
    /// offset, so it cannot be represented as one.
    LocalZone,
    TimeZone(jiff::tz::TimeZone),
    Pure(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DateKind {
    Calendar,
    Number,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RawYear {
    pub(super) value: u32,
    pub(super) digits: u8,
}

/// Parse a date and time string and resolve it against the given base date and
/// time, returning a [`ParsedDateTime`] result.
pub(crate) fn parse_at_date<S: AsRef<str> + Clone>(
    base: Zoned,
    input: S,
) -> Result<ParsedDateTime, Error> {
    match parse(&mut input.as_ref()) {
        Ok(builder) => builder.set_base(base).build(),
        Err(e) => Err(e.into()),
    }
}

/// Parse and resolve a date while retaining parser and builder trace data.
pub(crate) fn parse_at_date_with_trace<S: AsRef<str> + Clone>(
    base: Zoned,
    input: S,
) -> Result<ParsedDateTimeWithTrace, Error> {
    let (builder, trace) = parse_with_trace(&mut input.as_ref())?;
    let (datetime, trace) = builder.set_base(base).build_with_trace(trace)?;
    Ok(ParsedDateTimeWithTrace { datetime, trace })
}

/// Parse a date and time string and resolve it against the current local date
/// and time, returning a [`ParsedDateTime`] result.
pub(crate) fn parse_at_local<S: AsRef<str> + Clone>(input: S) -> Result<ParsedDateTime, Error> {
    match parse(&mut input.as_ref()) {
        Ok(builder) => builder.build(), // the builder uses current local date and time if no base is given.
        Err(e) => Err(e.into()),
    }
}

/// Parse a date and time string.
///
/// Grammar:
///
/// ```ebnf
/// spec                = [ tz_rule ] ( timestamp | items ) ;
///
/// tz_rule            = "TZ=" , "\"" , ( posix_tz | iana_tz ) , "\"" ;
///
/// timestamp           = "@" , float ;
///
/// items               = item , { item } ;
/// item                = datetime | date | time | relative | weekday | offset | pure ;
///
/// datetime            = date , [ "t" | whitespace ] , iso_time ;
///
/// date                = iso_date | us_date | literal1_date | literal2_date ;
///
/// iso_date            = year , [ iso_date_delim ] , month , [ iso_date_delim ] , day ;
/// iso_date_delim      = optional_whitespace , "-" , optional_whitespace ;
///
/// us_date             = month , [ us_date_delim ] , day , [ us_date_delim , year ];
/// us_date_delim       = optional_whitespace , "/" , optional_whitespace ;
///
/// literal1_date       = day , [ literal1_date_delim ] , literal_month , [ literal1_date_delim , year ] ;
/// literal1_date_delim = (optional_whitespace , "-" , optional_whitespace) | optional_whitespace ;
///
/// literal2_date       = literal_month , optional_whitespace , day , [ literal2_date_delim , year ] ;
/// literal2_date_delim = (optional_whitespace , "," , optional_whitespace) | optional_whitespace ;
///
/// year                = dec_uint ;
/// month               = dec_uint ;
/// day                 = dec_uint ;
///
/// literal_month       = "january" | "jan"
///                     | "february" | "feb"
///                     | "march" | "mar"
///                     | "april" | "apr"
///                     | "may"
///                     | "june" | "jun"
///                     | "july" | "jul"
///                     | "august" | "aug"
///                     | "september" | "sept" | "sep"
///                     | "october" | "oct"
///                     | "november" | "nov"
///                     | "december" | "dec" ;
///
/// time                = iso_time | meridiem_time ;
///
/// iso_time            = hour24 , [ ":" , minute , [ ":" , second ] ] , [ time_offset ] ;
///
/// meridiem_time       = hour12 , [ ":" , minute , [ ":" , second ] ] , meridiem ;
/// meridiem            = "am" | "pm" | "a.m." | "p.m." ;
///
/// hour24              = dec_uint ;
/// hour12              = dec_uint ;
/// minute              = dec_uint ;
/// second              = dec_uint ;
///
/// time_offset         = ( "+" | "-" ) , dec_uint , [ ":" , dec_uint ] ;
///
/// relative            = [ numeric_ordinal  ] , unit , [ "ago" | "hence" ] | day_shift ;
///
/// unit                = "year" | "years"
///                     | "month" | "months"
///                     | "fortnight" | "fortnights"
///                     | "week" | "weeks"
///                     | "day" | "days"
///                     | "hour" | "hours"
///                     | "minute" | "minutes" | "min" | "mins"
///                     | "second" | "seconds" | "sec" | "secs" ;
///
/// day_shift           = "tomorrow" | "yesterday" | "today" | "now" ;
///
/// weekday             = [ ordinal ] , day , [ "," ] ;
///
/// ordinal             = numeric_ordinal | text_ordinal ;
/// numeric_ordinal     = [ "+" | "-" ] , dec_uint ;
/// text_ordinal        = "last" | "this" | "next" | "first"
///                     | "third" | "fourth" | "fifth" | "sixth"
///                     | "seventh" | "eighth" | "ninth" | "tenth"
///                     | "eleventh" | "twelfth" ;
///
/// day                 = "monday" | "mon" | "mon."
///                     | "tuesday" | "tue" | "tue." | "tues"
///                     | "wednesday" | "wed" | "wed." | "wednes"
///                     | "thursday" | "thu" | "thu." | "thur" | "thurs"
///                     | "friday" | "fri" | "fri."
///                     | "saturday" | "sat" | "sat."
///                     | "sunday" | "sun" | "sun." ;
///
/// offset             = named_zone , [ time_offset ] ;
///
/// pure               = { digit }
///
/// optional_whitespace = { whitespace } ;
/// ```
pub(crate) fn parse(input: &mut &str) -> ModalResult<DateTimeBuilder> {
    trace("parse", alt((parse_timestamp, parse_items))).parse_next(input)
}

fn parse_with_trace(input: &mut &str) -> ModalResult<(DateTimeBuilder, ParseTrace)> {
    trace(
        "parse",
        alt((parse_timestamp_with_trace, parse_items_with_trace)),
    )
    .parse_next(input)
}

/// Parse a standalone epoch timestamp (e.g., `@1758724019`).
///
/// GNU `date` specifies that a timestamp item is *complete* and *must not* be
/// combined with any other date/time item.
///
/// Notes:
///
/// - If a timezone rule (`TZ="..."`) appears at the beginning of the input, it
///   has no effect on the epoch value. We intentionally parse and ignore it.
/// - Trailing input (aside from optional whitespaces) is rejected.
fn parse_timestamp(input: &mut &str) -> ModalResult<DateTimeBuilder> {
    // Parse and ignore an optional leading timezone rule.
    let _ = timezone::parse(input);

    trace(
        "parse_timestamp",
        // Expect exactly one timestamp and then EOF (allowing trailing spaces).
        terminated(epoch::parse, preceded(space, eof)),
    )
    .verify_map(|ts| DateTimeBuilder::new().set_timestamp(ts).ok())
    .parse_next(input)
}

fn parse_timestamp_with_trace(input: &mut &str) -> ModalResult<(DateTimeBuilder, ParseTrace)> {
    // Expect exactly one timestamp and then EOF (allowing trailing spaces).
    let _ = timezone::parse(input);
    let timestamp = trace(
        "parse_timestamp",
        terminated(epoch::parse, preceded(space, eof)),
    )
    .parse_next(input)?;
    let (seconds, nanoseconds) = timestamp.parts();
    // A fresh builder cannot contain another date/time item, so this is an
    // invariant rather than a parse error path.
    let builder = DateTimeBuilder::new()
        .set_timestamp(timestamp)
        .expect("a fresh builder accepts a timestamp");
    let mut trace = ParseTrace::default();
    trace.items.push(ParsedItem::Timestamp {
        seconds,
        nanoseconds: nanoseconds as i32,
    });
    Ok((builder, trace))
}

/// Parse a sequence of date/time items, honoring an optional leading TZ rule.
///
/// Notes:
///
/// - If a timezone rule (`TZ="..."`) appears at the beginning of the input,
///   parse it first. The timezone rule is case-sensitive.
/// - After the optional timezone rule is parsed, we convert the input to
///   lowercase to allow case-insensitive parsing of the remaining items.
/// - Trailing input (aside from optional whitespaces) is rejected.
fn parse_items(input: &mut &str) -> ModalResult<DateTimeBuilder> {
    let original = *input;
    let mut fast_input = original;
    let items = parse_items_fast(&mut fast_input)?;
    if !needs_context_resolution(&items, original) {
        *input = fast_input;
        return items.try_into().map_err(|e| expect_error(input, e));
    }

    *input = original;
    let (items, _) = parse_items_raw::<false>(input)?;
    items.try_into().map_err(|e| expect_error(input, e))
}

fn parse_items_with_trace(input: &mut &str) -> ModalResult<(DateTimeBuilder, ParseTrace)> {
    let (items, timezone_rule) = parse_items_raw::<true>(input)?;
    let (builder, mut trace) =
        DateTimeBuilder::try_from_with_trace(items).map_err(|e| expect_error(input, e))?;
    trace.timezone_rule = timezone_rule;
    Ok((builder, trace))
}

fn parse_items_raw<const TRACE: bool>(
    input: &mut &str,
) -> ModalResult<(Vec<Item>, Option<String>)> {
    // Parse and consume an optional leading timezone rule.
    let tz_start = *input;
    let tz = timezone::parse(input).map(Item::TimeZone);
    let timezone_rule = if TRACE {
        tz.as_ref().ok().map(|_| {
            let consumed = tz_start.len() - input.len();
            tz_start[..consumed].to_owned()
        })
    } else {
        None
    };

    // Convert input to lowercase for case-insensitive parsing.
    let lower: Cow<str> = if input.bytes().any(|b| b.is_ascii_uppercase()) {
        input.to_ascii_lowercase().into()
    } else {
        (*input).into()
    };
    let input = &mut lower.as_ref();

    // Parse zero or more items until EOF (allowing trailing spaces). This is
    // written as a small loop rather than `repeat_till` so the item parser can
    // see the items already reduced. GNU has one context-sensitive ambiguity:
    // after a time item, `+N unit` is both a numeric timezone correction and a
    // relative unit with its implicit multiplier of one.
    let mut items = Vec::new();
    loop {
        let start = input.checkpoint();
        match parse_item_with_context::<TRACE>(input, &mut items) {
            Ok(item) => items.push(item),
            Err(ErrMode::Backtrack(_)) => {
                input.reset(&start);
                preceded(space, eof).parse_next(input)?;
                break;
            }
            Err(e) => return Err(e),
        }
    }

    if let Ok(tz) = tz {
        items.push(tz);
    }

    Ok((items, timezone_rule))
}

fn parse_items_fast(input: &mut &str) -> ModalResult<Vec<Item>> {
    let tz = timezone::parse(input).map(Item::TimeZone);
    let original = *input;

    let lower: Cow<str> = if input.bytes().any(|b| b.is_ascii_uppercase()) {
        input.to_ascii_lowercase().into()
    } else {
        (*input).into()
    };
    let lower_input = &mut lower.as_ref();
    let (mut items, _): (Vec<Item>, _) = trace(
        "parse_items",
        repeat_till(0.., parse_item::<false>, preceded(space, eof)),
    )
    .parse_next(lower_input)?;

    if let Ok(tz) = tz {
        items.push(tz);
    }

    let consumed = original.len() - lower_input.len();
    *input = &original[consumed..];
    Ok(items)
}

fn needs_context_resolution(items: &[Item], input: &str) -> bool {
    has_unzoned_time(items) && !has_zero_relative(items) && has_signed_relative(input)
}

fn has_signed_relative(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if matches!(bytes[index], b'+' | b'-') {
            let mut next = index + 1;
            while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                next += 1;
            }
            if next < bytes.len() && bytes[next].is_ascii_digit() {
                while next < bytes.len() && (bytes[next].is_ascii_digit() || bytes[next] == b'.') {
                    next += 1;
                }
                while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                    next += 1;
                }
                if next < bytes.len() && bytes[next].is_ascii_alphabetic() {
                    return true;
                }
            }
        }
        index += 1;
    }
    false
}

/// Parse an item using the reductions to its left to resolve GNU's ambiguous
/// `+N unit` form. A time to the left and no other timezone means that `+N`
/// belongs to that time while the following unit is a relative item with an
/// implicit multiplier of one. Without a time to the left, the ordinary
/// relative parser consumes the signed multiplier as a whole.
fn parse_item_with_context<const TRACE: bool>(
    input: &mut &str,
    items: &mut [Item],
) -> ModalResult<Item> {
    // A zero-valued relative item (`today`, `now`, or `this ...`) explicitly
    // stresses the surrounding date string. In particular, GNU parses
    // `12:00 today +1 hour` as a one-hour relative adjustment, rather than
    // treating `+1` as a numeric timezone correction attached to `12:00`.
    // Keep the correction ambiguity for `12:00 +1 hour`, where GNU retains
    // the historical timezone interpretation.
    if !items.is_empty() && has_unzoned_time(items) && !has_zero_relative(items) {
        let start = input.checkpoint();
        if let Ok(offset) = offset::timezone_offset_raw.parse_next(input) {
            if let Ok(relative) = relative::parse.parse_next(input) {
                if attach_time_offset(items, offset) {
                    return Ok(Item::Relative(relative));
                }
            }
        }
        input.reset(&start);
    }

    parse_item::<TRACE>(input)
}

fn has_unzoned_time(items: &[Item]) -> bool {
    if items
        .iter()
        .any(|item| matches!(item, Item::Offset(_) | Item::LocalZone | Item::TimeZone(_)))
    {
        return false;
    }

    items.iter().any(|item| match item {
        Item::Time(time) => time.offset.is_none(),
        Item::DateTime(datetime, _) => datetime.time.offset.is_none(),
        _ => false,
    })
}

fn has_zero_relative(items: &[Item]) -> bool {
    items.iter().any(|item| {
        matches!(
            item,
            Item::Relative(
                relative::Relative::Years(0)
                    | relative::Relative::Months(0)
                    | relative::Relative::Days(0)
                    | relative::Relative::Hours(0)
                    | relative::Relative::Minutes(0)
                    | relative::Relative::Seconds(0, 0)
            )
        )
    })
}

fn attach_time_offset(items: &mut [Item], offset: offset::Offset) -> bool {
    for item in items.iter_mut().rev() {
        match item {
            Item::Time(time) if time.offset.is_none() => {
                time.offset = Some(offset);
                return true;
            }
            Item::DateTime(datetime, _) if datetime.time.offset.is_none() => {
                datetime.time.offset = Some(offset);
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Parse an item.
fn parse_item<const TRACE: bool>(input: &mut &str) -> ModalResult<Item> {
    // An ISO date is either the start of a combined date and time item or a
    // date item on its own. Parse it once instead of once per alternative.
    let start = input.checkpoint();
    let original = *input;
    if let Ok(date) = alt((date::iso1, date::iso2)).parse_next(input) {
        let after_date = input.checkpoint();
        match combined::parse_time_after_date(input, date.clone()) {
            Ok(dt) => {
                let source = &original[..original.len() - input.len()];
                let raw_year = if TRACE { raw_year(source, &date) } else { None };
                return Ok(Item::DateTime(dt, raw_year));
            }
            Err(ErrMode::Backtrack(_)) => {
                input.reset(&after_date);
                let source = &original[..original.len() - input.len()];
                let kind = if TRACE && source.chars().all(|character| character.is_ascii_digit()) {
                    DateKind::Number
                } else {
                    DateKind::Calendar
                };
                let raw_year = if TRACE { raw_year(source, &date) } else { None };
                return Ok(Item::Date(date, kind, raw_year));
            }
            Err(e) => return Err(e),
        }
    }
    input.reset(&start);

    let original = *input;
    if let Ok(date) = date::parse_non_iso.parse_next(input) {
        let source = &original[..original.len() - input.len()];
        let raw_year = if TRACE { raw_year(source, &date) } else { None };
        return Ok(Item::Date(date, DateKind::Calendar, raw_year));
    }
    input.reset(&start);

    trace(
        "parse_item",
        alt((
            time::parse.map(Item::Time),
            relative::parse.map(Item::Relative),
            weekday::parse.map(Item::Weekday),
            offset::parse.map(Item::Offset),
            offset::parse_local.map(|()| Item::LocalZone),
            pure::parse.map(Item::Pure),
        )),
    )
    .parse_next(input)
}

fn raw_year(source: &str, date: &date::Date) -> Option<RawYear> {
    let expected = date.year?;
    let source = source.trim();

    // An all-digit date stores its year before the final four month/day
    // digits, regardless of whether the year has two or more digits.
    if source.chars().all(|character| character.is_ascii_digit()) && source.len() >= 5 {
        let year = &source[..source.len() - 4];
        let value = year.parse().ok()?;
        return (year::year_from_str(year).ok() == Some(expected)).then_some(RawYear {
            value,
            digits: year.len().try_into().ok()?,
        });
    }

    // For the other date spellings, the explicit year is the numeric component
    // whose GNU two-digit normalization matches the parsed year. This keeps
    // the original spelling for diagnostics without making Date carry debug
    // metadata into its normal equality and conversion paths.
    source
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .find_map(|part| {
            let value = part.parse().ok()?;
            (year::year_from_str(part).ok() == Some(expected)).then_some(RawYear {
                value,
                digits: part.len().try_into().ok()?,
            })
        })
}

/// Create an error with context for unexpected input.
fn expect_error(input: &mut &str, reason: &'static str) -> ErrMode<ContextError> {
    ErrMode::Cut(ContextError::new()).add_context(
        input,
        &input.checkpoint(),
        StrContext::Expected(StrContextValue::Description(reason)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attaches_signed_relative_to_preceding_time() {
        let mut input = "12:00 +1 day";
        let (items, _) = parse_items_raw::<false>(&mut input).unwrap();

        assert!(matches!(
            items.as_slice(),
            [
                Item::Time(time),
                Item::Relative(relative::Relative::Days(1)),
            ] if time.offset.as_ref().is_some_and(|offset| offset.total_seconds() == 3600)
        ));
    }

    #[test]
    fn contextual_offset_attachment_can_fail_without_a_time() {
        let mut input = "+05:00";
        let offset = offset::timezone_offset_raw.parse_next(&mut input).unwrap();
        let mut items = [Item::Relative(relative::Relative::Days(1))];

        assert!(!attach_time_offset(&mut items, offset));
    }

    #[test]
    fn invalid_time_after_date_is_rejected() {
        let mut input = "2024-01-01 25:00";
        assert!(parse(&mut input).is_err());
    }

    #[test]
    fn traces_compact_numeric_date() {
        let mut input = "20240101";
        let (items, timezone_rule) = parse_items_raw::<true>(&mut input).unwrap();

        assert!(timezone_rule.is_none());
        assert!(matches!(
            items.as_slice(),
            [Item::Date(
                date,
                DateKind::Number,
                Some(RawYear {
                    value: 2024,
                    digits: 4,
                })
            )] if date.year == Some(2024) && date.month == 1 && date.day == 1
        ));
    }

    #[test]
    fn traces_datetime_context_offset_attachment() {
        let mut input = "2024-01-01T12:00 +1 day";
        let (items, _) = parse_items_raw::<false>(&mut input).unwrap();

        assert!(matches!(
            items.as_slice(),
            [
                Item::DateTime(datetime, _),
                Item::Relative(relative::Relative::Days(1)),
            ] if datetime.time.offset.as_ref().is_some_and(|offset| offset.total_seconds() == 3600)
        ));
    }

    #[test]
    fn signed_relative_detection_handles_spacing_and_non_relative_signs() {
        assert!(has_signed_relative("12:00 + 1 day"));
        assert!(!has_signed_relative("12:00 + day"));
        assert!(!has_signed_relative("2024-01-01"));
    }

    #[test]
    fn fast_parser_consumes_input() {
        let mut input = "2024-01-01";
        let _ = parse(&mut input).unwrap();
        assert!(input.is_empty());
    }
}
