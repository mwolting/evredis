use std::mem;

use slog::{slog_debug, slog_trace};
use slog_scope::{debug, trace};

use bytes::{BufMut, Bytes, BytesMut};

use crate::protocol::{Command, Error, Response};

use super::{DecodeError, EncodeError, ProtocolCodec};

#[derive(Debug, Clone)]
pub enum Value {
    SimpleString(Bytes),
    Error(Bytes),
    Integer(i64),
    BulkString(Bytes),
    Array(Vec<Value>),
    Nil,
}
impl<'a> Value {
    fn read_from(buffer: &mut BytesMut) -> Result<Option<Self>, DecodeError> {
        if buffer.len() == 0 {
            return Ok(None);
        }

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
                Value::SimpleString(command.split_to(command.len() - 2).freeze())
            })),
            b'-' => Ok(read_simple(buffer)?.map(|mut command| {
                command.advance(1);
                Value::Error(command.split_to(command.len() - 2).freeze())
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
                        return Ok(Some(Value::Nil));
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
                        Ok(Some(Value::Nil))
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
                            buffer
                                .split_to(len as usize + 2)
                                .split_to(len as usize)
                                .freeze(),
                        )))
                    }
                } else {
                    Ok(None)
                }
            }
            b => Err(DecodeError::UnexpectedByte(b)),
        }
    }

    fn write_to(self, buffer: &mut BytesMut) -> Result<(), EncodeError> {
        match self {
            Value::Nil => {
                buffer.reserve(5);
                buffer.put("$-1\r\n");
            }
            Value::SimpleString(data) => {
                buffer.reserve(3 + data.len());
                buffer.put("+");
                buffer.put(data);
                buffer.put("\r\n");
            }
            Value::Error(data) => {
                buffer.reserve(3 + data.len());
                buffer.put("-");
                buffer.put(data);
                buffer.put("\r\n");
            }
            Value::Integer(value) => {
                let data = value.to_string();
                buffer.reserve(3 + data.len());
                buffer.put(":");
                buffer.put(data);
                buffer.put("\r\n");
            }
            Value::BulkString(data) => {
                let data_len = data.len().to_string();
                buffer.reserve(5 + data.len() + data_len.len());

                buffer.put("$");
                buffer.put(data_len);
                buffer.put("\r\n");
                buffer.put(data);
                buffer.put("\r\n");
            }
            Value::Array(elements) => {
                let elements_len = elements.len().to_string();
                buffer.reserve(3 + elements.len() + elements_len.len());
                buffer.put("*");
                buffer.put(elements_len);
                buffer.put("\r\n");
                for element in elements.into_iter() {
                    element.write_to(buffer)?;
                }
            }
        }

        Ok(())
    }
}
impl ProtocolCodec for Value {
    fn decode_from(buffer: &mut BytesMut) -> Result<Option<Command>, DecodeError> {
        if let Some(value) = Self::read_from(buffer)? {
            debug!("Parsed raw value {:?}", value);

            if let Value::Array(elems) = value {
                Ok(Some(match elems[0] {
                    Value::SimpleString(ref command) | Value::BulkString(ref command) => {
                        match command.as_ref() {
                            b"PING" => match &elems[1..] {
                                &[] => Command::Ping(None),
                                &[Value::BulkString(ref msg)] => Command::Ping(Some(msg.clone())),
                                _ => Err(DecodeError::UnexpectedNumberOfArguments)?,
                            },
                            b"GET" => match &elems[1..] {
                                &[Value::BulkString(ref key)] => Command::Get(key.clone()),
                                _ => Err(DecodeError::UnexpectedNumberOfArguments)?,
                            },
                            b"SET" => match &elems[1..] {
                                &[Value::BulkString(ref key), Value::BulkString(ref value)] => {
                                    Command::Set(key.clone(), value.clone())
                                }
                                _ => Err(DecodeError::UnexpectedNumberOfArguments)?,
                            },
                            _ => Err(DecodeError::UnrecognizedCommand)?,
                        }
                    }
                    _ => Err(DecodeError::InvalidDataType)?,
                }))
            } else {
                Err(DecodeError::InvalidDataType)
            }
        } else {
            Ok(None)
        }
    }
    fn encode_to(response: Response, buffer: &mut BytesMut) -> Result<(), EncodeError> {
        let value = match response {
            Response::Nil => Value::Nil,
            Response::Pong => Value::SimpleString(Bytes::from(&b"PONG"[..])),
            Response::Ok => Value::SimpleString(Bytes::from(&b"OK"[..])),
            Response::Bulk(data) => Value::BulkString(data),
            Response::Error(Error::WrongType) => Value::Error(Bytes::from(
                &b"WRONGTYPE Operation against a key holding the wrong kind of value"[..],
            )),
        };
        debug!("Encoded raw value {:?}", value);

        value.write_to(buffer)?;

        Ok(())
    }
}

pub type StreamCodec<E> = super::StreamCodec<Value, E>;
