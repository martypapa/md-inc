use anyhow::{Context, Result};
use nom::bytes::complete::{take, take_until, take_while};
use nom::character::complete::{none_of, space0, space1};
use nom::character::is_alphanumeric;
use nom::combinator::map;
use nom::multi::{count, fold_many0, many0, many_till, separated_nonempty_list};
use nom::sequence::{delimited, pair, separated_pair, tuple};
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, tag},
    character::complete::char,
    IResult,
};
use std::path::PathBuf;
use crate::config::{DEFAULT_TAG_BEGIN, DEFAULT_TAG_END, DEFAULT_END_COMMAND};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Command<'a> {
    command: &'a str,
    args: Vec<&'a str>,
}
impl<'a> Command<'a> {
    pub fn new(command: &'a str, args: Vec<&'a str>) -> Self {
        Self { command, args }
    }
}
#[derive(Debug, Clone, PartialEq)]
struct CommandSec<'a> {
    commands: Vec<Command<'a>>,
    start_remaining: usize,
    end_remaining: usize,
}

impl CommandSec<'_> {
    pub fn start(&self, input: &str) -> usize {
        input.len() - self.start_remaining
    }
    pub fn end(&self, input: &str) -> usize {
        input.len() - self.end_remaining
    }
}

fn wrapped_string(input: &str) -> IResult<&str, &str> {
    let (i, cnt) = fold_many0(tag("#"), 0, |x, _| x + 1)(input)?;
    let (i, _) = tag("\"")(i)?;
    let end = pair(tag("\""), count(tag("#"), cnt));
    let (i, (inner, _)) = many_till(take(1u32), end)(i)?;
    let offset = cnt + 1;
    Ok((i, &input[offset..offset + inner.len()]))
}

static EXTRA_STRING_CHARS: &'static [u8] = "/\\-_.".as_bytes();

fn is_string_char(i: char) -> bool {
    if i.is_ascii() {
        let i = i as u8;
        is_alphanumeric(i) || EXTRA_STRING_CHARS.contains(&i)
    } else {
        false
    }
}

fn inner_string(i: &str) -> IResult<&str, &str> {
    take_while(is_string_char)(i)
}

fn maybe_wrapped_string(i: &str) -> IResult<&str, &str> {
    alt((wrapped_string, inner_string))(i)
}

fn command_args(i: &str) -> IResult<&str, Vec<&str>> {
    let (i, _) = space0(i)?;
    let (i, mut res) = separated_nonempty_list(space1, maybe_wrapped_string)(i)?;
    if matches!(res.last(), Some(x) if x.is_empty()) {
        res.pop();
    }
    Ok((i, res))
}

fn command<'a>(i: &'a str) -> IResult<&'a str, Command> {
    alt((
        map(
            separated_pair(
                maybe_wrapped_string,
                tuple((space0, tag(":"), space0)),
                command_args,
            ),
            move |(command, args): (&'a str, Vec<&'a str>)| Command::new(command, args),
        ),
        map(maybe_wrapped_string, move |command| {
            Command::new(command, vec![])
        }),
    ))(i)
}

fn command_block<'a>(tags: &'a CommandTags, input: &'a str) -> IResult<&'a str, CommandSec<'a>> {
    let start_remaining = input.len();
    let (i, _open) = tag(tags.opening.as_str())(input)?;
    let (i, command_1) = delimited(space0, command, space0)(i)?;
    let (rest, (mut other_commands, _end)) = many_till(
        delimited(delimited(space0, char('|'), space0), command, space0),
        tag(tags.closing.as_str()),
    )(i)?;

    let mut commands = vec![command_1];
    commands.append(&mut other_commands);
    let end_remaining = rest.len();
    Ok((
        rest,
        CommandSec {
            commands,
            start_remaining,
            end_remaining,
        },
    ))
}

// fn end_block(i: &str, end_command: &str) -> IResult<&str, &str> {
//     tuple((tag(begin), space0, tag(end_command), space0, tag(end)))(i)
// }

