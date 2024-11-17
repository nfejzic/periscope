use nom::{bytes::complete, character, combinator, sequence};

/// Parses an unsigned integer.
pub fn uint(input: &str) -> nom::IResult<&str, u64> {
    combinator::map_res(character::complete::digit1, |s: &str| s.parse())(input)
}

/// Parses a newline (`\n`).
pub fn newline(input: &str) -> nom::IResult<&str, &str> {
    complete::tag("\n")(input)
}

/// Parses BTOR2 comments.
pub fn comment(input: &str) -> nom::IResult<&str, ()> {
    let first = sequence::preceded(complete::tag(";"), complete::take_until("\\n"));
    combinator::map(sequence::terminated(first, newline), |_| ())(input)
}
