// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore Caseless shhmm colonless

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

use winnow::{
    ascii::{dec_uint, float, Caseless},
    combinator::{alt, opt, preceded},
    seq,
    stream::AsChar,
    token::take_while,
    PResult, Parser,
};

use super::s;

#[derive(PartialEq, Debug)]
pub struct Time {
    hour: i32,
    minute: i32,
    second: f64,
    offset: Option<Offset>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Offset {
    negative: bool,
    hours: u32,
    minutes: u32,
}

#[derive(Clone)]
enum Suffix {
    Am,
    Pm,
    TimeZone(Offset),
}

pub fn parse(input: &mut &str) -> PResult<Time> {
    // Hour followed by suffix is a special case where minutes are optional
    alt((
        (hour, suffix).map(|(h, s)| (h, 0, 0.0, Some(s))),
        seq!(
            hour,
            _: colon,
            minute,
            opt(preceded(colon, second)).map(|s| s.unwrap_or(0.0)),
            opt(suffix)
        ),
    ))
    .verify_map(|(mut hour, minute, second, suffix)| {
        let mut offset = None;
        match suffix {
            None => {}
            // 12am should be noon
            Some(Suffix::Am) => {
                // With an AM/PM suffix
                if hour > 12 {
                    return None;
                }
                if hour == 12 {
                    hour = 0;
                };
            }
            // 12pm should be noon
            Some(Suffix::Pm) => {
                if hour > 12 {
                    return None;
                } else if hour == 12 {
                    hour = 12;
                } else {
                    hour += 12;
                }
            }
            Some(Suffix::TimeZone(tz)) => offset = Some(tz),
        };
        Some(Time {
            hour: hour as i32,
            minute: minute as i32,
            second,
            offset,
        })
    })
    .parse_next(input)
}

/// Parse a colon preceded by whitespace
fn colon(input: &mut &str) -> PResult<()> {
    s(':').void().parse_next(input)
}

/// Parse a number of hours (preceded by whitespace)
fn hour(input: &mut &str) -> PResult<u32> {
    s(dec_uint).verify(|x| *x < 24).parse_next(input)
}

/// Parse a number of minutes (preceded by whitespace)
fn minute(input: &mut &str) -> PResult<u32> {
    s(dec_uint).verify(|x| *x < 60).parse_next(input)
}

/// Parse a number of seconds (preceded by whitespace)
fn second(input: &mut &str) -> PResult<f64> {
    s(float).verify(|x| *x < 60.0).parse_next(input)
}

/// Parse a suffix of am, pm or an offset (preceded by whitespace)
fn suffix(input: &mut &str) -> PResult<Suffix> {
    alt((
        s(Caseless("am")).value(Suffix::Am),
        s(Caseless("a.m.")).value(Suffix::Am),
        s(Caseless("pm")).value(Suffix::Pm),
        s(Caseless("p.m.")).value(Suffix::Pm),
        timezone.map(Suffix::TimeZone),
    ))
    .parse_next(input)
}

/// Parse a timezone starting with `+` or `-`
fn timezone(input: &mut &str) -> PResult<Offset> {
    seq!(plus_or_minus, alt((timezone_colon, timezone_colonless)))
        .map(|(negative, (hours, minutes))| Offset {
            negative,
            hours,
            minutes,
        })
        .parse_next(input)
}

/// Parse a timezone offset with a colon separating hours and minutes
fn timezone_colon(input: &mut &str) -> PResult<(u32, u32)> {
    seq!(
        // There's an edge case here: GNU allows the hours to be omitted
        s(take_while(0..=2, AsChar::is_dec_digit)).try_map(|x: &str| {
            // parse will fail on empty input
            if x == "" {
                Ok(0)
            } else {
                x.parse()
            }
        }),
        _: colon,
        s(take_while(1..=2, AsChar::is_dec_digit)).try_map(|x: &str| x.parse()),
    )
    .parse_next(input)
}

/// Parse a timezone offset without colon
fn timezone_colonless(input: &mut &str) -> PResult<(u32, u32)> {
    s(take_while(0..=4, AsChar::is_dec_digit))
        .verify_map(|x: &str| {
            Some(match x.len() {
                0 => (0, 0),
                1 | 2 => (x.parse().ok()?, 0),
                // The minutes are the last two characters here, for some reason.
                3 => (x[..1].parse().ok()?, x[1..].parse().ok()?),
                4 => (x[..2].parse().ok()?, x[2..].parse().ok()?),
                _ => unreachable!("We only take up to 4 characters"),
            })
        })
        .parse_next(input)
}

/// Parse the plus or minus character and return whether it was negative
fn plus_or_minus(input: &mut &str) -> PResult<bool> {
    s(alt(("+".value(false), "-".value(true)))).parse_next(input)
}

#[cfg(test)]
mod test {
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
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }
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
            "11 a.m.",
            "11   :  00",
            "11:00:00",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
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
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
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
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
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
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
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
            "03:45+535",
            "3   :  45  +  535",
            "3:45+0535",
            "3:45+5:35",
            "3:45+05:35",
            "3:45  + 05 : 35",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
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
            "3:45+:35",
            "3:45  + 00 : 35",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
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
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }
    }
}
