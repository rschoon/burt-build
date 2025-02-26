use std::collections::HashMap;

use anyhow::anyhow;
use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, is_a, tag, take, take_while};
use nom::character::complete::{alpha1, alphanumeric1, char, crlf, line_ending, multispace0, not_line_ending};
use nom::combinator::{complete, cut, eof, opt, recognize, value};
use nom::error::ParseError;
use nom::multi::{fold, many0, many0_count, many1, many1_count, separated_list0};
use nom::sequence::{delimited, pair, preceded, terminated};
use nom::{Err, IResult, Parser as _};

#[derive(Debug)]
pub struct RootSection {
    pub targets: HashMap<String, TargetSection>
}

#[derive(Debug)]
pub struct TargetSection {
    pub commands: Vec<Command>,
}

#[derive(Debug)]
pub enum Command {
    From(FromCommand),
    Run(RunCommand),
}

#[derive(Debug)]
pub struct FromCommand {
    src: String,
}

#[derive(Debug)]
pub struct RunCommand {
    cmd: RunCommandArgs,
}

#[derive(Debug)]
pub enum RunCommandArgs {
    List(Vec<String>),
    String(String),
}

fn some_space(input: &str) -> IResult<&str, &str> {
    alt((is_a(" \t"), tag("\\\n"))).parse(input)
}

fn not_whitespace(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| !c.is_ascii_whitespace()).parse(input)
}

fn space0(input: &str) -> IResult<&str, &str> {
    recognize(many0_count(some_space)).parse(input)
}

fn space1(input: &str) -> IResult<&str, &str> {
    recognize(many1_count(some_space)).parse(input)
}

fn comment(input: &str) -> IResult<&str, ()> {
    (space0, tag("#"), opt(not_line_ending)).map(|_|()).parse(input)
}

fn nl(input: &str) -> IResult<&str, ()> {
    many1((space0, opt(comment), alt((line_ending, eof)))).map(|_|()).parse(input)
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
    // TODO: fixme replace with fold
    // line_parser will get called multiple times for just the first line

    let mut prefix: Option<String> = None;
    let line_parser = move |s: &'a str| -> IResult<&'a str, R> {
        let parser = parser.clone();
        if let Some(p) = prefix.as_deref() {
            let try_prefix = tag(p).map(|_|());
            return preceded(try_prefix, parser).parse(s);
        }

        let first_line = (space1, parser).parse(s)?;
        prefix = Some(first_line.1.0.to_owned());
        Ok(first_line.1)
    };

    many1(line_parser)
}

fn command_string(input: &str) -> IResult<&str, &str> {
    recognize(many0_count(alt((some_space, not_whitespace)))).parse(input)
}

fn string(input: &str) -> IResult<&str, String> {
    delimited(
        char('"'),
        escaped_transform(
            alpha1,
            '\\',
            alt((
                value("\\", tag("\\")),
                value("\"", tag("\"")),
                value("\n", tag("n")),
            ))
        ),
        char('"'),
    ).parse(input)
}

fn string_list(input: &str) -> IResult<&str, Vec<String>> {
    preceded(
        char('['),
        cut(terminated(
            separated_list0(preceded(some_space, char(',')), string),
            preceded(multispace0, char(']'))
    ))).parse(input)
}

fn parse_from_command(input: &str) -> IResult<&str, FromCommand> {
    (tag("FROM"), space1, not_whitespace, nl).map(|r| {
        FromCommand {
            src: r.2.to_owned()
        }
    }).parse(input)
}

fn parse_run_command(input: &str) -> IResult<&str, RunCommand> {
    (tag("RUN"), space1, alt((
        string_list.map(RunCommandArgs::List),
        command_string.map(|r| RunCommandArgs::String(r.to_owned()))
    )), nl).map(|r| {
        RunCommand {
            cmd: r.2
        }
    }).parse(input)
}

fn parse_target_command(input: &str) -> IResult<&str, Command> {
    macro_rules! cmd {
        ($name:ident($type:ident), $func:path) => {
            nom::combinator::map($func, |s: $type| Command::$name(s))
        }
    }
    alt((
        cmd!(From(FromCommand), parse_from_command),
        cmd!(Run(RunCommand), parse_run_command),
    )).parse(input)
}

fn parse_target_section(input: &str) -> IResult<&str, TargetSection> {
    let items = indented_block(parse_target_command).parse(input)?;

    Ok((items.0, TargetSection {
        commands: items.1
    }))
}

fn parse_root_child(input: &str) -> IResult<&str, (String, TargetSection)> {
    let with_prefix = terminated(tag("TARGET"), space1);
    let colon_end = (tag(":"), nl);
    let label = terminated(preceded((opt(nl), opt(with_prefix)), target_label), colon_end);
    
    (label.map(|s: &str| s.to_owned()), parse_target_section).parse(input)
}

fn parse_root(input: &str) -> IResult<&str, RootSection> {
    let sections = preceded(opt(nl), many1(parse_root_child)).parse(input)?.1;
    Ok((input, RootSection {
        targets: sections.into_iter().collect()
    }))
}

pub fn parse(input: &str) -> anyhow::Result<RootSection> {
    let result = complete(parse_root).parse(input).map_err(|e| anyhow::format_err!("{}", e))?;
    Ok(result.1)
}

pub fn parse_reader<R>(mut reader: R) -> anyhow::Result<RootSection>
where
    R: std::io::Read
{
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let buffer = String::from_utf8(buffer)?;
    parse(&buffer)
}
