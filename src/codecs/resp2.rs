use std::io;
use std::marker::PhantomData;

use bytes::BytesMut;
use tokio_codec::{Encoder, Decoder};

use crate::protocol::{Command, Response};

use super::{EncodeError, DecodeError};


#[derive(Debug)]
pub struct Codec<E>
    where E: From<EncodeError>,
            E: From<DecodeError>,
            E: From<io::Error>,
{
    __err: PhantomData<E>
}
impl<E> Clone for Codec<E>
    where E: From<EncodeError>,
            E: From<DecodeError>,
            E: From<io::Error>,
{
    fn clone(&self) -> Self {
        Codec {
            __err: self.__err
        }
    }
}
impl<E> Default for Codec<E>
    where E: From<EncodeError>,
            E: From<DecodeError>,
            E: From<io::Error>,
{
    fn default() -> Self {
        Codec {
            __err: PhantomData
        }
    }
}
impl<E> Encoder for Codec<E> 
    where E: From<EncodeError>,
            E: From<DecodeError>,
            E: From<io::Error>,
{
    type Item = Response;
    type Error = E;

    fn encode(&mut self, response: Response, buffer: &mut BytesMut) -> Result<(), E> {
        Ok(())
    }
}
impl<E> Decoder for Codec<E> 
    where E: From<EncodeError>,
            E: From<DecodeError>,
            E: From<io::Error>,
{
    type Item = Command;
    type Error = E;

    fn decode(&mut self, buffer: &mut BytesMut) -> Result<Option<Command>, E> {
        Ok(None)
    }
}