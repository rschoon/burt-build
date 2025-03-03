
use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, is_a, tag, take_until, take_while1};
use nom::character::complete::{alpha1, alphanumeric1, char, line_ending, multispace0, not_line_ending};
use nom::combinator::{all_consuming, cut, eof, opt, recognize, value};
use nom::error::context;
use nom::multi::{many0_count, many1, many1_count, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, preceded, terminated};
use nom::{Finish, IResult, Parser as _};

use super::types::*;
use super::error::ParseError;

type ParseResult<'a, T> = IResult<&'a str, T, ParseError<&'a str>>;

fn some_space(input: &str) -> ParseResult<&str> {
    alt((is_a(" \t"), tag("\\\n"))).parse(input)
}

fn not_whitespace1(input: &str) -> ParseResult<&str> {
    take_while1(|c: char| !c.is_ascii_whitespace()).parse(input)
}

fn jinja_nonspace(input: &str) -> ParseResult<&str> {
    let jinja_block1 = (tag("{{"), cut((take_until("}}"), tag("}}"))));
    let jinja_block2 = (tag("{%"), cut((take_until("%}"), tag("%}"))));
    let any_part = alt((recognize(jinja_block1), recognize(jinja_block2), not_whitespace1));

    recognize(many1_count(any_part)).parse(input)
}

fn space0(input: &str) -> ParseResult<&str> {
    recognize(many0_count(some_space)).parse(input)
}

fn space1(input: &str) -> ParseResult<&str> {
    recognize(many1_count(some_space)).parse(input)
}

fn comment(input: &str) -> ParseResult<()> {
    (space0, tag("#"), opt(not_line_ending)).map(|_|()).parse(input)
}

fn nl_final_empty(input: &str) -> ParseResult<&str> {
    recognize((space0, opt(comment), eof)).parse(input)
}

fn nl(input: &str) -> ParseResult<()> {
    let empty_lines = recognize(many1_count((space0, opt(comment), line_ending)));
    alt((recognize((empty_lines, recognize(opt(nl_final_empty)))), nl_final_empty)).map(|_|()).parse(input)
}

fn target_label(input: &str) -> ParseResult<&str> {
    // TODO: Allow additional characters (which unicode classes?)
    recognize(
        pair(
            alt((alpha1, tag("_"), tag("-"))),
            many0_count(alt((alphanumeric1, tag("_"), tag("-"))))
        )
      ).parse(input)
}

fn indented_block<'a, P, R>(parser: P) -> impl nom::Parser<&'a str, Output=Vec<R>, Error=ParseError<&'a str>>
where
    P: nom::Parser<&'a str, Output=R, Error=ParseError<&'a str>> + Clone,
{
    space1.flat_map(move |r| {
        separated_list1(tag(r), parser.clone())
    })
}

fn command_string(input: &str) -> ParseResult<&str> {
    recognize(many1_count(alt((some_space, jinja_nonspace)))).parse(input)
}

