use std::mem;

use slog::{slog_debug, slog_trace};
use slog_scope::{debug, trace};

use bytes::BytesMut;

use crate::protocol::{Command, Response};

use super::{DecodeError, EncodeError, ProtocolCodec};

#[derive(Debug, Clone)]
pub enum Value {
    SimpleString(BytesMut),
    Error(BytesMut),
    Integer(i64),
    BulkString(BytesMut),
    Array(Vec<Value>),
    Null,
}
impl<'a> Value {
    fn read_from(buffer: &mut BytesMut) -> Result<Option<Self>, DecodeError> {
        fn read_simple(buffer: &mut BytesMut) -> Result<Option<BytesMut>, DecodeError> {
            let pos = match buffer.iter().position(|&x| x == b'\r' || x == b'\n') {
                Some(pos) => pos,
                None => return Ok(None),
            };
            if pos + 1 == buffer.len() {
                return Ok(None);
            }
            if buffer[pos] != b'\r' {
                return Err(DecodeError::UnexpectedByte(buffer[pos]));
            }
            if buffer[pos + 1] != b'\n' {
                return Err(DecodeError::UnexpectedByte(buffer[pos + 1]));
            }
            return Ok(Some(buffer.split_to(pos + 2)));
        }

        debug!("Attempting to parse RESPv2 value");

        let mut original = buffer.clone();
        trace!("Buffer: {:?}", original);

        match buffer[0] {
            b'+' => Ok(read_simple(buffer)?.map(|mut command| {
                command.advance(1);
                Value::SimpleString(command.split_to(command.len() - 2))
            })),
            b'-' => Ok(read_simple(buffer)?.map(|mut command| {
                command.advance(1);
                Value::Error(command.split_to(command.len() - 2))
            })),
            b':' => read_simple(buffer)?
                .map(|command| -> Result<Value, DecodeError> {
                    let repr = std::str::from_utf8(&command[1..command.len() - 2])?;
                    trace!("Parsing RESPv2 integer from '{}'", repr);
                    Ok(Value::Integer(repr.parse()?))
                })
                .transpose(),
            b'*' => {
                if let Some(len) = read_simple(buffer)?
                    .map(|command| -> Result<isize, DecodeError> {
                        let repr = std::str::from_utf8(&command[1..command.len() - 2])?;
                        trace!("Parsing RESPv2 array size from '{}'", repr);
                        Ok(repr.parse()?)
                    })
                    .transpose()
                    .map_err(|err| {
                        mem::swap(&mut original, buffer);
                        err
                    })?
                {
                    if len == -1 {
                        return Ok(Some(Value::Null));
                    }

                    let mut values: Vec<Value> = Vec::with_capacity(len as usize);
                    for _ in 0..len {
                        if let Some(value) = Value::read_from(buffer).map_err(|err| {
                            mem::swap(&mut original, buffer);
                            err
                        })? {
                            values.push(value)
                        } else {
                            mem::swap(&mut original, buffer);
                            return Ok(None);
                        }
                    }

                    Ok(Some(Value::Array(values)))
                } else {
                    Ok(None)
                }
            }
            b'$' => {
                if let Some(len) = read_simple(buffer)?
                    .map(|command| -> Result<isize, DecodeError> {
                        let repr = std::str::from_utf8(&command[1..command.len() - 2])?;
                        trace!("Parsing RESPv2 bulk string size from '{}'", repr);
                        Ok(repr.parse()?)
                    })
                    .transpose()
                    .map_err(|err| {
                        mem::swap(&mut original, buffer);
                        err
                    })?
                {
                    if len == -1 {
                        Ok(Some(Value::Null))
                    } else if len < 0 {
                        mem::swap(&mut original, buffer);
                        Err(DecodeError::InvalidLength)
                    } else if (buffer.len() as isize) < len + 2 {
                        Ok(None)
                    } else if buffer[len as usize] != b'\r' {
                        Err(DecodeError::UnexpectedByte(buffer[len as usize]))
                    } else if buffer[len as usize + 1] != b'\n' {
                        Err(DecodeError::UnexpectedByte(buffer[(len as usize) + 1]))
                    } else {
                        Ok(Some(Value::BulkString(
                            buffer.split_to(len as usize + 2).split_to(len as usize),
                        )))
                    }
                } else {
                    Ok(None)
                }
            }
            b => Err(DecodeError::UnexpectedByte(b)),
        }
    }
}
impl ProtocolCodec for Value {
    fn decode_from(buffer: &mut BytesMut) -> Result<Option<Command>, DecodeError> {
        if let Some(value) = Self::read_from(buffer)? {
            debug!("Parsed raw value {:?}", value);
            Ok(None)
        } else {
            Ok(None)
        }
    }
    fn encode_to(response: Response, buffer: &mut BytesMut) -> Result<(), EncodeError> {
        unimplemented!()
    }
}

pub type StreamCodec<E> = super::StreamCodec<Value, E>;
