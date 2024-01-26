// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore chrono multispace0

//! From the GNU docs:
//!
//! > A date is a string, possibly empty, containing many items separated by
//! > whitespace. The whitespace may be omitted when no ambiguity arises. The
//! > empty string means the beginning of today (i.e., midnight). Order of the
//! > items is immaterial. A date string may contain many flavors of items:
//! >  - calendar date items
//! >  - time of day items
//! >  - time zone items
//! >  - combined date and time of day items
//! >  - day of the week items
//! >  - relative items
//! >  - pure numbers.
//!
//! We put all of those in separate modules:
//!  - [`date`]
//!  - [`time`]
//!  - [`time_zone`]
//!  - [`combined`]
//!  - [`weekday`]
//!  - [`relative`]
//!  - [`number]

use chrono::{NaiveDateTime, Weekday};
use winnow::{
    ascii::multispace0,
    combinator::{alt, preceded},
    error::ParserError,
    stream::{AsChar, Stream, StreamIsPartial},
    PResult, Parser,
};
mod date;
mod time;

pub enum Item {
    Date(date::Date),
    Time(time::Time),
    _TimeZone,
    Combined(NaiveDateTime),
    Weekday(Weekday),
    _Relative,
    _PureNumber,
}

/// Allow spaces after a parser
fn s<I, O, E>(p: impl Parser<I, O, E>) -> impl Parser<I, O, E>
where
    I: StreamIsPartial + Stream,
    <I as Stream>::Token: AsChar + Clone,
    E: ParserError<I>,
{
    preceded(multispace0, p)
}

pub fn parse(input: &mut &str) -> PResult<Item> {
    alt((date::parse.map(Item::Date), time::parse.map(Item::Time))).parse_next(input)
}

mod time_zone {}

mod combined {}

mod weekday {}

mod relative {}

mod number {}
