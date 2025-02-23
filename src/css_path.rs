use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while1},
    character::complete::{char, multispace0},
    combinator::{map, opt},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded},
    IResult, Parser,
};

use crate::{node::TreeNode, Element};

#[derive(Debug, PartialEq)]
enum Combinator {
    Descendant,
    Child,
    Adjacent,
    Sibling,
}

#[derive(Debug, PartialEq)]
pub struct Selector<'a> {
    name: Option<&'a str>,
    id: Option<&'a str>,
    classes: Option<Vec<&'a str>>,
    attr: Option<(&'a str, Option<&'a str>)>,
    combinator: Option<Combinator>,
}

impl <'a>Selector<'a> {
    pub fn match_node(&self, t: &TreeNode) -> bool {
        if let Some(el) = t.as_element() {
            if !self.match_name(el) {
                return false;
            }
            if !self.match_id_attr(el) {
                return false;
            }
            if !self.match_classes(el) {
                return false;
            }
            if !self.match_attr(el) {
                return false;
            }
            true

        } else {
            false
        }
    }

    fn match_name(&self, el: &Element) -> bool {
        self.name.map_or(true, |name| &el.name.local == name)
    }

    fn match_id_attr(&self, el: &Element) -> bool {
        if let Some(id) = self.id {
            if let Some(id_attr) = el.id() {
                return id_attr.as_ref() == id;
            }
        }
        true
    }
    fn match_classes(&self, el: &Element) -> bool {
        let Some(ref classes) = self.classes else {
            return true;
        };
        classes.iter().all(|class|el.has_class(class))
    }

    fn match_attr(&self, el: &Element) -> bool {
        if let Some((key, value)) = self.attr {
            if let Some(v) = value {
                if let Some(attr_value) = el.attr(key) {
                    return attr_value.as_ref() == v
                }else {
                    return false;
                }
            } else {
                return el.has_attr(key)
            }
        }
        true
    }
}

fn parse_name(input: &str) -> IResult<&str, &str> {
    map(
        take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-'),
        |s| s,
    )
    .parse(input)
}

fn parse_id(input: &str) -> IResult<&str, &str> {
    preceded(
        char('#'),
        map(
            take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-'),
            |s| s,
        ),
    )
    .parse(input)
}

fn parse_classes(input: &str) -> IResult<&str, Vec<&str>> {
    many1(preceded(
        char('.'),
        map(
            take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-'),
            |s| s,
        ),
    ))
    .parse(input)
    .map(|(input, classes)| (input, classes.into_iter().collect()))
}

fn parse_attr(input: &str) -> IResult<&str, (&str, Option<&str>)> {
    let key = take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-');
    let value = opt(preceded(
        char('='),
        delimited(char('"'), is_not("\""), char('"')),
    ));
    delimited(
        char('['),
        pair(map(key, |k| k), map(value, |v| v)),
        char(']'),
    )
    .parse(input)
}

fn parse_combinator(input: &str) -> IResult<&str, Combinator> {
    delimited(
        multispace0,
        alt((
            map(tag(">"), |_| Combinator::Child),
            map(tag("+"), |_| Combinator::Adjacent),
            map(tag("~"), |_| Combinator::Sibling),
        )),
        multispace0,
    )
    .parse(input)
}

fn parse_single_selector(input: &str) -> IResult<&str, Selector> {
    let (input, mut combinator) = opt(parse_combinator).parse(input)?;
    let (input, name) = opt(parse_name).parse(input)?;
    let (input, id) = opt(parse_id).parse(input)?;
    let (input, classes) = opt(parse_classes).parse(input)?;
    let (input, attr) = opt(parse_attr).parse(input)?;

    if name.is_none() && id.is_none() && classes.is_none() && attr.is_none() && combinator.is_none()
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Fail,
        )));
    }
    if combinator.is_none() {
        combinator = Some(Combinator::Descendant);
    }
    let sel = Selector {
        name,
        id,
        classes,
        attr,
        combinator,
    };
    Ok((input, sel))
}

pub fn parse_selector_chain(input: &str) -> IResult<&str, Vec<Selector>> {
    let mut parser = many0(delimited(multispace0, parse_single_selector, multispace0));
    let (input, selectors) = parser.parse(input)?;
    Ok((input, selectors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_path() {
        let sel = r#"div > a[href="example"] + span.class-1.class-2"#;
        let parsed = parse_selector_chain(sel).unwrap();
        let expected = vec![
            Selector { name: Some("div"), id: None, classes: None, attr: None, combinator: Some(Combinator::Descendant) },
            Selector { name: Some("a"), id: None, classes: None, attr: Some(("href", Some("example"))), combinator: Some(Combinator::Child) },
            Selector { name: Some("span"), id: None, classes: Some(vec!["class-1", "class-2"]), attr: None, combinator: Some(Combinator::Adjacent) }

        ];
        assert_eq!(parsed.1, expected);
    }

    #[test]
    fn test_names() {
        let sel = r#"body td a"#;
        let parsed = parse_selector_chain(sel).unwrap();
        assert_eq!(parsed.1.len(), 3);
    }
}
