use std::fmt::{Debug, Display};

#[derive(Debug, Eq, PartialEq)]
pub struct ParseError<I> {
    errors: Vec<(I, ParseErrorKind)>
}

impl<I: Display + Debug> ParseError<I> {
    pub fn extract_error(self, input: &str) -> anyhow::Error {
        let pos = if let Some(e) = self.errors.get(0) {
            e.0.to_string()
        } else {
            String::new()
        };
        let (line, column) = find_position(input, &pos);
        anyhow::anyhow!("line {line}:{column}: {}", self)
    }

    fn find_context(&self) -> Option<&'static str> {
        self.errors.iter().find_map(|e| {
            if let ParseErrorKind::Context(c) = e.1 {
                Some(c)
            } else {
                None
            }
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ParseErrorKind {
    Context(&'static str),
    Nom(nom::error::ErrorKind),
}

impl<T: Display + Debug> std::error::Error for ParseError<T> {}

impl<T: Display + Debug> Display for ParseError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(c) = self.find_context() {
            write!(f, "expected {c}")
        } else if let Some(e) = self.errors.first() {
            write!(f, "error {:?}", e.1)
        } else {
            write!(f, "error")
        }
    }
}

impl<T> nom::error::ParseError<T> for ParseError<T> {
    fn from_error_kind(input: T, kind: nom::error::ErrorKind) -> Self {
        Self {
            errors: vec![(input, ParseErrorKind::Nom(kind))]
        }
    }

    fn append(input: T, kind: nom::error::ErrorKind, mut other: Self) -> Self {
        other.errors.push((input, ParseErrorKind::Nom(kind)));
        other
    }
}

impl<T> nom::error::ContextError<T> for ParseError<T> {
    fn add_context(input: T, ctx: &'static str, mut other: Self) -> Self {
        other.errors.push((input, ParseErrorKind::Context(ctx)));
        other
    }
}

impl<T, E> nom::error::FromExternalError<T, E> for ParseError<T> {
    fn from_external_error(input: T, kind: nom::error::ErrorKind, _e: E) -> Self {
        Self {
            errors: vec![(input, ParseErrorKind::Nom(kind))]
        }
    }
}

fn find_position(input: &str, pos: &str) -> (usize, usize) {
    let prefix = &input[..input.len()-pos.len()];
    let mut pos = 0;
    let mut line = 0;
    while let Some(next_nl) = prefix[pos..].find('\n') {
        line += 1;
        pos += next_nl + 1;
    }
    (line, prefix[pos..].len())
}