fn json_string(input: &str) -> ParseResult<String> {
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

fn arg_string(input: &str) -> ParseResult<String> {
    alt((json_string, not_whitespace1.map(|s| s.to_owned()))).parse(input)
}

fn string_list(input: &str) -> ParseResult<Vec<String>> {
    preceded(
        char('['),
        cut(terminated(
            separated_list0(preceded(multispace0, char(',')), json_string),
            preceded(multispace0, char(']'))
    ))).parse(input)
}

fn parse_from_command(input: &str) -> ParseResult<FromCommand> {
    (tag("FROM"), space1, not_whitespace1, nl).map(|r| {
        FromCommand {
            src: r.2.to_owned()
        }
    }).parse(input)
}

fn parse_run_command(input: &str) -> ParseResult<RunCommand> {
    let options = alt((
        string_list.map(RunCommandArgs::List),
        command_string.map(|r| RunCommandArgs::String(r.to_owned()))
    ));

    (tag("RUN"), space1, options, nl).map(|r| {
        RunCommand {
            cmd: r.2
        }
    }).parse(input)
}

fn parse_save_artifact_command(input: &str) -> ParseResult<SaveArtifactCommand> {
    let cmd_prefix = (tag("SAVE"), space1, tag("ARTIFACT"), space1);

    (cmd_prefix, arg_string, opt(preceded(space1, arg_string)), nl).map(|r| {
        SaveArtifactCommand {
            src: r.1,
            dest: r.2,
        }
    }).parse(input)
}

fn parse_target_command(input: &str) -> ParseResult<Command> {
    macro_rules! cmd {
        ($name:ident($type:ident), $func:path) => {
            nom::combinator::map($func, |s: $type| Command::$name(s))
        }
    }
    context(
        "target command", 
        cut(alt((
            cmd!(From(FromCommand), parse_from_command),
            cmd!(Run(RunCommand), parse_run_command),
            cmd!(SaveArtifact(SaveArtifactCommand), parse_save_artifact_command),
        )))
    ).parse(input)
}

fn parse_target_section(input: &str) -> ParseResult<TargetSection> {
    let items = indented_block(parse_target_command).parse(input)?;

    Ok((items.0, TargetSection {
        commands: items.1
    }))
}

fn parse_root_child(input: &str) -> ParseResult<(String, TargetSection)> {
    let with_prefix = terminated(tag("TARGET"), space1);
    let colon_end = (tag(":"), nl);
    let label = terminated(preceded((opt(nl), opt(with_prefix)), target_label), colon_end);
    context(
        "target or other top level item",
        (label.map(|s: &str| s.to_owned()), parse_target_section)
    ).parse(input)
}

fn parse_root(input: &str) -> ParseResult<RootSection> {
    preceded(opt(nl), many1(parse_root_child))
        .map(|r| {
            RootSection {
                targets: r.into_iter().collect()
            }
        })
        .parse(input)
}

pub fn parse(input: &str) -> anyhow::Result<RootSection> {
    let result = all_consuming(parse_root).parse(input).finish().map_err(|e| e.extract_error(input))?;
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nl() {
        assert_eq!(nl("\n"), Ok(("", ())));
        assert_eq!(nl("\na"), Ok(("a", ())));
        assert_eq!(nl(""), Ok(("", ())));
        assert!(nl("a").is_err());
        assert!(nl("a\n").is_err());
        assert!(nl("a\na").is_err());
    }

    #[test]
    fn test_command_string() {
        assert_eq!(command_string("abba"), Ok(("", "abba")));
        assert_eq!(command_string("hello world"), Ok(("", "hello world")));
        assert_eq!(command_string("abba\nba"), Ok(("\nba", "abba")));
        assert_eq!(command_string("hello world\ncya"), Ok(("\ncya", "hello world")));
        assert!(command_string("").is_err());
    }

    #[test]
    fn test_run_command() {
        assert_eq!(parse_run_command("RUN hello"), Ok(("", RunCommand { cmd: RunCommandArgs::String("hello".to_owned())})));
        assert_eq!(parse_run_command("RUN hello\nnext"), Ok(("next", RunCommand { cmd: RunCommandArgs::String("hello".to_owned())})));
    }

    #[test]
    fn test_indented_block_simple() {
        let line = |s| terminated(nom::bytes::take(1u8), nl).parse(s);

        assert_eq!(indented_block(line).parse(" a\n b\n"), Ok(("", vec!["a", "b"])));
        assert_eq!(indented_block(line).parse(" a\nb\n"), Ok(("b\n", vec!["a"])));
        assert_eq!(indented_block(line).parse(" a\n b"), Ok(("", vec!["a", "b"])));
    }

    #[test]
    fn test_indented_block_multiline() {
        let statement = |s| {
            terminated(nom::bytes::take_while(|c| c != ';'), tag(";\n")).parse(s)
        };

        assert_eq!(indented_block(statement).parse(" a\n b;\n"), Ok(("", vec!["a\n b"])));
        assert!(indented_block(statement).parse(" a\nb\n").is_err());
    }
}
