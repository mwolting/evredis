use std::io;

use quick_error::quick_error;

use futures::{Future, IntoFuture, Sink, Stream};
use tokio_codec::{Decoder, Encoder};
use tokio_io::{AsyncRead, AsyncWrite};

use crate::codecs::{DecodeError, EncodeError};
use crate::protocol::{Command, Response};

quick_error! {
    #[derive(Debug)]
    pub enum ConnectionError {
        Io(err: io::Error) {
            display("IO error: {}", err)
            from()
        }
        ResponseEncoding(err: EncodeError) {
            display("Failed to encode response: {}", err)
            from()
        }
        CommandDecoding(err: DecodeError) {
            display("Failed to decode command: {}", err)
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

    pub fn run(self) -> impl IntoFuture<Item = (), Error = ConnectionError> {
        let (tx, rx) = self.stream.split();
        tx.send_all(rx.map(|cmd| Response::String("Tzt".into())))
            .map(|_| ())
    }
}

pub fn accept<S, D>(stream: S, codec: D) -> impl IntoFuture<Item = (), Error = ConnectionError>
where
    S: AsyncRead + AsyncWrite,
    D: Decoder<Item = Command, Error = ConnectionError>,
    D: Encoder<Item = Response, Error = ConnectionError>,
{
    let conn = Connection::new(codec.framed(stream));

    conn.run()
}
