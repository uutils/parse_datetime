// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, tag};
use nom::character::complete::none_of;
use nom::combinator::{map, value};
use nom::sequence::delimited;
use nom::Parser;

use crate::parse_items::items::Item;
use crate::parse_items::singleton_list;
use crate::parse_items::PResult;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TzIdentifier(pub String);

pub fn tz_identifier(input: &str) -> PResult<Vec<Item>> {
    singleton_list(map(raw_tz_identifier, Item::TimeZoneRule)).parse(input)
}

fn raw_tz_identifier(input: &str) -> PResult<TzIdentifier> {
    let (tail, id) = delimited(
        tag("TZ=\""),
        escaped_transform(
            none_of("\\\""),
            '\\',
            alt((value("\\", tag("\\")), value("\"", tag("\"")))),
        ),
        tag("\""),
    )
    .parse(input)?;
    Ok((tail, TzIdentifier(id)))
}

#[cfg(test)]
mod tests {
    use nom::Parser;

    use super::*;

    #[test]
    fn amsterdam() {
        assert_eq!(
            raw_tz_identifier.parse(r#"TZ="Europe/Amsterdam" 14 november"#),
            Ok((" 14 november", TzIdentifier("Europe/Amsterdam".into())))
        );
    }

    #[test]
    fn new_york() {
        assert_eq!(
            raw_tz_identifier.parse(r#"TZ="Americas/New_York" nov14"#),
            Ok((" nov14", TzIdentifier("Americas/New_York".into())))
        );
    }

    #[test]
    fn escape() {
        assert_eq!(
            raw_tz_identifier.parse(r#"TZ="\"Escape\"\\this"nov14"#),
            Ok(("nov14", TzIdentifier(r#""Escape"\this"#.into())))
        );
    }
}
