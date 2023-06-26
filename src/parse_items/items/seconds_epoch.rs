// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use nom::combinator::map;
use nom::{
    bytes::complete::tag,
    character::complete,
    combinator::opt,
    sequence::{preceded, tuple},
    Parser,
};

use crate::parse_items::items::Item;
use crate::parse_items::nano_seconds::nano_seconds;
use crate::parse_items::singleton_list;
use crate::parse_items::PResult;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct SecondsEpoch {
    pub seconds: i64,
    pub nanoseconds: u32,
}

pub fn seconds_epoch(input: &str) -> PResult<Vec<Item>> {
    singleton_list(map(raw_seconds_epoch, Item::SecondsEpoch)).parse(input)
}

fn raw_seconds_epoch(input: &str) -> PResult<SecondsEpoch> {
    let (tail, (seconds, nanoseconds)) =
        preceded(tag("@"), tuple((complete::i64, opt(nano_seconds)))).parse(input)?;

    let nanoseconds = nanoseconds.unwrap_or(0);
    Ok((
        tail,
        SecondsEpoch {
            seconds,
            nanoseconds,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parse_items::tests::ptest;

    use super::*;

    macro_rules! epoch {
        ($name:ident : $input:literal => $seconds:literal:$nanoseconds:literal + $tail:literal) => {
            ptest! { $name : raw_seconds_epoch($input) => SecondsEpoch { seconds: $seconds, nanoseconds: $nanoseconds } , $tail }
        };
        ($name:ident : $input:literal => X) => {
            ptest! { $name : raw_seconds_epoch($input) => X }
        };
    }

    epoch! { positive                    : "@123abc"                    => 123:0          + "abc" }
    epoch! { negative                    : "@-9876abc"                  => -9876:0        + "abc" }
    epoch! { no_at                       : "-9876abc"                   => X }
    epoch! { short_fraction              : "@123.456"                   => 123:456000000  + "" }
    epoch! { almost_a_second             : "@0.999999999"               => 0:999999999    + "" }
    epoch! { silent_ignore_long_fraction : "@-123.98765432184723146abc" => -123:987654321 + "abc" }
}
