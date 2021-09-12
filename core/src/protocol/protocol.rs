use std::borrow::Cow;
use tokio_util::codec::{Encoder, Decoder};
use bytes::{BytesMut, Bytes};
use snafu::{Backtrace, ResultExt, Snafu, OptionExt};
use crate::protocol::parser::{ParseError, parse_operation, parse_response, encode_string};
use std::string::FromUtf8Error;
use log::debug;


#[derive(Debug, Snafu)]
pub enum ProtocolError {
    #[snafu(context(false))]
    ParseError {
        source: ParseError
    },
    #[snafu(context(false))]
    IOError {
        source: std::io::Error
    },
    #[snafu(display("Protocol decoding UTF-8 String failed {}", source.to_string()), context(false))]
    EncodingError {
        source: FromUtf8Error
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Operation<'a> {
    GetOperation {
        path: Cow<'a, str>
    },
    ListOperation
}

#[derive(Debug, PartialEq, Clone)]
pub enum Response<'a> {
    GetOperation {
       content: Cow<'a, str>
    },
    ListOperation {
        files: Vec<String>
    }
}


impl<'a> From<Operation<'a>> for Vec<u8> {
    fn from(o: Operation<'a>) -> Self {
        match o {
            Operation::GetOperation { path } =>
                format!("GET {}", encode_string(path)).as_bytes().to_vec(),
            Operation::ListOperation => "LIST".as_bytes().to_vec()
        }
    }
}

impl<'a> From<Response<'a>> for Vec<u8> {
    fn from(r: Response<'a>) -> Self {
        let mut buffer = BytesMut::from("OK ".as_bytes());

        match r {
            Response::GetOperation { content } => {
                buffer.extend("GET ".as_bytes());
                buffer.extend(encode_string(content).as_bytes())
            },
            Response::ListOperation { files } => {
                buffer.extend("LIST ".as_bytes());

                let files: Vec<u8> = files.into_iter()
                    .map(encode_string)
                    .reduce(|a, b| { format!("{},{}", a, b).into() })
                    .map_or("".into(), |s| s.as_bytes().into());

                buffer.extend(files);
            }
        };

        buffer.to_vec()
    }
}


pub struct ProtocolOperationCodec;

impl Decoder for ProtocolOperationCodec {
    type Item = Operation<'static>;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None)
        }

        let command = String::from_utf8(src.to_vec())?;

        debug!("Parsing the command: {}", command);

        let command = parse_operation(command)?;

        src.clear();

        Ok(Some(command))
    }
}

impl Encoder<Operation<'static>> for ProtocolOperationCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Operation<'static>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes: Vec<u8> = item.into();

        dst.extend(bytes);

        Ok(())
    }
}

pub struct ProtocolResponseCodec;

impl Encoder<Response<'static>> for ProtocolResponseCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: Response, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes: Vec<u8> = item.into();

        dst.extend(bytes);

        Ok(())
    }
}

impl Decoder for ProtocolResponseCodec {
    type Item = Response<'static>;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None)
        }

        let response = String::from_utf8(src.to_vec())?;

        debug!("Parsing the response: {}", response);

        let response = parse_response(response)?;

        src.clear();

        Ok(Some(response))
    }
}