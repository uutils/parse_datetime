use nom::bytes::complete::take_while1;
use nom::character::complete;
use nom::character::complete::one_of;
use nom::combinator::all_consuming;
use nom::sequence::preceded;
use nom::Parser;

use crate::parse_items::PResult;

pub(crate) fn nano_seconds(input: &str) -> PResult<u32> {
    let (tail, fraction) =
        preceded(one_of(",."), take_while1(|c: char| c.is_ascii_digit())).parse(input)?;

    let digits_used = fraction.len().min(9);
    let ns_per_frac = 10u32.pow(9 - digits_used as u32);

    let (_, fraction) = all_consuming(complete::u32).parse(&fraction[..digits_used])?;
    let nanoseconds = fraction * ns_per_frac;

    Ok((tail, nanoseconds))
}

#[cfg(test)]
mod tests {
    use crate::parse_items::tests::ptest;

    use super::*;

    macro_rules! ns {
        ($name:ident : $input:literal => $nano_seconds:literal + $tail:literal) => {
            ptest! { $name : nano_seconds($input) => $nano_seconds, $tail }
        };
        ($name:ident : $input:literal => X) => {
            ptest! { $name : nano_seconds($input) => X }
        };
    }

    ns! { without_digits : "." => X }
    ns! { one : ".1" => 100000000 + "" }
    ns! { comma : ",123" => 123000000 + "" }
    ns! { nine : ".123456789" => 123456789 + "" }
    ns! { more : ".123456789101112" => 123456789 + "" }
    ns! { negative : ".-123456789" => X }
}
