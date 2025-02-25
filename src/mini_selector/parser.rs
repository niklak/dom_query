use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while1},
    character::complete::{char, multispace0},
    combinator::{cut, map, opt},
    multi::{many0, many1},
    sequence::{delimited, preceded, terminated},
    IResult, Parser,
};

use super::selector::{AttrOperator, Attribute, Combinator, MiniSelector};

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

fn parse_attr_operator(input: &str) -> IResult<&str, AttrOperator> {
    alt((
        map(tag("~="), |_| AttrOperator::Includes),
        map(tag("|="), |_| AttrOperator::DashMatch),
        map(tag("^="), |_| AttrOperator::Prefix),
        map(tag("$="), |_| AttrOperator::Suffix),
        map(tag("*="), |_| AttrOperator::Substring),
        map(tag("="), |_| AttrOperator::Equals),
    ))
    .parse(input)
}

fn parse_attr(input: &str) -> IResult<&str, Attribute> {
    let key = take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-');
    let op = opt(parse_attr_operator);
    let value = opt(preceded(
        char('"'),
        cut(terminated(is_not("\""), char('"'))),
    ));

    let (input, (k, op, v)) =
        delimited(char('['), (map(key, |k| k), op, value), char(']')).parse(input)?;

    Ok((
        input,
        Attribute {
            key: k,
            op,
            value: v,
        },
    ))
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

pub fn parse_single_selector(input: &str) -> IResult<&str, MiniSelector> {
    let (input, combinator) = opt(parse_combinator).parse(input)?;
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

    let combinator = combinator.unwrap_or(Combinator::Descendant);

    let sel = MiniSelector {
        name,
        id,
        classes,
        attr,
        combinator,
    };
    Ok((input, sel))
}

pub fn parse_selector_list(input: &str) -> IResult<&str, Vec<MiniSelector>> {
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
        let parsed = parse_selector_list(sel).unwrap();
        let expected = vec![
            MiniSelector {
                name: Some("div"),
                id: None,
                classes: None,
                attr: None,
                combinator: Combinator::Descendant,
            },
            MiniSelector {
                name: Some("a"),
                id: None,
                classes: None,
                attr: Some(Attribute {
                    key: "href",
                    op: Some(AttrOperator::Equals),
                    value: Some("example"),
                }),
                combinator: Combinator::Child,
            },
            MiniSelector {
                name: Some("span"),
                id: None,
                classes: Some(vec!["class-1", "class-2"]),
                attr: None,
                combinator: Combinator::Adjacent,
            },
        ];
        assert_eq!(parsed.1, expected);
    }

    #[test]
    fn test_names() {
        let sel = r#"body td a"#;
        let parsed = parse_selector_list(sel).unwrap();
        assert_eq!(parsed.1.len(), 3);
    }
    #[test]
    fn test_attr_operators() {
        let test_cases = vec![
            (
                "span[title]",
                Some(Attribute {
                    key: "title",
                    op: None,
                    value: None,
                }),
            ),
            (
                r##"span[title="Title"]"##,
                Some(Attribute {
                    key: "title",
                    op: Some(AttrOperator::Equals),
                    value: Some("Title"),
                }),
            ),
            (
                r##"span[title~="Title"]"##,
                Some(Attribute {
                    key: "title",
                    op: Some(AttrOperator::Includes),
                    value: Some("Title"),
                }),
            ),
            (
                r##"span[title|="Title"]"##,
                Some(Attribute {
                    key: "title",
                    op: Some(AttrOperator::DashMatch),
                    value: Some("Title"),
                }),
            ),
            (
                r##"span[title^="Title"]"##,
                Some(Attribute {
                    key: "title",
                    op: Some(AttrOperator::Prefix),
                    value: Some("Title"),
                }),
            ),
            (
                r##"span[title$="Title"]"##,
                Some(Attribute {
                    key: "title",
                    op: Some(AttrOperator::Suffix),
                    value: Some("Title"),
                }),
            ),
            (
                r##"span[title*="Title"]"##,
                Some(Attribute {
                    key: "title",
                    op: Some(AttrOperator::Substring),
                    value: Some("Title"),
                }),
            ),
            (r##"span[title ="Title"]"##, None),
            (r##"span[title**"Title"]"##, None),
        ];

        for test in test_cases {
            let parsed = parse_single_selector(test.0).unwrap();
            let expected = MiniSelector {
                name: Some("span"),
                id: None,
                classes: None,
                attr: test.1,
                combinator: Combinator::Descendant,
            };
            assert_eq!(parsed.1, expected);
        }
    }

    #[test]
    fn test_mini_selector() {
        let sel = r#"a#main-link.main-class.extra-class[href="https://example.com"]"#;
        let parsed = parse_single_selector(sel).unwrap();
        let expected = MiniSelector {
            name: Some("a"),
            id: Some("main-link"),
            classes: Some(vec!["main-class", "extra-class"]),
            attr: Some(Attribute {
                key: "href",
                op: Some(AttrOperator::Equals),
                value: Some("https://example.com"),
            }),
            combinator: Combinator::Descendant,
        };
        assert_eq!(parsed.1, expected);
    }
}
