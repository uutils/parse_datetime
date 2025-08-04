// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore shhmm colonless

//! Parse a time item (without a date)
//!
//! The GNU docs state:
//!
//! > More generally, the time of day may be given as ‘hour:minute:second’,
//! > where hour is a number between 0 and 23, minute is a number between 0 and
//! > 59, and second is a number between 0 and 59 possibly followed by ‘.’ or
//! > ‘,’ and a fraction containing one or more digits. Alternatively,
//! > ‘:second’ can be omitted, in which case it is taken to be zero. On the
//! > rare hosts that support leap seconds, second may be 60.
//! >
//! > If the time is followed by ‘am’ or ‘pm’ (or ‘a.m.’ or ‘p.m.’), hour is
//! > restricted to run from 1 to 12, and ‘:minute’ may be omitted (taken to be
//! > zero). ‘am’ indicates the first half of the day, ‘pm’ indicates the
//! > second half of the day. In this notation, 12 is the predecessor of 1:
//! > midnight is ‘12am’ while noon is ‘12pm’. (This is the zero-oriented
//! > interpretation of ‘12am’ and ‘12pm’, as opposed to the old tradition
//! > derived from Latin which uses ‘12m’ for noon and ‘12pm’ for midnight.)
//! >
//! > The time may alternatively be followed by a time zone correction,
//! > expressed as ‘shhmm’, where s is ‘+’ or ‘-’, hh is a number of zone hours
//! > and mm is a number of zone minutes. The zone minutes term, mm, may be
//! > omitted, in which case the one- or two-digit correction is interpreted as
//! > a number of hours. You can also separate hh from mm with a colon. When a
//! > time zone correction is given this way, it forces interpretation of the
//! > time relative to Coordinated Universal Time (UTC), overriding any
//! > previous specification for the time zone or the local time zone. For
//! > example, ‘+0530’ and ‘+05:30’ both stand for the time zone 5.5 hours
//! > ahead of UTC (e.g., India). This is the best way to specify a time zone
//! > correction by fractional parts of an hour. The maximum zone correction is
//! > 24 hours.
//! >
//! > Either ‘am’/‘pm’ or a time zone correction may be specified, but not both.

use std::fmt::Display;

use chrono::FixedOffset;
use winnow::{
    combinator::{alt, opt, peek, preceded},
    error::{ContextError, ErrMode},
    seq,
    stream::AsChar,
    token::take_while,
    ModalResult, Parser,
};

use crate::ParseDateTimeError;

use super::{
    primitive::{ctx_err, dec_uint, float, s},
    relative,
};

#[derive(PartialEq, Debug, Clone, Default)]
pub struct Offset {
    pub(crate) negative: bool,
    pub(crate) hours: u32,
    pub(crate) minutes: u32,
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
    pub second: f64,
    pub offset: Option<Offset>,
}

impl Offset {
    fn merge(self, offset: Offset) -> Offset {
        fn combine(a: u32, neg_a: bool, b: u32, neg_b: bool) -> (u32, bool) {
            if neg_a == neg_b {
                (a + b, neg_a)
            } else if a > b {
                (a - b, neg_a)
            } else {
                (b - a, neg_b)
            }
        }
        let (hours_minutes, negative) = combine(
            self.hours * 60 + self.minutes,
            self.negative,
            offset.hours * 60 + offset.minutes,
            offset.negative,
        );
        let hours = hours_minutes / 60;
        let minutes = hours_minutes % 60;

        Offset {
            negative,
            hours,
            minutes,
        }
    }
}

impl TryFrom<Offset> for chrono::FixedOffset {
    type Error = ParseDateTimeError;

