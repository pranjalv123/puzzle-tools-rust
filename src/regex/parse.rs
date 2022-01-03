use nom::branch::alt;
use nom::bytes::complete::{tag};
use nom::character::complete::{anychar};
use nom::combinator::{fail, map, value};
use nom::{IResult};
use nom::character::is_alphanumeric;
use nom::multi::{many1, separated_list1};
use nom::sequence::{delimited, preceded, terminated};

#[derive(Debug, PartialEq, Clone)]
pub struct Pattern {
    pub(crate) elements: Vec<Element>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Element {
    Literal(char),
    Wildcard,
    Set(Vec<char>),
    Repeat(Box<Element>),
    Optional(Box<Element>),
    Group(Vec<Pattern>),
}


pub fn pattern(input: &str) -> IResult<&str, Pattern> {
    map(
        many1(element),
        |x| Pattern { elements: x },
    )(input)
}

fn escaped(input: &str) -> IResult<&str, char> {
    preceded(tag("\\"), anychar)(input)
}

fn get_one_alphanum(input: &str) -> IResult<&str, char> {
    let fst = input.chars().nth(0);
    match fst {
        None => fail(input),
        Some(c) => if is_alphanumeric(c as u8) {
            Ok((&input[1..], c))
        } else { fail(input) }
    }
}

fn literal(input: &str) -> IResult<&str, char> {
    alt((
        escaped,
        get_one_alphanum
    ))(input)
}

fn set(input: &str) -> IResult<&str, Vec<char>> {
    delimited(tag("["), many1(literal), tag("]"))(input)
}

fn group(input: &str) -> IResult<&str, Vec<Pattern>> {
    delimited(tag("("),
              separated_list1(tag("|"), pattern),
              tag(")"))(input)
}

fn modifier_acceptor(input: &str) -> IResult<&str, Element> {
    use Element::*;
    alt((map(literal, Literal),
         map(group, Group),
         map(set, Set)))(input)
}

fn repeat(input: &str) -> IResult<&str, Element> {
    terminated(modifier_acceptor, tag("*"))(input)
}

fn optional(input: &str) -> IResult<&str, Element> {
    terminated(modifier_acceptor, tag("?"))(input)
}

fn wildcard(input: &str) -> IResult<&str, Element> {
    value(Element::Wildcard, tag("."))(input)
}

fn element(input: &str) -> IResult<&str, Element> {
    use Element::*;

    alt((
        map(repeat, |x| Repeat(Box::new(x))),
        map(optional, |x| Optional(Box::new(x))),
        map(set, Set),
        map(group, Group),
        // wildcard and literal have to come after repeat and optional so something like "a*" gets recognized properly
        wildcard,
        map(literal, Literal),
    ))(input)
}


#[cfg(test)]
mod tests {
    use crate::regex::parse::{pattern, Pattern};
    use crate::regex::parse::Element::*;

    #[test]
    fn parse_simple_regex() {
        assert_eq!(pattern("asdf").unwrap().1, Pattern {
            elements: vec![Literal('a'), Literal('s'), Literal('d'), Literal('f')],
        })
    }

    #[test]
    fn parse_with_set() {
        assert_eq!(pattern("asd[gh]f").unwrap().1, Pattern {
            elements: vec![Literal('a'), Literal('s'), Literal('d'), Set(vec!['g', 'h']), Literal('f')],
        })
    }

    #[test]
    fn parse_with_group() {
        assert_eq!(pattern("xy(as|df)").unwrap().1, Pattern {
            elements: vec![Literal('x'), Literal('y'), Group(vec![
                Pattern { elements: vec![Literal('a'), Literal('s')] },
                Pattern { elements: vec![Literal('d'), Literal('f')] },
            ])],
        })
    }

    #[test]
    fn parse_with_star() {

        assert_eq!(pattern("xyb*").unwrap().1, Pattern {
            elements: vec![Literal('x'),
                           Literal('y'),
                           Repeat(Box::from(Literal('b')))]});


        assert_eq!(pattern("xy(foo)*").unwrap().1, Pattern {
            elements: vec![Literal('x'), Literal('y'), Repeat(Box::new(Group(vec![
                pattern("foo").unwrap().1
            ]))),
            ]
        });

        assert_eq!(pattern("xyb*(foo)*").unwrap().1, Pattern {
            elements: vec![Literal('x'), Literal('y'),
                           Repeat(Box::new(
                               Literal('b')
                           )),
                           Repeat(Box::new(Group(vec![
                    pattern("foo").unwrap().1
                ])))
            ]
        });
    }

    #[test]
    fn parse_with_question() {
        assert_eq!(pattern("xy(foo)?").unwrap().1, Pattern {
            elements: vec![Literal('x'), Literal('y'), Optional(Box::new(Group(vec![
                pattern("foo").unwrap().1
            ]))),
            ]
        })
    }
}