// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore multispace0

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

mod date;
mod time;
mod time_zone;
mod weekday;
mod combined {}
mod relative;
mod number {}

use winnow::{
    ascii::multispace0,
    combinator::{alt, delimited, preceded, repeat, separated},
    error::ParserError,
    token::none_of,
    PResult, Parser,
};

pub enum Item {
    Date(date::Date),
    Time(time::Time),
    Weekday(weekday::Weekday),
    Relative(()),
    TimeZone(()),
}

/// Allow spaces and comments before a parser
///
/// Every token parser should be wrapped in this to allow spaces and comments.
/// It is only preceding, because that allows us to check mandatory whitespace
/// after running the parser.
fn s<'a, O, E>(p: impl Parser<&'a str, O, E>) -> impl Parser<&'a str, O, E>
where
    E: ParserError<&'a str>,
{
    preceded(space, p)
}

/// Parse the space in-between tokens
///
/// You probably want to use the [`s`] combinator instead.
fn space<'a, E>(input: &mut &'a str) -> PResult<(), E>
where
    E: ParserError<&'a str>,
{
    separated(0.., multispace0, comment).parse_next(input)
}

/// Parse a comment
///
/// A comment is given between parentheses, which must be balanced. Any other
/// tokens can be within the comment.
fn comment<'a, E>(input: &mut &'a str) -> PResult<(), E>
where
    E: ParserError<&'a str>,
{
    delimited(
        '(',
        repeat(0.., alt((none_of(['(', ')']).void(), comment))),
        ')',
    )
    .parse_next(input)
}

/// Parse an item
pub fn parse(input: &mut &str) -> PResult<Item> {
    alt((
        date::parse.map(Item::Date),
        time::parse.map(Item::Time),
        relative::parse.map(Item::Relative),
        weekday::parse.map(Item::Weekday),
        time_zone::parse.map(Item::TimeZone),
    ))
    .parse_next(input)
}
