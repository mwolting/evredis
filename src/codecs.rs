//! Codecs for Redis commands/responses

use std::io;
use std::marker::PhantomData;
use std::num::ParseIntError;
use std::str::Utf8Error;

use slog::slog_debug;
use slog_scope::debug;

use quick_error::quick_error;

use bytes::BytesMut;
use tokio_codec::{Decoder, Encoder};

use crate::protocol::{Command, Response};

pub mod resp2;

quick_error! {
    /// An error encountered during value encoding
    #[derive(Debug)]
    pub enum EncodeError {
        Dummy {}
    }
}

quick_error! {
    /// An error encountered during value decoding
    #[derive(Debug)]
    pub enum DecodeError {
        /// Unexpected byte at the current position
        UnexpectedByte(byte: u8) {
            display("Unexpected byte: {}", byte)
        }
        /// Unrecognized Redis command
        UnrecognizedCommand {}
        /// Unexpected number of arguments to a command
        UnexpectedNumberOfArguments {}
        /// Invalid length value for bulk string/array
        InvalidLength {}
        /// Invalid datatype for command
        InvalidDataType {}
        /// Invalid string value
        InvalidString(err: Utf8Error) {
            display("Invalid string: {}", err)
            from()
        }
        /// Invalid integer value
        InvalidInteger(err: ParseIntError) {
            display("Invalid integer: {}", err)
            from()
        }
    }
}

/// A codec that translates between high-level Redis commands/responses and a low-level wire format
pub trait ProtocolCodec {
    fn decode_from(buffer: &mut BytesMut) -> Result<Option<Command>, DecodeError>;
    fn encode_to(response: Response, buffer: &mut BytesMut) -> Result<(), EncodeError>;
}

/// A stream codec for framing bidirectional byte streams as command/response streams
#[derive(Debug)]
pub struct StreamCodec<P, E>
where
    P: ProtocolCodec,
    E: From<EncodeError>,
    E: From<DecodeError>,
    E: From<io::Error>,
{
    __protocol: PhantomData<P>,
    __err: PhantomData<E>,
}
impl<P, E> Clone for StreamCodec<P, E>
where
    P: ProtocolCodec,
    E: From<EncodeError>,
    E: From<DecodeError>,
    E: From<io::Error>,
{
    fn clone(&self) -> Self {
        StreamCodec {
            __protocol: self.__protocol,
            __err: self.__err,
        }
    }
}
impl<P, E> Default for StreamCodec<P, E>
where
    P: ProtocolCodec,
    E: From<EncodeError>,
    E: From<DecodeError>,
    E: From<io::Error>,
{
    fn default() -> Self {
        StreamCodec {
            __protocol: PhantomData,
            __err: PhantomData,
        }
    }
}

impl<P, E> Encoder for StreamCodec<P, E>
where
    P: ProtocolCodec,
    E: From<EncodeError>,
    E: From<DecodeError>,
    E: From<io::Error>,
{
    type Item = Response;
    type Error = E;

    fn encode(&mut self, response: Response, buffer: &mut BytesMut) -> Result<(), E> {
        P::encode_to(response, buffer)?;
        Ok(())
    }
}
impl<P, E> Decoder for StreamCodec<P, E>
where
    P: ProtocolCodec,
    E: From<EncodeError>,
    E: From<DecodeError>,
    E: From<io::Error>,
{
    type Item = Command;
    type Error = E;

    fn decode(&mut self, buffer: &mut BytesMut) -> Result<Option<Command>, E> {
        if buffer.is_empty() {
            return Ok(None);
        }

        let value = P::decode_from(buffer)?;
        debug!("Decoded value {:?}", value);

        Ok(value)
    }
}
