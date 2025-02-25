use std::collections::HashMap;

use nom::branch::alt;
use nom::bytes::complete::{is_a, tag, take_while, take};
use nom::character::complete::{alpha1, alphanumeric1, crlf, line_ending, space0, space1};
use nom::combinator::{eof, opt, recognize, value};
use nom::error::ParseError;
use nom::multi::{many0_count, many1};
use nom::sequence::{delimited, pair, preceded, terminated};
use nom::{Err, IResult, Parser as _};


pub struct RootSection {
    pub targets: HashMap<String, TargetSection>
}

pub struct TargetSection {
    pub commands: Vec<Command>,
}

pub enum Command {
    From(FromCommand),
    Run(RunCommand),
}

pub struct FromCommand {
    src: String,
}

pub struct RunCommand {
    cmd: String,
}

fn nl(input: &str) -> IResult<&str, ()> {   
    let nl_or_eof = alt((line_ending, eof));
    preceded(space0, nl_or_eof).map(|_|()).parse(input)
}

fn target_label(input: &str) -> IResult<&str, &str> {
    // TODO: Allow additional characters (which unicode classes?)
    recognize(
        pair(
            alt((alpha1, tag("_"), tag("-"))),
            many0_count(alt((alphanumeric1, tag("_"), tag("-"))))
        )
      ).parse(input)
}

fn indented_block<'a, P, R>(parser: P) -> impl nom::Parser<&'a str, Output=Vec<R>, Error=nom::error::Error<&'a str>>
where
    P: nom::Parser<&'a str, Output=R, Error=nom::error::Error<&'a str>> + Clone,
{
    let mut prefix: Option<String> = None;
    let line_parser = move |s: &'a str| -> IResult<&'a str, R> {
        let parser = parser.clone();
        if let Some(p) = prefix.as_deref() {
            let try_prefix = tag(p).map(|_|());
            return Ok(delimited(try_prefix, parser, nl).parse(s)?);
        }

        let first_line = terminated((space1, parser), nl).parse(s)?;
        prefix = Some(first_line.0.to_owned());
        Ok(first_line.1)
    };

    many1(line_parser)
}

fn parse_target_command(input: &str) -> IResult<&str, Command> {
    todo!()
}

fn parse_target_section(input: &str) -> IResult<&str, TargetSection> {
    let items = indented_block(parse_target_command).parse(input)?;

    todo!()
}

fn parse_root_child(input: &str) -> IResult<&str, (String, TargetSection)> {
    let with_prefix = terminated(tag("TARGET"), space1);
    let colon_end = (tag(":"), nl);
    let label = terminated(preceded(opt(with_prefix), target_label), colon_end);
    
    (label.map(|s: &str| s.to_owned()), parse_target_section).parse(input)
}

pub fn parse(input: &str) -> IResult<&str, RootSection> {
    let sections = many1(parse_root_child).parse(input)?.1;
    Ok((input, RootSection {
        targets: sections.into_iter().collect()
    }))
}

