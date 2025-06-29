use winnow::ModalResult;

use super::time;

pub(crate) fn parse(input: &mut &str) -> ModalResult<time::Offset> {
    time::timezone(input)
}