fn next_command_block<'a>(
    tags: &'a CommandTags,
) -> impl Fn(&'a str) -> IResult<&'a str, CommandSec<'a>> {
    move |i: &'a str| -> IResult<&'a str, CommandSec<'a>> {
        let mut input = i;
        loop {
            // Skip to next match...
            let (i, _) = take_until::<&'a str, &'a str, (&'a str, nom::error::ErrorKind)>(
                &tags.opening,
            )(input)?;
            if let Ok(x) = command_block(tags, i) {
                return Ok(x);
            }
            input = &input[1..];
        }
    }
}
#[derive(Clone, Debug)]
pub struct CommandTags {
    pub opening: String,
    pub closing: String,
}
impl CommandTags {
    pub fn new<S: Into<String>>(opening: S, closing: S) -> Self {
        Self {
            opening: opening.into(),
            closing: closing.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParserConfig {
    pub tags: CommandTags,
    pub end_command: String,
    pub base_dir: PathBuf,
}

impl Default for ParserConfig {
    fn default() -> Self {
        ParserConfig {
            tags: CommandTags::new(DEFAULT_TAG_BEGIN, DEFAULT_TAG_END),
            end_command: DEFAULT_END_COMMAND.to_string(),
            base_dir: std::env::current_dir().unwrap(),
        }
    }
}

pub(crate) enum Span {
    Existing((usize, usize)),
    Replace(String),
}

// pub fn get_args<S: AsRef<str>>(raw_args: S) -> S {
//     // Group into quotes (and remove escapes)
//     let re = regex::Regex::new(r#""()""#)
// }

pub fn escaped<'a>(input: &'a str) -> String {
    let get = move |input: &'a str| -> IResult<&'a str, String> {
        use nom::{alt, tag};
        escaped_transform(none_of("\\"), '\\', |i: &str| {
            alt!(i,
                tag!("\\")       => { |_| "\\" }
              | tag!("\"")       => { |_| "\"" }
              | tag!("n")        => { |_| "\n" }
            )
        })(input)
    };
    get(input)
        .map(|(_, x)| x)
        .unwrap_or_else(|_err| String::new())
}

pub(crate) fn transform<S: AsRef<str>>(input: S, command: &Command) -> Result<String> {
    let input = input.as_ref();
    let args = &command.args;
    let command = command.command;
    Ok(match command {
        "code" => match args.first() {
            Some(language) => format!("```{}\n{}\n```", language, input),
            _ => format!("```\n{}\n```", input),
        },
        "lines" => {
            let from_line = args
                .get(0)
                .map(|x| x.parse().context("Invalid 'from' line"))
                .unwrap_or(Ok(0))?;
            let to_line = args
                .get(1)
                .map(|x| x.parse().context("Invalid 'to' line"))
                .unwrap_or(Ok(input.len()))?;
            (&input
                .lines()
                .skip(from_line)
                .take(to_line - from_line)
                .collect::<Vec<&str>>())
                .join("\n")
        }
        "line" => args
            .iter()
            .map(|x| -> Result<String> {
                let line = x.parse::<usize>().context("Invalid line")?;
                Ok(input
                    .lines()
                    .skip(line)
                    .next()
                    .context("Missing line")?
                    .to_string())
            })
            .collect::<Result<Vec<String>>>()?
            .join("\n"),
        "before" => {
            let value = args.get(0).context("Missing 'before' argument")?;
            format!("{}{}", escaped(value), input)
        }
        "after" => {
            let value = args.get(0).context("Missing 'after' argument")?;
            format!("{}{}", input, escaped(value))
        }
        "wrap" => {
            let before = args.get(0).context("Missing 'before' wrap argument")?;
            let after = args.get(1).unwrap_or(before);
            format!("{}{}{}", escaped(before), input, escaped(after))
        }
        "match" => {
            let re = args.get(0).context("Missing regex string given")?.as_ref();
            let re = regex::Regex::new(re)?;
            let group = args
                .get(1)
                .map(|&x| x.parse().context("Invalid group number"))
                .unwrap_or(Ok(0))?; // Captrue all if no group specified
            let m = re.captures(input).context("Could not find match")?;
            let group = m
                .get(group)
                .with_context(|| format!("Only {} groups in match", m.len()))?;
            input[group.start()..group.end()].to_string()
        }

        // Todo:
        // Structured data (Csv, Json...) - row & column sorting, filtering, into table
        _ => input.to_string(), // No transforms
    })
}

pub struct Parser {
    pub config: ParserConfig,
    pub content: String,
}

impl Parser {
    pub fn new(config: ParserConfig, content: String) -> Self {
        Self { config, content }
    }

    fn command_blocks(&self) -> IResult<&str, Vec<CommandSec>> {
        let cmd = next_command_block(&self.config.tags);
        many0(cmd)(&self.content)
    }

    fn command_groups(&self) -> Result<Vec<(CommandSec, CommandSec)>> {
        let (_, commands) = match self.command_blocks() {
            Ok(x) => x,
            Err(err) => {
                return Err(anyhow::anyhow!("Failed parsing: {}", format!("{}", err)));
            }
        };

        let mut begin_blocks: Vec<CommandSec> = vec![];
        let mut end_blocks: Vec<CommandSec> = vec![];
        for command in commands {
            match command.commands.first() {
                Some(x) if x.command == self.config.end_command => end_blocks.push(command),
                _ => begin_blocks.push(command),
            }
        }

        if begin_blocks.len() != end_blocks.len() {
            return Err(anyhow::anyhow!(
                "Mismatch between command count ({}) and end count ({})",
                begin_blocks.len(),
                end_blocks.len()
            ));
        }
        begin_blocks
            .into_iter()
            .zip(end_blocks)
            .map(|(begin, end)| {
                if begin.end(&self.content) >= end.start(&self.content) {
                    return Err(anyhow::anyhow!(
                        "Found extra end block before command: ({})",
                        begin
                            .commands
                            .into_iter()
                            .map(|x| format!("{:?}", x))
                            .collect::<Vec<_>>()
                            .join(" | ")
                    ));
                }
                Ok((begin, end))
            })
            .collect::<Result<Vec<_>>>()
    }

    ///
    ///
    /// # Parameters
    /// * relative_filepath A filepath relative to `dir`
    pub fn parse(&self) -> Result<String> {
        let groups = self.command_groups()?;
        let mut prev_end = 0;
        let mut spans: Vec<Span> = vec![];
        for (begin, end) in groups {
            let filename = begin.commands.get(0).context("No filename")?.command;
            let filename = self.config.base_dir.join(&filename);
            let contents = std::fs::read_to_string(&filename)
                .context(format!("Could not read: {:?}", &filename))?;
            spans.push(Span::Existing((prev_end, begin.end(&self.content))));
            let mut output = contents.trim().to_string();
            for cmd in begin.commands.iter().skip(1) {
                output = transform(&output, &cmd)?;
            }
            spans.push(Span::Replace(output));
            prev_end = end.start(&self.content);
        }
        spans.push(Span::Existing((prev_end, self.content.len())));

        Ok(spans
            .iter()
            .map(|x| -> &str {
                match x {
                    Span::Existing((begin, end)) => &self.content[*begin..*end],
                    Span::Replace(text) => text.as_str(),
                }
            })
            .collect::<Vec<&str>>()
            .join("\n"))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn strings() {
        assert_eq!(wrapped_string(r#""hello"X"#), Ok(("X", "hello")));
        assert_eq!(wrapped_string(r##"#"hello"#X"##), Ok(("X", "hello")));
        assert_eq!(wrapped_string(r###"##"hello"##X"###), Ok(("X", "hello")));
        assert_eq!(maybe_wrapped_string(r#""hello"X"#), Ok(("X", "hello")));
        assert_eq!(
            maybe_wrapped_string(r#"abc/def\ghi.txt|"#),
            Ok(("|", "abc/def\\ghi.txt"))
        );
        assert_eq!(maybe_wrapped_string(r##"#"hello"#X"##), Ok(("X", "hello")));
        assert_eq!(
            maybe_wrapped_string(r##"#"using " is ok"#X"##),
            Ok(("X", "using \" is ok"))
        );
    }

    #[test]
    fn test_command_args() {
        assert_eq!(
            command_args(r#"one two "three 3" |"#),
            Ok(("|", vec!["one", "two", "three 3"]))
        );
        assert_eq!(command_args(r#" one |"#), Ok(("|", vec!["one"])));
        assert_eq!(command_args(r#" " one " |"#), Ok(("|", vec![" one "])));
        assert_eq!(
            command_args(r#" one "two  three"   | "#),
            Ok(("| ", vec!["one", "two  three"]))
        );
    }
    #[test]
    fn test_command() {
        assert_eq!(
            command("cmd"),
            Ok((
                "",
                Command {
                    command: "cmd",
                    args: vec![]
                }
            ))
        );
        assert_eq!(
            command(r#"cmd: a b "c d""#),
            Ok((
                "",
                Command {
                    command: "cmd",
                    args: vec!["a", "b", "c d"]
                }
            ))
        );
    }

    #[test]
    fn test_command_block() {
        let tags = CommandTags::new("{{", "}}");

        assert_eq!(
            command_block(&tags, "{{ cmd1: a1 | cmd2: a2 a2.1 }} X"),
            Ok((
                " X",
                CommandSec {
                    start_remaining: 32,
                    end_remaining: 2,
                    commands: vec![
                        Command {
                            command: "cmd1",
                            args: vec!["a1"]
                        },
                        Command {
                            command: "cmd2",
                            args: vec!["a2", "a2.1"]
                        },
                    ]
                }
            ))
        );
        let tags = CommandTags::new("<!--{{", "}}-->");
        assert_eq!(
            command_block(&tags, "<!--{{ cmd1: a1 | cmd2: a2 a2.1 }}--> X"),
            Ok((
                " X",
                CommandSec {
                    start_remaining: 39,
                    end_remaining: 2,
                    commands: vec![
                        Command {
                            command: "cmd1",
                            args: vec!["a1"]
                        },
                        Command {
                            command: "cmd2",
                            args: vec!["a2", "a2.1"]
                        },
                    ]
                }
            ))
        );
    }
    #[test]
    fn test_command_block_with_pipe_ends() {
        let tags = CommandTags::new("(|", "|)");
        assert_eq!(
            command_block(&tags, "(| cmd1: a1 | cmd2: a2 a2.1 |) X"),
            Ok((
                " X",
                CommandSec {
                    start_remaining: 32,
                    end_remaining: 2,
                    commands: vec![
                        Command {
                            command: "cmd1",
                            args: vec!["a1"]
                        },
                        Command {
                            command: "cmd2",
                            args: vec!["a2", "a2.1"]
                        },
                    ]
                }
            ))
        );
        let tags = CommandTags::new("(|", "|)");
        assert_eq!(
            command_block(&tags, r#"(|cmd1: a1|cmd2: a2 "a2 |) 3 4"|) X"#),
            Ok((
                " X",
                CommandSec {
                    start_remaining: 35,
                    end_remaining: 2,
                    commands: vec![
                        Command {
                            command: "cmd1",
                            args: vec!["a1"]
                        },
                        Command {
                            command: "cmd2",
                            args: vec!["a2", "a2 |) 3 4"]
                        },
                    ]
                }
            ))
        );
    }
    #[test]
    fn test_next_command_block() {
        let tags = CommandTags::new("(|", "|)");
        assert_eq!(
            next_command_block(&tags)(r#"A(|cmd|)Z"#),
            Ok((
                "Z",
                CommandSec {
                    start_remaining: 8,
                    end_remaining: 1,
                    commands: vec![Command {
                        command: "cmd",
                        args: vec![]
                    },]
                }
            ))
        );
        assert_eq!(
            next_command_block(&tags)(r#"A(|B(|cmd|)Z"#),
            Ok((
                "Z",
                CommandSec {
                    start_remaining: 8,
                    end_remaining: 1,
                    commands: vec![Command {
                        command: "cmd",
                        args: vec![]
                    },]
                }
            ))
        );
        let res = next_command_block(&tags)(r#"A(|nothing"#);
        assert!(res.is_err());
    }
    #[test]
    fn test_command_blocks() {
        let parser = Parser {
            config: ParserConfig {
                tags: CommandTags::new("(|", "|)"),
                end_command: "end".to_string(),
                base_dir: PathBuf::new(),
            },
            content: r#"A(|cmd|)X(|(|end|)Z"#.to_string(),
        };
        let blocks = parser.command_blocks();
        assert_eq!(
            blocks,
            Ok((
                "Z",
                vec![
                    CommandSec {
                        start_remaining: 18,
                        end_remaining: 11,
                        commands: vec![Command {
                            command: "cmd",
                            args: vec![]
                        },]
                    },
                    CommandSec {
                        start_remaining: 8,
                        end_remaining: 1,
                        commands: vec![Command {
                            command: "end",
                            args: vec![]
                        },]
                    }
                ]
            ))
        );
    }

    #[test]
    fn test_command_groups() {
        let parser = Parser {
            config: ParserConfig {
                tags: CommandTags::new("(|", "|)"),
                end_command: "end".to_string(),
                base_dir: PathBuf::new(),
            },
            content: r#"A(|cmd|)X(|(|end|)Z"#.to_string(),
        };
        let groups = parser.command_groups().unwrap();
        assert_eq!(
            groups,
            vec![(
                CommandSec {
                    start_remaining: 18,
                    end_remaining: 11,
                    commands: vec![Command {
                        command: "cmd",
                        args: vec![]
                    },]
                },
                CommandSec {
                    start_remaining: 8,
                    end_remaining: 1,
                    commands: vec![Command {
                        command: "end",
                        args: vec![]
                    },]
                }
            )]
        );
    }
}