    fn try_from(
        Offset {
            negative,
            hours,
            minutes,
        }: Offset,
    ) -> Result<Self, Self::Error> {
        let secs = hours * 3600 + minutes * 60;

        let offset = if negative {
            FixedOffset::west_opt(
                secs.try_into()
                    .map_err(|_| ParseDateTimeError::InvalidInput)?,
            )
            .ok_or(ParseDateTimeError::InvalidInput)?
        } else {
            FixedOffset::east_opt(
                secs.try_into()
                    .map_err(|_| ParseDateTimeError::InvalidInput)?,
            )
            .ok_or(ParseDateTimeError::InvalidInput)?
        };

        Ok(offset)
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

#[derive(Clone)]
enum Meridiem {
    Am,
    Pm,
}

pub fn parse(input: &mut &str) -> ModalResult<Time> {
    alt((am_pm_time, iso)).parse_next(input)
}

/// Parse an ISO 8601 time string
///
/// Also used by the [`combined`](super::combined) module
pub fn iso(input: &mut &str) -> ModalResult<Time> {
    alt((
        (hour24, timezone_num).map(|(hour, offset)| Time {
            hour,
            minute: 0,
            second: 0.0,
            offset: Some(offset),
        }),
        seq!(Time {
            hour: hour24,
            _: colon,
            minute: minute,
            second: opt(preceded(colon, second)).map(|s| s.unwrap_or(0.0)),
            offset: opt(timezone_num)
        }),
    ))
    .parse_next(input)
}

/// Parse a time ending with AM or PM
///
/// The hours are restricted to 12 or lower in this format
fn am_pm_time(input: &mut &str) -> ModalResult<Time> {
    let (h, m, s, meridiem) = (
        hour12,
        opt(preceded(colon, minute)),
        opt(preceded(colon, second)),
        alt((
            s("am").value(Meridiem::Am),
            s("a.m.").value(Meridiem::Am),
            s("pm").value(Meridiem::Pm),
            s("p.m.").value(Meridiem::Pm),
        )),
    )
        .parse_next(input)?;

    if h == 0 {
        return Err(ErrMode::Cut(ctx_err(
            "hour must be greater than 0 when meridiem is specified",
        )));
    }

    let mut h = h % 12;
    if let Meridiem::Pm = meridiem {
        h += 12;
    }
    Ok(Time {
        hour: h,
        minute: m.unwrap_or(0),
        second: s.unwrap_or(0.0),
        offset: None,
    })
}

/// Parse a colon preceded by whitespace
fn colon(input: &mut &str) -> ModalResult<()> {
    s(':').void().parse_next(input)
}

/// Parse a number of hours in `0..24`(preceded by whitespace)
fn hour24(input: &mut &str) -> ModalResult<u32> {
    s(dec_uint).verify(|x| *x < 24).parse_next(input)
}

/// Parse a number of hours in `0..=12` (preceded by whitespace)
fn hour12(input: &mut &str) -> ModalResult<u32> {
    s(dec_uint).verify(|x| *x <= 12).parse_next(input)
}

/// Parse a number of minutes (preceded by whitespace)
fn minute(input: &mut &str) -> ModalResult<u32> {
    s(dec_uint).verify(|x| *x < 60).parse_next(input)
}

/// Parse a number of seconds (preceded by whitespace)
fn second(input: &mut &str) -> ModalResult<f64> {
    s(float)
        .verify(|x| *x < 60.0)
        .map(|x| {
            // Truncates the fractional part of seconds to 9 digits.
            let factor = 10f64.powi(9);
            (x * factor).trunc() / factor
        })
        .parse_next(input)
}

pub(crate) fn timezone(input: &mut &str) -> ModalResult<Offset> {
    timezone_name_offset.parse_next(input)
}

/// Parse a timezone starting with `+` or `-`
fn timezone_num(input: &mut &str) -> ModalResult<Offset> {
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
fn timezone_colon(input: &mut &str) -> ModalResult<(u32, u32)> {
    seq!(
        s(take_while(1..=2, AsChar::is_dec_digit)).try_map(|x: &str| x.parse()),
        _: colon,
        s(take_while(1..=2, AsChar::is_dec_digit)).try_map(|x: &str| x.parse()),
    )
    .parse_next(input)
}

/// Parse a timezone offset without colon
fn timezone_colonless(input: &mut &str) -> ModalResult<(u32, u32)> {
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
    let tz = tzname_to_offset(nextword)?;

    // Strings like "UTC +8 years" are ambiguous, they can either be parsed as
    // "UTC+8" and "years", or "UTC" and "+8 years". GNU date parses them the
    // second way, so we do the same here.
    //
    // Only process if the input cannot be parsed as a relative time.
    if peek(relative::parse).parse_next(input).is_err() {
        if let Ok(other_tz) = timezone_num.parse_next(input) {
            let newtz = tz.merge(other_tz);

            return Ok(newtz);
        };
    }

    Ok(tz)
}

/// Named timezone list.
///
/// The full list of timezones can be extracted from
/// https://www.timeanddate.com/time/zones/. GNU date only supports a subset of
/// these. We support the same subset as GNU date.
///
/// From the GNU date manual:
///
/// > Time zone items other than ‘UTC’ and ‘Z’ are obsolescent and are not
/// > recommended, because they are ambiguous; for example, ‘EST’ has a
/// > different meaning in Australia than in the United States, and ‘A’ has
/// > different meaning as a military time zone than as an obsolete RFC 822
/// > time zone.  Instead, it's better to use unambiguous numeric time zone
/// > corrections like ‘-0500’.
fn tzname_to_offset(input: &str) -> ModalResult<Offset> {
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
    use super::{Offset, Time};
    use crate::items::time::parse;

    #[test]
    fn simple() {
        let reference = Time {
            hour: 20,
            minute: 2,
            second: 0.0,
            offset: None,
        };

        for mut s in [
            "20:02:00.000000",
            "20:02:00",
            "20:02+:00",
            "20:02-:00",
            "20----:02--(these hyphens are ignored)--:00",
            "20++++:02++(these plusses are ignored)++:00",
            "20: (A comment!)   02 (Another comment!)  :00",
            "20:02  (A nested (comment!))  :00",
            "20:02  (So (many (nested) comments!!!!))  :00",
            "20   :    02  :   00.000000",
            "20:02",
            "20  :   02",
            "8:02pm",
            "8:   02     pm",
            "8:02p.m.",
            "8:   02     p.m.",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn invalid() {
        assert!(parse(&mut "00:00am").is_err());
        assert!(parse(&mut "00:00:00am").is_err());
    }

    #[test]
    fn hours_only() {
        let reference = Time {
            hour: 11,
            minute: 0,
            second: 0.0,
            offset: None,
        };

        for mut s in [
            "11am",
            "11 am",
            "11 - am",
            "11 + am",
            "11 a.m.",
            "11   :  00",
            "11:00:00",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn noon() {
        let reference = Time {
            hour: 12,
            minute: 0,
            second: 0.0,
            offset: None,
        };

        for mut s in [
            "12:00",
            "12pm",
            "12 pm",
            "12 (A comment!) pm",
            "12 pm",
            "12 p.m.",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn midnight() {
        let reference = Time {
            hour: 0,
            minute: 0,
            second: 0.0,
            offset: None,
        };

        for mut s in ["00:00", "12am"] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn offset_hours() {
        let reference = Time {
            hour: 1,
            minute: 23,
            second: 0.0,
            offset: Some(Offset {
                negative: false,
                hours: 5,
                minutes: 0,
            }),
        };

        for mut s in [
            "1:23+5",
            "1:23 + 5",
            "1:23+05",
            "1:23 + 5 : 00",
            "1:23+05:00",
            "1:23+05:0",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn offset_hours_and_minutes() {
        let reference = Time {
            hour: 3,
            minute: 45,
            second: 0.0,
            offset: Some(Offset {
                negative: false,
                hours: 5,
                minutes: 35,
            }),
        };

        for mut s in [
            "3:45+535",
            "3:45-+535",
            "03:45+535",
            "3   :  45  +  535",
            "3:45+0535",
            "3:45+5:35",
            "3:45+05:35",
            "3:45  + 05 : 35",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn offset_minutes() {
        let reference = Time {
            hour: 3,
            minute: 45,
            second: 0.0,
            offset: Some(Offset {
                negative: false,
                hours: 0,
                minutes: 35,
            }),
        };

        for mut s in [
            "3:45+035",
            "03:45+035",
            "3   :  45  +  035",
            "3:45+0035",
            "3:45+0:35",
            "3:45+00:35",
            "3:45  + 00 : 35",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn offset_negative() {
        let reference = Time {
            hour: 3,
            minute: 45,
            second: 0.0,
            offset: Some(Offset {
                negative: true,
                hours: 5,
                minutes: 35,
            }),
        };

        for mut s in [
            "3:45-535",
            "03:45-535",
            "3   :  45  -  535",
            "3:45-0535",
            "3:45-5:35",
            "3:45-05:35",
            "3:45  - 05 : 35",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn test_timezone_colonless() {
        use super::timezone_colonless;

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
        use super::timezone;
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
        use super::timezone_num;
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
