use std::io;

use quick_error::quick_error;

use futures::{IntoFuture, Sink, Stream};
use tokio_codec::{Decoder, Encoder};
use tokio_io::{AsyncRead, AsyncWrite};

use crate::codecs::{EncodeError, DecodeError};
use crate::protocol::{Command, Response};

quick_error! {
    #[derive(Debug)]
    pub enum ConnectionError {
        Io(err: io::Error) {
            from()
        }
        ResponseEncoding(err: EncodeError) {
            from()
        }
        CommandDecoding(err: DecodeError) {
            from()
        }
    }
}

pub struct Connection<T>
where
    T: Stream<Item = Command, Error = ConnectionError>,
    T: Sink<SinkItem = Response, SinkError = ConnectionError>,
{
    stream: T,
}

impl<T> Connection<T>
where
    T: Stream<Item = Command, Error = ConnectionError>,
    T: Sink<SinkItem = Response, SinkError = ConnectionError>,
{
    pub fn new(stream: T) -> Self {
        Connection { stream }
    }
}

pub fn accept<S, D>(stream: S, codec: D) -> impl IntoFuture<Item = (), Error = ConnectionError>
where
    S: AsyncRead + AsyncWrite,
    D: Decoder<Item = Command, Error = ConnectionError>,
    D: Encoder<Item = Response, Error = ConnectionError>,
{
    let conn = Connection::new(codec.framed(stream));

    Ok(())
}
