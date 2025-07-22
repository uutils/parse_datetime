// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse a date item (without time component)
//!
//! The GNU docs say:
//!
//! > A calendar date item specifies a day of the year. It is specified
//! > differently, depending on whether the month is specified numerically
//! > or literally.
//! >
//! > ...
//! >
//! > For numeric months, the ISO 8601 format ‘year-month-day’ is allowed,
//! > where year is any positive number, month is a number between 01 and
//! > 12, and day is a number between 01 and 31. A leading zero must be
//! > present if a number is less than ten. If year is 68 or smaller, then
//! > 2000 is added to it; otherwise, if year is less than 100, then 1900
//! > is added to it. The construct ‘month/day/year’, popular in the United
//! > States, is accepted. Also ‘month/day’, omitting the year.
//! >
//! > Literal months may be spelled out in full: ‘January’, ‘February’,
//! > ‘March’, ‘April’, ‘May’, ‘June’, ‘July’, ‘August’, ‘September’,
//! > ‘October’, ‘November’ or ‘December’. Literal months may be
//! > abbreviated to their first three letters, possibly followed by an
//! > abbreviating dot. It is also permitted to write ‘Sept’ instead of
//! > ‘September’.

use winnow::{
    ascii::alpha1,
    combinator::{alt, opt, preceded, trace},
    error::ErrMode,
    seq,
    stream::AsChar,
    token::take_while,
    ModalResult, Parser,
};

use super::primitive::{ctx_err, dec_uint, s};
use crate::ParseDateTimeError;

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct Date {
    pub day: u32,
    pub month: u32,
    pub year: Option<u32>,
}

impl TryFrom<(&str, u32, u32)> for Date {
    type Error = &'static str;

    /// Create a `Date` from a tuple of `(year, month, day)`.
    ///
    /// Note: The `year` is represented as a `&str` to handle a specific GNU
    /// compatibility quirk. According to the GNU documentation: "if the year is
    /// 68 or smaller, then 2000 is added to it; otherwise, if year is less than
    /// 100, then 1900 is added to it." This adjustment only applies to
    /// two-digit year strings. For example, `"00"` is interpreted as `2000`,
    /// whereas `"0"`, `"000"`, or `"0000"` are interpreted as `0`.
    fn try_from(value: (&str, u32, u32)) -> Result<Self, Self::Error> {
        let (year_str, month, day) = value;

        let mut year = year_str
            .parse::<u32>()
            .map_err(|_| "year must be a valid number")?;

        // If year is 68 or smaller, then 2000 is added to it; otherwise, if year
        // is less than 100, then 1900 is added to it.
        //
        // GNU quirk: this only applies to two-digit years. For example,
        // "98-01-01" will be parsed as "1998-01-01", while "098-01-01" will be
        // parsed as "0098-01-01".
        if year_str.len() == 2 {
            if year <= 68 {
                year += 2000
            } else {
                year += 1900
            }
        }

        // 2147485547 is the maximum value accepted by GNU, but chrono only
        // behaves like GNU for years in the range: [0, 9999], so we keep in the
        // range [0, 9999].
        //
        // See discussion in https://github.com/uutils/parse_datetime/issues/160.
        if year > 9999 {
            return Err("year must be no greater than 9999");
        }

        if !(1..=12).contains(&month) {
            return Err("month must be between 1 and 12");
        }

        let is_leap_year = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);

        if !(1..=31).contains(&day)
            || (month == 2 && day > (if is_leap_year { 29 } else { 28 }))
            || ((month == 4 || month == 6 || month == 9 || month == 11) && day > 30)
        {
            return Err("day is not valid for the given month");
        }

        Ok(Date {
            day,
            month,
            year: Some(year),
        })
    }
}

impl TryFrom<(u32, u32)> for Date {
    type Error = &'static str;

