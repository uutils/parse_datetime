// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use winnow::{ModalResult, Parser};

use super::time;

pub(crate) fn parse(input: &mut &str) -> ModalResult<time::Offset> {
    let _ = "tz=\"".parse_next(input)?;
    let tz = time::timezone(input);
    let _ = "\" ".parse_next(input)?;
    return tz
}
