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

mod combined;
mod date;
mod ordinal;
mod relative;
mod time;
mod time_zone;
mod weekday;
mod number {}

use winnow::{
    ascii::{alpha1, dec_uint, multispace0},
    combinator::{alt, delimited, not, opt, peek, preceded, repeat, separated},
    error::ParserError,
    stream::AsChar,
    token::{none_of, take_while},
    PResult, Parser,
};

pub enum Item {
    DateTime(combined::DateTime),
    Date(date::Date),
    Time(time::Time),
    Weekday(weekday::Weekday),
    Relative(relative::Relative),
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
    separated(0.., multispace0, alt((comment, ignored_hyphen_or_plus))).parse_next(input)
}

/// A hyphen or plus is ignored when it is not followed by a digit
///
/// This includes being followed by a comment! Compare these inputs:
/// ```txt
/// - 12 weeks
/// - (comment) 12 weeks
/// ```
/// The last comment should be ignored.
///
/// The plus is undocumented, but it seems to be ignored.
fn ignored_hyphen_or_plus<'a, E>(input: &mut &'a str) -> PResult<(), E>
where
    E: ParserError<&'a str>,
{
    (
        alt(('-', '+')),
        multispace0,
        peek(not(take_while(1, AsChar::is_dec_digit))),
    )
        .void()
        .parse_next(input)
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
        combined::parse.map(Item::DateTime),
        date::parse.map(Item::Date),
        time::parse.map(Item::Time),
        relative::parse.map(Item::Relative),
        weekday::parse.map(Item::Weekday),
        // time_zone::parse.map(Item::TimeZone),
    ))
    .parse_next(input)
}
