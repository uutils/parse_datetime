// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::{FixedOffset, NaiveDate, TimeZone};
use chrono_tz::Tz;
use winnow::{combinator::peek, error::ErrMode, token::take_while, ModalResult, Parser};

use super::{primitive::ctx_err, timezone};

pub(crate) fn parse(input: &mut &str) -> ModalResult<FixedOffset> {
    let _ = "tz=\"".parse_next(input)?;
    let mut tz_name = take_while(1.., |character| character != '\"').parse_next(input)?;
    let _ = "\" ".parse_next(input)?;

    // Try and use the built in timezone system first before trying to use the
    // chrono_tz library which can handle named timezones
    if let Ok(mut offset) = peek(timezone::parse).parse_next(&mut tz_name) {
        offset.negative = !offset.negative;
        offset
            .try_into()
            .map_err(|_| ErrMode::Cut(ctx_err("Invalid timezone")))
    } else {
        let conv: Tz = chrono_tz::Tz::from_str_insensitive(tz_name)
            .map_err(|_| ErrMode::Cut(ctx_err("Invalid timezone")))?;
        Ok(chrono::Offset::fix(
            &conv.offset_from_utc_date(&NaiveDate::default()),
        ))
    }
}