    /// Create a `Date` from a tuple of `(month, day)`.
    fn try_from((month, day): (u32, u32)) -> Result<Self, Self::Error> {
        if !(1..=12).contains(&month) {
            return Err("month must be between 1 and 12");
        }

        if !(1..=31).contains(&day)
            || (month == 2 && day > 29)
            || ((month == 4 || month == 6 || month == 9 || month == 11) && day > 30)
        {
            return Err("day is not valid for the given month");
        }

        Ok(Date {
            day,
            month,
            year: None,
        })
    }
}

pub fn parse(input: &mut &str) -> ModalResult<Date> {
    alt((iso1, iso2, us, literal1, literal2)).parse_next(input)
}

/// Parse `[year]-[month]-[day]`
///
/// This is also used by [`combined`](super::combined).
pub fn iso1(input: &mut &str) -> ModalResult<Date> {
    let (year, _, month, _, day) = (
        // `year` must be a `&str`, see comment in `TryFrom` impl for `Date`.
        s(take_while(1.., AsChar::is_dec_digit)),
        s('-'),
        s(dec_uint),
        s('-'),
        s(dec_uint),
    )
        .parse_next(input)?;

    (year, month, day)
        .try_into()
        .map_err(|e| ErrMode::Cut(ctx_err(e)))
}

/// Parse `[year][month][day]`
///
/// This is also used by [`combined`](super::combined).
pub fn iso2(input: &mut &str) -> ModalResult<Date> {
    let date_str = take_while(5.., AsChar::is_dec_digit).parse_next(input)?;
    let len = date_str.len();

    // `year` must be a `&str`, see comment in `TryFrom` impl for `Date`.
    let year = &date_str[..len - 4];

    let month = date_str[len - 4..len - 2]
        .parse::<u32>()
        .map_err(|_| ErrMode::Cut(ctx_err("month must be a valid number")))?;

    let day = date_str[len - 2..]
        .parse::<u32>()
        .map_err(|_| ErrMode::Cut(ctx_err("day must be a valid number")))?;

    (year, month, day)
        .try_into()
        .map_err(|e| ErrMode::Cut(ctx_err(e)))
}

/// Parse `[year]/[month]/[day]` or `[month]/[day]/[year]` or `[month]/[day]`.
fn us(input: &mut &str) -> ModalResult<Date> {
    let (s1, _, n, s2) = (
        s(take_while(1.., AsChar::is_dec_digit)),
        s('/'),
        s(dec_uint),
        opt(preceded(s('/'), s(take_while(1.., AsChar::is_dec_digit)))),
    )
        .parse_next(input)?;

    match s2 {
        Some(s2) if s1.len() >= 4 => {
            // [year]/[month]/[day]
            //
            // GNU quirk: interpret as [year]/[month]/[day] if the first part is at
            // least 4 characters long.
            let day = s2
                .parse::<u32>()
                .map_err(|_| ErrMode::Cut(ctx_err("day must be a valid number")))?;
            (s1, n, day)
                .try_into()
                .map_err(|e| ErrMode::Cut(ctx_err(e)))
        }
        Some(s2) => {
            // [month]/[day]/[year]
            let month = s1
                .parse::<u32>()
                .map_err(|_| ErrMode::Cut(ctx_err("month must be a valid number")))?;
            (s2, month, n)
                .try_into()
                .map_err(|e| ErrMode::Cut(ctx_err(e)))
        }
        None => {
            // [month]/[day]
            let month = s1
                .parse::<u32>()
                .map_err(|_| ErrMode::Cut(ctx_err("month must be a valid number")))?;
            (month, n).try_into().map_err(|e| ErrMode::Cut(ctx_err(e)))
        }
    }
}

/// Parse `14 November 2022`, `14 Nov 2022`, "14nov2022", "14-nov-2022", "14-nov2022", "14nov-2022"
fn literal1(input: &mut &str) -> ModalResult<Date> {
    seq!(Date {
        day: day,
        _: opt(s('-')),
        month: literal_month,
        year: opt(preceded(opt(s('-')), year)),
    })
    .parse_next(input)
}

