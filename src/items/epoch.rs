use winnow::{combinator::preceded, ModalResult, Parser};

use super::primitive::{dec_int, s};

pub fn parse(input: &mut &str) -> ModalResult<i32> {
    s(preceded("@", dec_int)).parse_next(input)
}
