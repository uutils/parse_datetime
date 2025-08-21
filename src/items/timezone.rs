// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse a timezone item.
//!
//! The GNU docs state:
//!
//! > A “time zone item” specifies an international time zone, indicated by a
//! > small set of letters, e.g., ‘UTC’ or ‘Z’ for Coordinated Universal Time.
//! > Any included periods are ignored.  By following a non-daylight-saving
//! > time zone by the string ‘DST’ in a separate word (that is, separated by
//! > some white space), the corresponding daylight saving time zone may be
//! > specified.  Alternatively, a non-daylight-saving time zone can be
//! > followed by a time zone correction, to add the two values.  This is
//! > normally done only for ‘UTC’; for example, ‘UTC+05:30’ is equivalent to
//! > ‘+05:30’.
//! >
//! >    Time zone items other than ‘UTC’ and ‘Z’ are obsolescent and are not
//! > recommended, because they are ambiguous; for example, ‘EST’ has a
//! > different meaning in Australia than in the United States, and ‘A’ has
//! > different meaning as a military time zone than as an obsolete RFC 822
//! > time zone.  Instead, it's better to use unambiguous numeric time zone
//! > corrections like ‘-0500’, as described in the previous section.
//! >
//! >    If neither a time zone item nor a time zone correction is supplied,
//! > timestamps are interpreted using the rules of the default time zone

use std::fmt::Display;

use winnow::{
    combinator::{alt, peek, seq},
    error::{ContextError, ErrMode},
    stream::{AsChar, Stream},
    token::take_while,
    ModalResult, Parser,
};

use crate::{items::primitive::colon, ParseDateTimeError};

use super::{
    primitive::{ctx_err, s},
    relative,
};

#[derive(PartialEq, Debug, Clone, Default)]
pub(crate) struct Offset {
    pub(crate) negative: bool,
    pub(crate) hours: u8,
    pub(crate) minutes: u8,
}

impl Offset {
    fn merge(self, offset: Offset) -> Offset {
        fn combine(a: u16, neg_a: bool, b: u16, neg_b: bool) -> (u16, bool) {
            if neg_a == neg_b {
                (a + b, neg_a)
            } else if a > b {
                (a - b, neg_a)
            } else {
                (b - a, neg_b)
            }
        }
        let (total_minutes, negative) = combine(
            (self.hours as u16) * 60 + (self.minutes as u16),
            self.negative,
            (offset.hours as u16) * 60 + (offset.minutes as u16),
            offset.negative,
        );
        let hours = (total_minutes / 60) as u8;
        let minutes = (total_minutes % 60) as u8;

        Offset {
            negative,
            hours,
            minutes,
        }
    }
}

impl TryFrom<&Offset> for jiff::tz::TimeZone {
    type Error = ParseDateTimeError;

    fn try_from(
        Offset {
            negative,
            hours,
            minutes,
        }: &Offset,
    ) -> Result<Self, Self::Error> {
        let secs = (*hours as i32) * 3600 + (*minutes as i32) * 60;
        let secs = if *negative { -secs } else { secs };

        let offset =
            jiff::tz::Offset::from_seconds(secs).map_err(|_| ParseDateTimeError::InvalidInput)?;
        let tz = jiff::tz::TimeZone::fixed(offset);

        Ok(tz)
    }
}

impl Display for Offset {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "{}{:02}:{:02}",
            if self.negative { "-" } else { "+" },
            self.hours,
            self.minutes
        )
    }
}

pub(crate) fn parse(input: &mut &str) -> ModalResult<Offset> {
    timezone(input)
}

fn timezone(input: &mut &str) -> ModalResult<Offset> {
    timezone_name_offset.parse_next(input)
}

