use nom::error::Error;
use nom::{IResult, Parser};

pub mod items;
pub(self) mod fixed_number;
pub(self) mod nano_seconds;

type PError<'i> = Error<&'i str>;
type PResult<'i, O> = IResult<&'i str, O, PError<'i>>;

fn singleton_list<'i, O>(mut inner: impl Parser<&'i str, O, PError<'i>>) -> impl Parser<&'i str, Vec<O>, PError<'i>> {
    move |input: &'i str| {
        let (tail, result) = inner.parse(input)?;
        Ok((tail, vec![result]))
    }
}

#[cfg(test)]
mod tests {
    macro_rules! ptest {
    ($name:ident : $parser:ident($input:literal) => $out:expr, $tail:literal) => {
        #[test]
        fn $name() {
            assert_eq!(
                $parser.parse($input),
                Ok((
                    $tail,
                    $out
                ))
            );
        }
        };
        ($name:ident : $parser:ident($input:literal) => X) => {
            #[test]
            fn $name() {
                let result = $parser.parse($input);
                assert!(result.is_err(), "{:?}", result);
            }
        };
    }

    pub(super) use ptest;
}