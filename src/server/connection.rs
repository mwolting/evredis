use std::io;

use slog::{slog_info, slog_o, Logger};
use slog_scope::info;

use quick_error::quick_error;
use uuid::Uuid;

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
    id: Uuid,
    stream: T,
    logger: Logger,
}

impl<T> Connection<T>
where
    T: Stream<Item = Command, Error = ConnectionError>,
    T: Sink<SinkItem = Response, SinkError = ConnectionError>,
{
    pub fn new(stream: T) -> Self {
        let id = Uuid::new_v4();
        let logger = slog_scope::logger().new(slog_o!("connection" => format!("{}", id)));
        slog_info!(logger, "Opening connection");
        Connection { id, stream, logger }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ConnectionError> {
        let (tx, rx) = self.stream.split();
        let logger = self.logger;
        let id = self.id;
        rx.map(move |cmd| {
            slog_info!(logger, "Processing command {:?}", cmd);

            match cmd {
                Command::Ping(None) => Response::Pong,
                Command::Ping(Some(msg)) => Response::Bulk(msg),
            }
        })
        .forward(tx)
        .map(move |_| info!("Closing connection"; "connection" => format_args!("{}", id)))
    }
}

pub fn accept<S, D>(stream: S, codec: D) -> impl IntoFuture<Item = (), Error = ConnectionError>
where
    S: AsyncRead + AsyncWrite,
    D: Decoder<Item = Command, Error = ConnectionError>,
    D: Encoder<Item = Response, Error = ConnectionError>,
{
    let framed = codec.framed(stream);
    let conn = Connection::new(framed);

    conn.run()
}
