// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Numbers without other symbols
//!
//! The GNU docs state:
//!
//! > If the decimal number is of the form yyyymmdd and no other calendar date
//! > item (see Calendar date items) appears before it in the date string, then
//! > yyyy is read as the year, mm as the month number and dd as the day of the
//! > month, for the specified calendar date.
//! >
//! > If the decimal number is of the form hhmm and no other time of day item
//! > appears before it in the date string, then hh is read as the hour of the
//! > day and mm as the minute of the hour, for the specified time of day. mm
//! > can also be omitted.

use winnow::{combinator::cond, PResult};

enum Number {
    Date,
    Time,
    Year,
}

pub fn parse(seen_date: bool, seen_time: bool, input: &mut &str) -> PResult<Number> {
    alt((
        cond(!seen_date, date_number),
        cond(!seen_time, time_number),
        cond(seen_date && seen_time, year_number),
    ))
    .parse_next(input)
}

fn date_number(input: &mut &str) -> PResult<Number> {
    todo!()
}

fn time_number(input: &mut &str) -> PResult<Number> {
    todo!()
}

fn year_number(input: &mut &str) -> PResult<Number> {
    todo!()
}
