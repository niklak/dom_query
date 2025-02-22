use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while1},
    character::complete::{char, multispace0},
    combinator::{map, opt},
    multi::many0,
    sequence::{delimited, pair, preceded},
    IResult, Parser,
};

#[derive(Debug, PartialEq)]
enum Combinator {
    Descendant,
    Child,
    Adjacent,
    Sibling,
}

#[derive(Debug, PartialEq)]
struct Selector {
    tag: Option<String>,
    id: Option<String>,
    class: Option<String>,
    attr: Option<(String, Option<String>)>,
    combinator: Option<Combinator>,
}

fn parse_tag(input: &str) -> IResult<&str, String> {
    map(take_while1(|c: char| c.is_alphanumeric() || c == '-'), String::from).parse(input)
}

fn parse_id(input: &str) -> IResult<&str, String> {
    preceded(char('#'), map(take_while1(|c: char| c.is_alphanumeric() || c == '-'), String::from)).parse(input)
}

fn parse_class(input: &str) -> IResult<&str, String> {
    preceded(char('.'), map(take_while1(|c: char| c.is_alphanumeric() || c == '-'), String::from)).parse(input)
}

fn parse_attr(input: &str) -> IResult<&str, (String, Option<String>)> {
    let key = take_while1(|c: char| c.is_alphanumeric() || c == '-');
    let value = opt(preceded(char('='), delimited(char('"'), is_not("\""), char('"'))));
    delimited(char('['), pair(map(key, String::from), map(value, |v| v.map(String::from))), char(']')).parse(input)
}


fn parse_combinator(input: &str) -> IResult<&str, Combinator> {
    delimited(
        multispace0,
        alt((
            map(tag(">"), |_| Combinator::Child),
            map(tag("+"), |_| Combinator::Adjacent),
            map(tag("~"), |_| Combinator::Sibling),
        )),
        multispace0
    ).parse(input)
}

fn parse_single_selector(input: &str) -> IResult<&str, Selector> {
    let (input, mut combinator) = opt(parse_combinator).parse(input)?;
    let (input, tag) = opt(parse_tag).parse(input)?;
    let (input, id) = opt(parse_id).parse(input)?;
    let (input, class) = opt(parse_class).parse(input)?;
    let (input, attr) = opt(parse_attr).parse(input)?;
    
    
    if tag.is_none() && id.is_none() && class.is_none() && attr.is_none() && combinator.is_none() {
        return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Fail)));
    }
    if combinator.is_none() {
        combinator = Some(Combinator::Descendant);
    }
    let sel = Selector { tag, id, class, attr, combinator };
    Ok((input, sel))
}

fn parse_selector_chain(input: &str) -> IResult<&str, Vec<Selector>> {
    let mut parser = many0(parse_single_selector);
    let (input, selectors) = parser.parse(input)?;
    Ok((input, selectors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_path() {
        let sel = r#"div > a[href="example"] + span.class"#;
        let parsed = parse_selector_chain(sel).unwrap();
        assert_eq!(parsed.1.len(), 3);
    }

   
}
