use pest::{Parser, RuleType};
use pest::iterators::Pair;
use crate::protocol::protocol::{Operation, Response};
use snafu::{Backtrace, ResultExt, Snafu, OptionExt};
use std::borrow::Cow;
use std::iter::Peekable;


const SHOULD_ESCAPE: [char; 1] = ['"'];


#[derive(Debug, Snafu)]
pub enum ParseError {
    #[snafu(display("Failed to parse command {}", source.to_string()), context(false))]
    InvalidSyntax {
        source: pest::error::Error<Rule>,
        backtrace: Backtrace
    },
    #[snafu(display("Escape error {}", source.to_string()), context(false))]
    EscapeError {
        source: EscapeError
    },
    #[snafu(display("Server error {}", message))]
    ServerError {
        message: String
    }
}


#[derive(Parser)]
#[grammar = "protocol/protocol.pest"]
struct GrammarParser;

pub fn parse_operation<'a, T: Into<Cow<'a,str>>>(command: T) -> Result<Operation<'a>, ParseError> {
    let command = command.into();

    let mut command = GrammarParser::parse(Rule::operation, &command)?;

    let operation = command.next().unwrap().into_inner().next().unwrap();

    Ok(match operation.as_rule() {
        Rule::get_operation => {
            Operation::GetOperation {
                path: nexth_string(operation, 2)
                    .unwrap()
                    .into()
            }
        },
        Rule::list_operation => Operation::ListOperation,
        _ => unreachable!()
    })
}

pub fn parse_response<'a, T: Into<Cow<'a,str>>>(response: T) -> Result<Response<'a>, ParseError> {
    let response = response.into();

    let mut response = GrammarParser::parse(Rule::operation_response, &response)?;

    let operation = response.next().unwrap().into_inner().next().unwrap();

    match operation.as_rule() {
        Rule::operation_error => {
            let message = nexth_string(operation, 2).unwrap();
            Err(ParseError::ServerError { message })
        },
        Rule::get_operation_response => {
            let content = nexth_string(operation, 2).unwrap();
            Ok(Response::GetOperation { content: interpret_escaped_string(content)?.into() })
        },
        Rule::list_operation_response => {
            let files = operation.into_inner().next().unwrap()
                .into_inner().flat_map(|f| f.into_inner().next())
                .map(|f| f.as_str().to_owned())
                .collect();

            Ok(Response::ListOperation { files })
        }
        _ => unreachable!()
    }
}

fn nexth<T: RuleType>(pair: Pair<T>, amount: i32) -> Option<Pair<T>> {
    (0..amount).fold(Some(pair), |a, _| {
        a.and_then(|b| b.into_inner().next())
    })
}

fn nexth_string<T: RuleType>(pair: Pair<T>, amount: i32) -> Option<String> {
    nexth(pair, amount).map(|v| v.as_str().to_owned())
}


struct InterpretEscapedString<'a> {
    s: std::str::Chars<'a>,
}


#[derive(Debug, Snafu)]
pub enum EscapeError {
    #[snafu(display("Escape at end of the string"))]
    EscapeAtEndOfString,
    #[snafu(display("Invalid escaped character: {}", char))]
    InvalidEscapedChar {
        char: char
    }
}

impl<'a> Iterator for InterpretEscapedString<'a> {
    type Item = Result<char, EscapeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.s.next().map(|c| match c {
            '\\' => match self.s.next() {
                None => Err(EscapeError::EscapeAtEndOfString),
                Some('"') => Ok('"'),
                Some(c) => Err(EscapeError::InvalidEscapedChar { char: c }),
            },
            c => Ok(c),
        })
    }
}

pub(crate) fn interpret_escaped_string<'a, T: Into<Cow<'a,str>>>(s: T) -> Result<String, EscapeError> {
    (InterpretEscapedString { s: s.into().chars() }).collect()
}

struct StringToEscape<'a> {
    s: Peekable<std::str::Chars<'a>>,
    previous_escaped: bool
}

impl<'a> Iterator for StringToEscape<'a> {
    type Item = Result<char, EscapeError>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_ok = |p: &mut Peekable<std::str::Chars>| {
            p.next().map(Ok)
        };

        if self.previous_escaped {
            self.previous_escaped = false;
            return next_ok(&mut self.s);
        }

        let c = match self.s.peek() {
            Some(c) => c,
            None => return None
        };

        if SHOULD_ESCAPE.contains(c) {
            self.previous_escaped = true;
            return Some(Ok('\\'))
        }

        next_ok(&mut self.s)
    }
}

pub(crate) fn escape_string<'a, T: Into<Cow<'a,str>>>(s: T) -> Result<String, EscapeError> {
    (StringToEscape { s: s.into().chars().peekable(), previous_escaped: false }).collect()
}

pub(crate) fn encode_string<'a, T: Into<Cow<'a,str>>>(string: T) -> Cow<'a, str> {
    let string = format!("\"{}\"", escape_string(string).unwrap());
    string.into()
}



#[cfg(test)]
mod tests {
    use crate::protocol::parser::{parse_response, escape_string, interpret_escaped_string};

    #[test]
    fn test_escape() {
        let lock = include_str!("../../../Cargo.lock");
        let es = escape_string(lock).unwrap();
        let des = interpret_escaped_string(es).unwrap();

        assert_eq!(lock, des);

        let a = 123;
    }

}
