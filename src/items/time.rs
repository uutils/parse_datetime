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
    ascii::digit1,
    combinator::{alt, eof, opt, peek, preceded, terminated},
    error::{ContextError, ErrMode, StrContext, StrContextValue},
    seq,
    stream::AsChar,
    token::take_while,
    ModalResult, Parser,
};

use crate::ParseDateTimeError;

use super::{
    primitive::{dec_uint, float, s},
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
    fn merge(self, offset: Offset) -> Option<Offset> {
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

        Some(Offset {
            negative,
            hours,
            minutes,
        })
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
        (hour24, timezone).map(|(hour, offset)| Time {
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
            offset: opt(timezone)
        }),
    ))
    .parse_next(input)
}

/// Parse a time ending with AM or PM
///
/// The hours are restricted to 12 or lower in this format
fn am_pm_time(input: &mut &str) -> ModalResult<Time> {
    let (h, m, s, meridiem) = seq!(
        hour12,
        opt(preceded(colon, minute)),
        opt(preceded(colon, second)),
        terminated(
            alt((
                s("am").value(Meridiem::Am),
                s("a.m.").value(Meridiem::Am),
                s("pm").value(Meridiem::Pm),
                s("p.m.").value(Meridiem::Pm)
            )),
            eof
        )
    )
    .parse_next(input)?;

    if h == 0 {
        let mut ctx_err = ContextError::new();
        ctx_err.push(StrContext::Expected(StrContextValue::Description(
            "hour must be greater than 0 when meridiem is specified",
        )));
        return Err(ErrMode::Cut(ctx_err));
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
    alt((timezone_num, timezone_name_offset)).parse_next(input)
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
                let mut ctx_err = ContextError::new();
                ctx_err.push(StrContext::Expected(StrContextValue::Description(
                    "timezone hour between 0 and 12",
                )));
                return Err(ErrMode::Cut(ctx_err));
            }

            if !(0..=60).contains(&minutes) {
                let mut ctx_err = ContextError::new();
                ctx_err.push(StrContext::Expected(StrContextValue::Description(
                    "timezone minute between 0 and 60",
                )));
                return Err(ErrMode::Cut(ctx_err));
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
        s(take_while(1..=2, AsChar::is_dec_digit)).try_map(|x: &str| {
            // parse will fail on empty input
            if x.is_empty() {
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
fn timezone_colonless(input: &mut &str) -> ModalResult<(u32, u32)> {
    if let Ok(x) = peek(s(digit1::<&str, ContextError>)).parse_next(input) {
        if x.len() > 4 {
            let mut ctx_err = ContextError::new();
            ctx_err.push(StrContext::Expected(StrContextValue::Description(
                "timezone offset cannot be more than 4 digits",
            )));
            return Err(ErrMode::Cut(ctx_err));
        }
    }

    // TODO: GNU date supports number strings with leading zeroes, e.g.,
    // `UTC+000001100` is valid.
    s(take_while(0..=4, AsChar::is_dec_digit))
        .verify_map(|x: &str| {
            Some(match x.len() {
                0 => (0, 0),
                1 | 2 => (x.parse().ok()?, 0),
                // The minutes are the last two characters here, for some reason.
                3 => (x[..1].parse().ok()?, x[1..].parse().ok()?),
                4 => (x[..2].parse().ok()?, x[2..].parse().ok()?),
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
            let newtz = tz
                .merge(other_tz)
                .ok_or(ErrMode::Cut(ContextError::new()))?;

            return Ok(newtz);
        };
    }

    Ok(tz)
}

/// Timezone list extracted from:
///   https://www.timeanddate.com/time/zones/
fn tzname_to_offset(input: &str) -> ModalResult<Offset> {
    let mut offset_str = match input {
        "z" => Ok("+0"),
        "yekt" => Ok("+5"),
        "yekst" => Ok("+6"),
        "yapt" => Ok("+10"),
        "yakt" => Ok("+9"),
        "yakst" => Ok("+10"),
        "y" => Ok("-12"),
        "x" => Ok("-11"),
        "wt" => Ok("+0"),
        "wst" => Ok("+13"),
        "wita" => Ok("+8"),
        "wit" => Ok("+9"),
        "wib" => Ok("+7"),
        "wgt" => Ok("-2"),
        "wgst" => Ok("-1"),
        "wft" => Ok("+12"),
        "wet" => Ok("+0"),
        "west" => Ok("+1"),
        "wat" => Ok("+1"),
        "wast" => Ok("+2"),
        "warst" => Ok("-3"),
        "wakt" => Ok("+12"),
        "w" => Ok("-10"),
        "vut" => Ok("+11"),
        "vost" => Ok("+6"),
        "vlat" => Ok("+10"),
        "vlast" => Ok("+11"),
        "vet" => Ok("-4"),
        "v" => Ok("-9"),
        "uzt" => Ok("+5"),
        "uyt" => Ok("-3"),
        "uyst" => Ok("-2"),
        "utc" => Ok("+0"),
        "ulat" => Ok("+8"),
        "ulast" => Ok("+9"),
        "u" => Ok("-8"),
        "tvt" => Ok("+12"),
        "trt" => Ok("+3"),
        "tot" => Ok("+13"),
        "tost" => Ok("+14"),
        "tmt" => Ok("+5"),
        "tlt" => Ok("+9"),
        "tkt" => Ok("+13"),
        "tjt" => Ok("+5"),
        "tft" => Ok("+5"),
        "taht" => Ok("-10"),
        "t" => Ok("-7"),
        "syot" => Ok("+3"),
        "sst" => Ok("-11"),
        "srt" => Ok("-3"),
        "sret" => Ok("+11"),
        "sgt" => Ok("+8"),
        "sct" => Ok("+4"),
        "sbt" => Ok("+11"),
        "sast" => Ok("+2"),
        "samt" => Ok("+4"),
        "sakt" => Ok("+11"),
        "s" => Ok("-6"),
        "rott" => Ok("-3"),
        "ret" => Ok("+4"),
        "r" => Ok("-5"),
        "qyzt" => Ok("+6"),
        "q" => Ok("-4"),
        "pyt" => Ok("-4"),
        "pyst" => Ok("-3"),
        "pwt" => Ok("+9"),
        "pt" => Ok("-7"),
        "pst" => Ok("-8"),
        "pont" => Ok("+11"),
        "pmst" => Ok("-3"),
        "pmdt" => Ok("-2"),
        "pkt" => Ok("+5"),
        "pht" => Ok("+8"),
        "phot" => Ok("+13"),
        "pgt" => Ok("+10"),
        "pett" => Ok("+12"),
        "petst" => Ok("+12"),
        "pet" => Ok("-5"),
        "pdt" => Ok("-7"),
        "p" => Ok("-3"),
        "orat" => Ok("+5"),
        "omst" => Ok("+6"),
        "omsst" => Ok("+7"),
        "o" => Ok("-2"),
        "nzst" => Ok("+12"),
        "nzdt" => Ok("+13"),
        "nut" => Ok("-11"),
        "nst" => Ok("-3:30"),
        "nrt" => Ok("+12"),
        "npt" => Ok("+5:45"),
        "novt" => Ok("+7"),
        "novst" => Ok("+7"),
        "nft" => Ok("+11"),
        "nfdt" => Ok("+12"),
        "ndt" => Ok("-2:30"),
        "nct" => Ok("+11"),
        "n" => Ok("-1"),
        "myt" => Ok("+8"),
        "mvt" => Ok("+5"),
        "mut" => Ok("+4"),
        "mt" => Ok("-6"),
        "mst" => Ok("-7"),
        "msk" => Ok("+3"),
        "msd" => Ok("+4"),
        "mmt" => Ok("+6:30"),
        "mht" => Ok("+12"),
        "mdt" => Ok("-6"),
        "mawt" => Ok("+5"),
        "mart" => Ok("-9:30"),
        "magt" => Ok("+11"),
        "magst" => Ok("+12"),
        "m" => Ok("+12"),
        "lint" => Ok("+14"),
        "lhst" => Ok("+10:30"),
        "lhdt" => Ok("+11"),
        "l" => Ok("+11"),
        "kuyt" => Ok("+4"),
        "kst" => Ok("+9"),
        "krat" => Ok("+7"),
        "krast" => Ok("+8"),
        "kost" => Ok("+11"),
        "kgt" => Ok("+6"),
        "k" => Ok("+10"),
        "jst" => Ok("+9"),
        "ist" => Ok("+5:30"),
        "irst" => Ok("+3:30"),
        "irkt" => Ok("+8"),
        "irkst" => Ok("+9"),
        "irdt" => Ok("+4:30"),
        "iot" => Ok("+6"),
        "idt" => Ok("+3"),
        "ict" => Ok("+7"),
        "i" => Ok("+9"),
        "hst" => Ok("-10"),
        "hovt" => Ok("+7"),
        "hovst" => Ok("+8"),
        "hkt" => Ok("+8"),
        "hdt" => Ok("-9"),
        "h" => Ok("+8"),
        "gyt" => Ok("-4"),
        "gst" => Ok("+4"),
        "gmt" => Ok("+0"),
        "gilt" => Ok("+12"),
        "gft" => Ok("-3"),
        "get" => Ok("+4"),
        "gamt" => Ok("-9"),
        "galt" => Ok("-6"),
        "g" => Ok("+7"),
        "fnt" => Ok("-2"),
        "fkt" => Ok("-4"),
        "fkst" => Ok("-3"),
        "fjt" => Ok("+12"),
        "fjst" => Ok("+13"),
        "fet" => Ok("+3"),
        "f" => Ok("+6"),
        "et" => Ok("-4"),
        "est" => Ok("-5"),
        "egt" => Ok("-1"),
        "egst" => Ok("+0"),
        "eet" => Ok("+2"),
        "eest" => Ok("+3"),
        "edt" => Ok("-4"),
        "ect" => Ok("-5"),
        "eat" => Ok("+3"),
        "east" => Ok("-6"),
        "easst" => Ok("-5"),
        "e" => Ok("+5"),
        "ddut" => Ok("+10"),
        "davt" => Ok("+7"),
        "d" => Ok("+4"),
        "chst" => Ok("+10"),
        "cxt" => Ok("+7"),
        "cvt" => Ok("-1"),
        "ct" => Ok("-5"),
        "cst" => Ok("-6"),
        "cot" => Ok("-5"),
        "clt" => Ok("-4"),
        "clst" => Ok("-3"),
        "ckt" => Ok("-10"),
        "cist" => Ok("-5"),
        "cidst" => Ok("-4"),
        "chut" => Ok("+10"),
        "chot" => Ok("+8"),
        "chost" => Ok("+9"),
        "chast" => Ok("+12:45"),
        "chadt" => Ok("+13:45"),
        "cet" => Ok("+1"),
        "cest" => Ok("+2"),
        "cdt" => Ok("-5"),
        "cct" => Ok("+6:30"),
        "cat" => Ok("+2"),
        "cast" => Ok("+8"),
        "c" => Ok("+3"),
        "btt" => Ok("+6"),
        "bst" => Ok("+6"),
        "brt" => Ok("-3"),
        "brst" => Ok("-2"),
        "bot" => Ok("-4"),
        "bnt" => Ok("+8"),
        "b" => Ok("+2"),
        "aoe" => Ok("-12"),
        "azt" => Ok("+4"),
        "azst" => Ok("+5"),
        "azot" => Ok("-1"),
        "azost" => Ok("+0"),
        "awst" => Ok("+8"),
        "awdt" => Ok("+9"),
        "at" => Ok("-4:00"),
        "ast" => Ok("-3"),
        "art" => Ok("-3"),
        "aqtt" => Ok("+5"),
        "anat" => Ok("+12"),
        "anast" => Ok("+12"),
        "amt" => Ok("-4"),
        "amst" => Ok("-3"),
        "almt" => Ok("+6"),
        "akst" => Ok("-9"),
        "akdt" => Ok("-8"),
        "aft" => Ok("+4:30"),
        "aet" => Ok("+11"),
        "aest" => Ok("+10"),
        "aedt" => Ok("+11"),
        "adt" => Ok("+4"),
        "acwst" => Ok("+8:45"),
        "act" => Ok("-5"),
        "acst" => Ok("+9:30"),
        "acdt" => Ok("+10:30"),
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

        assert_eq!("+00:00", make_timezone(&mut "+00:00"));
        assert_eq!("+00:00", make_timezone(&mut "+0000"));
        assert_eq!("-00:00", make_timezone(&mut "-0000"));
        assert_eq!("+00:00", make_timezone(&mut "gmt"));
        assert_eq!("+01:00", make_timezone(&mut "a"));
        assert_eq!("+00:00", make_timezone(&mut "utc"));
        assert_eq!("-02:00", make_timezone(&mut "brst"));
        assert_eq!("-03:00", make_timezone(&mut "brt"));
    }
}