/// Parse `November 14, 2022` and `Nov 14, 2022`
fn literal2(input: &mut &str) -> ModalResult<Date> {
    seq!(Date {
        month: literal_month,
        day: day,
        // FIXME: GNU requires _some_ space between the day and the year,
        // probably to distinguish with floats.
        year: opt(preceded(s(","), year)),
    })
    .parse_next(input)
}

pub fn year(input: &mut &str) -> ModalResult<u32> {
    // 2147485547 is the maximum value accepted
    // by GNU, but chrono only behaves like GNU
    // for years in the range: [0, 9999], so we
    // keep in the range [0, 9999]
    trace(
        "year",
        s(
            take_while(1..=4, AsChar::is_dec_digit).map(|number_str: &str| {
                let year = number_str.parse::<u32>().unwrap();
                if number_str.len() == 2 {
                    if year <= 68 {
                        year + 2000
                    } else {
                        year + 1900
                    }
                } else {
                    year
                }
            }),
        ),
    )
    .parse_next(input)
}

fn day(input: &mut &str) -> ModalResult<u32> {
    s(dec_uint)
        .try_map(|x| {
            (1..=31)
                .contains(&x)
                .then_some(x)
                .ok_or(ParseDateTimeError::InvalidInput)
        })
        .parse_next(input)
}

