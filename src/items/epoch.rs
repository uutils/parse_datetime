// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use winnow::{combinator::preceded, ModalResult, Parser};

use super::primitive::{dec_int, s};

pub fn parse(input: &mut &str) -> ModalResult<i32> {
    s(preceded("@", dec_int)).parse_next(input)
}