/// Parse a timezone starting with `+` or `-`
pub(super) fn timezone_num(input: &mut &str) -> ModalResult<Offset> {
    // Strings like "+8 years" are ambiguous, they can either be parsed as a
    // timezone offset "+8" and a relative time "years", or just a relative time
    // "+8 years". GNU date parses them the second way, so we do the same here.
    //
    // Return early if the input can be parsed as a relative time.
    if peek(relative::parse).parse_next(input).is_ok() {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    seq!(plus_or_minus, alt((timezone_colon, timezone_colonless)))
        .parse_next(input)
        .and_then(|(negative, (hours, minutes))| {
            if !(0..=12).contains(&hours) {
                return Err(ErrMode::Cut(ctx_err("timezone hour between 0 and 12")));
            }

            if !(0..=60).contains(&minutes) {
                return Err(ErrMode::Cut(ctx_err("timezone minute between 0 and 60")));
            }

            Ok(Offset {
                negative,
                hours,
                minutes,
            })
        })
}

/// Parse a timezone offset with a colon separating hours and minutes
fn timezone_colon(input: &mut &str) -> ModalResult<(u8, u8)> {
    seq!(
        s(take_while(1..=2, AsChar::is_dec_digit)).try_map(|x: &str| x.parse()),
        _: colon,
        s(take_while(1..=2, AsChar::is_dec_digit)).try_map(|x: &str| x.parse()),
    )
    .parse_next(input)
}

/// Parse a timezone offset without colon
fn timezone_colonless(input: &mut &str) -> ModalResult<(u8, u8)> {
    s(take_while(0.., AsChar::is_dec_digit))
        .verify_map(|s: &str| {
            // GNU date supports number strings with leading zeroes, e.g.,
            // `UTC+000001100` is valid.
            let s = if s.len() > 4 {
                s.trim_start_matches('0')
            } else {
                s
            };
            Some(match s.len() {
                0 => (0, 0),
                1 | 2 => (s.parse().ok()?, 0),
                // The minutes are the last two characters here, for some reason.
                3 => (s[..1].parse().ok()?, s[1..].parse().ok()?),
                4 => (s[..2].parse().ok()?, s[2..].parse().ok()?),
                _ => return None,
            })
        })
        .parse_next(input)
}

/// Parse a timezone by name
fn timezone_name_offset(input: &mut &str) -> ModalResult<Offset> {
    /// I'm assuming there are no timezone abbreviations with more
    /// than 6 charactres
    const MAX_TZ_SIZE: usize = 6;
    let nextword = s(take_while(1..=MAX_TZ_SIZE, AsChar::is_alpha)).parse_next(input)?;
    let tz = timezone_name_to_offset(nextword)?;

    // Strings like "UTC +8 years" are ambiguous, they can either be parsed as
    // "UTC+8" and "years", or "UTC" and "+8 years". GNU date parses them the
    // second way, so we do the same here.
    //
    // Only process if the input cannot be parsed as a relative time.
    if peek(relative::parse).parse_next(input).is_err() {
        let start = input.checkpoint();
        if let Ok(other_tz) = timezone_num.parse_next(input) {
            let new_tz = tz.merge(other_tz);

            return Ok(new_tz);
        };
        input.reset(&start);
    }

    Ok(tz)
}

/// Named timezone list.
///
/// The full list of timezones can be extracted from
/// https://www.timeanddate.com/time/zones/. GNU date only supports a subset of
/// these. We support the same subset as GNU date.
fn timezone_name_to_offset(input: &str) -> ModalResult<Offset> {
    let mut offset_str = match input {
        "z" => Ok("+0"),
        "y" => Ok("-12"),
        "x" => Ok("-11"),
        "wet" => Ok("+0"),
        "west" => Ok("+1"),
        "wat" => Ok("+1"),
        "w" => Ok("-10"),
        "v" => Ok("-9"),
        "utc" => Ok("+0"),
        "u" => Ok("-8"),
        "t" => Ok("-7"),
        "sst" => Ok("-11"),
        "sgt" => Ok("+8"),
        "sast" => Ok("+2"),
        "s" => Ok("-6"),
        "r" => Ok("-5"),
        "q" => Ok("-4"),
        "pst" => Ok("-8"),
        "pdt" => Ok("-7"),
        "p" => Ok("-3"),
        "o" => Ok("-2"),
        "nzst" => Ok("+12"),
        "nzdt" => Ok("+13"),
        "nst" => Ok("-3:30"),
        "ndt" => Ok("-2:30"),
        "n" => Ok("-1"),
        "mst" => Ok("-7"),
        "msk" => Ok("+3"),
        "msd" => Ok("+4"),
        "mdt" => Ok("-6"),
        "m" => Ok("+12"),
        "l" => Ok("+11"),
        "k" => Ok("+10"),
        "jst" => Ok("+9"),
        "ist" => Ok("+5:30"),
        "i" => Ok("+9"),
        "hst" => Ok("-10"),
        "h" => Ok("+8"),
        "gst" => Ok("+4"),
        "gmt" => Ok("+0"),
        "g" => Ok("+7"),
        "f" => Ok("+6"),
        "est" => Ok("-5"),
        "eet" => Ok("+2"),
        "eest" => Ok("+3"),
        "edt" => Ok("-4"),
        "eat" => Ok("+3"),
        "e" => Ok("+5"),
        "d" => Ok("+4"),
        "cst" => Ok("-6"),
        "clt" => Ok("-4"),
        "clst" => Ok("-3"),
        "cet" => Ok("+1"),
        "cest" => Ok("+2"),
        "cdt" => Ok("-5"),
        "cat" => Ok("+2"),
        "c" => Ok("+3"),
        "bst" => Ok("+6"),
        "brt" => Ok("-3"),
        "brst" => Ok("-2"),
        "b" => Ok("+2"),
        "ast" => Ok("-3"),
        "art" => Ok("-3"),
        "akst" => Ok("-9"),
        "akdt" => Ok("-8"),
        "adt" => Ok("+4"),
        "a" => Ok("+1"),
        _ => Err(ErrMode::Backtrack(ContextError::new())),
    }?;

    timezone_num(&mut offset_str)
}

/// Parse the plus or minus character and return whether it was negative
fn plus_or_minus(input: &mut &str) -> ModalResult<bool> {
    s(alt(("+".value(false), "-".value(true)))).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timezone_colonless() {
        fn aux(inp: &mut &str) -> String {
            format!("{:?}", timezone_colonless(inp).expect("timezone_colonless"))
        }

        assert_eq!("(0, 0)", aux(&mut "0000"));
        assert_eq!("(12, 34)", aux(&mut "1234"));
        assert_eq!("(12, 34)", aux(&mut "00001234"));
        assert!(timezone_colonless(&mut "12345").is_err());
    }

    #[test]
    fn test_timezone() {
        let make_timezone = |input: &mut &str| {
            timezone(input)
                .map_err(|e| eprintln!("TEST FAILED AT:\n{e}"))
                .map(|offset| format!("{offset}"))
                .expect("expect tests to succeed")
        };

        assert_eq!("+00:00", make_timezone(&mut "gmt"));
        assert_eq!("+01:00", make_timezone(&mut "a"));
        assert_eq!("+00:00", make_timezone(&mut "utc"));
        assert_eq!("-02:00", make_timezone(&mut "brst"));
        assert_eq!("-03:00", make_timezone(&mut "brt"));
    }

    #[test]
    fn test_timezone_num() {
        let make_timezone = |input: &mut &str| {
            timezone_num(input)
                .map_err(|e| eprintln!("TEST FAILED AT:\n{e}"))
                .map(|offset| format!("{offset}"))
                .expect("expect tests to succeed")
        };

        assert_eq!("+00:00", make_timezone(&mut "+00:00"));
        assert_eq!("+00:00", make_timezone(&mut "+0000"));
        assert_eq!("-00:00", make_timezone(&mut "-0000"));
    }
}