/// Parse the name of a month (case-insensitive)
fn literal_month(input: &mut &str) -> ModalResult<u32> {
    s(alpha1)
        .verify_map(|s: &str| {
            Some(match s {
                "january" | "jan" => 1,
                "february" | "feb" => 2,
                "march" | "mar" => 3,
                "april" | "apr" => 4,
                "may" => 5,
                "june" | "jun" => 6,
                "july" | "jul" => 7,
                "august" | "aug" => 8,
                "september" | "sep" | "sept" => 9,
                "october" | "oct" => 10,
                "november" | "nov" => 11,
                "december" | "dec" => 12,
                _ => return None,
            })
        })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::{parse, Date};

    // Test cases from the GNU docs:
    //
    // ```
    // 2022-11-14     # ISO 8601.
    // 22-11-14       # Assume 19xx for 69 through 99,
    //                # 20xx for 00 through 68 (not recommended).
    // 11/14/2022     # Common U.S. writing.
    // 14 November 2022
    // 14 Nov 2022    # Three-letter abbreviations always allowed.
    // November 14, 2022
    // 14-nov-2022
    // 14nov2022
    // ```

    #[test]
    fn iso1() {
        let reference = Date {
            year: Some(1),
            month: 2,
            day: 3,
        };

        for mut s in ["1-2-3", "1 - 2 - 3", "1-02-03", "1-002-003", "001-02-03"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        // GNU quirk: when year string is 2 characters long and year is 68 or
        // smaller, 2000 is added to it.
        let reference = Date {
            year: Some(2001),
            month: 2,
            day: 3,
        };

        for mut s in ["01-2-3", "01-02-03"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        // GNU quirk: when year string is 2 characters long and year is less
        // than 100, 1900 is added to it.
        let reference = Date {
            year: Some(1970),
            month: 2,
            day: 3,
        };

        for mut s in ["70-2-3", "70-02-03"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        for mut s in ["01-00-01", "01-13-01", "01-01-32", "01-02-29", "01-04-31"] {
            let old_s = s.to_owned();
            assert!(parse(&mut s).is_err(), "Format string: {old_s}");
        }
    }

    #[test]
    fn iso2() {
        let reference = Date {
            year: Some(1),
            month: 2,
            day: 3,
        };

        for mut s in ["10203", "0010203", "00010203", "000010203"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        // GNU quirk: when year string is 2 characters long and year is 68 or
        // smaller, 2000 is added to it.
        let reference = Date {
            year: Some(2001),
            month: 2,
            day: 3,
        };

        let mut s = "010203";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");

        // GNU quirk: when year string is 2 characters long and year is less
        // than 100, 1900 is added to it.
        let reference = Date {
            year: Some(1970),
            month: 2,
            day: 3,
        };

        let mut s = "700203";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");

        for mut s in ["010001", "011301", "010132", "010229", "010431"] {
            let old_s = s.to_owned();
            assert!(parse(&mut s).is_err(), "Format string: {old_s}");
        }
    }

    #[test]
    fn us() {
        let reference = Date {
            year: Some(1),
            month: 2,
            day: 3,
        };

        for mut s in ["2/3/1", "2 / 3 / 1", "02/03/ 001", "0001/2/3"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        let reference = Date {
            year: None,
            month: 2,
            day: 3,
        };

        for mut s in ["2/3", "2 / 3"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        // GNU quirk: when year string is 2 characters long and year is 68 or
        // smaller, 2000 is added to it.
        let reference = Date {
            year: Some(2001),
            month: 2,
            day: 3,
        };

        let mut s = "2/3/01";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");

        // GNU quirk: when year string is 2 characters long and year is less
        // than 100, 1900 is added to it.
        let reference = Date {
            year: Some(1970),
            month: 2,
            day: 3,
        };

        let mut s = "2/3/70";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");

        for mut s in ["00/01/01", "13/01/01", "01/32/01", "02/30/01", "04/31/01"] {
            let old_s = s.to_owned();
            assert!(parse(&mut s).is_err(), "Format string: {old_s}");
        }
    }

    #[test]
    fn with_year() {
        let reference = Date {
            year: Some(2022),
            month: 11,
            day: 14,
        };

        for mut s in [
            "2022-11-14",
            "2022    -  11  -   14",
            "22-11-14",
            "2022---11----14",
            "22(comment 1)-11(comment 2)-14",
            "11/14/2022",
            "11--/14--/2022",
            "11(comment 1)/(comment 2)14(comment 3)/(comment 4)2022",
            "11   /  14   /      2022",
            "11/14/22",
            "14 november 2022",
            "14 nov 2022",
            "november 14, 2022",
            "november 14     ,     2022",
            "nov 14, 2022",
            "14-nov-2022",
            "14nov2022",
            "14nov      2022",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }
    }

    #[test]
    fn no_year() {
        let reference = Date {
            year: None,
            month: 11,
            day: 14,
        };
        for mut s in [
            "11/14",
            "14 november",
            "14 nov",
            "14(comment!)nov",
            "november 14",
            "november(comment!)14",
            "nov 14",
            "14-nov",
            "14nov",
            "14(comment????)nov",
        ] {
            assert_eq!(parse(&mut s).unwrap(), reference);
        }
    }

    #[test]
    fn test_year() {
        use super::year;

        // the minimun input length is 2
        // assert!(year(&mut "0").is_err());
        // -> GNU accepts year 0
        // test $(date -d '1-1-1' '+%Y') -eq '0001'

        // test $(date -d '68-1-1' '+%Y') -eq '2068'
        // 2-characters are converted to 19XX/20XX
        assert_eq!(year(&mut "10").unwrap(), 2010u32);
        assert_eq!(year(&mut "68").unwrap(), 2068u32);
        assert_eq!(year(&mut "69").unwrap(), 1969u32);
        assert_eq!(year(&mut "99").unwrap(), 1999u32);
        // 3,4-characters are converted verbatim
        assert_eq!(year(&mut "468").unwrap(), 468u32);
        assert_eq!(year(&mut "469").unwrap(), 469u32);
        assert_eq!(year(&mut "1568").unwrap(), 1568u32);
        assert_eq!(year(&mut "1569").unwrap(), 1569u32);
        // consumes at most 4 characters from the input
        //assert_eq!(year(&mut "1234567").unwrap(), 1234u32);
    }
}
