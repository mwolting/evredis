use std::io;

use slog::{slog_debug, slog_info, slog_o, Logger};
use slog_scope::info;

use quick_error::quick_error;
use uuid::Uuid;

use actix::Addr;
use futures::{Future, IntoFuture, Sink, Stream};
use tokio_codec::{Decoder, Encoder};
use tokio_io::{AsyncRead, AsyncWrite};

use crate::codecs::{DecodeError, EncodeError};
use crate::protocol::{Command, Response};
use crate::storage::reader::Reader;
use crate::storage::writer::Writer;
use crate::storage::{Operation, StorageError};

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
        Storage(err: StorageError) {
            display("Failed to execute storage operation: {}", err)
            from()
        }
        Mailbox(err: actix::MailboxError) {
            display("Mailbox error: {}", err)
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
    reader: Addr<Reader>,
    writer: Addr<Writer>,
}

impl<T> Connection<T>
where
    T: Stream<Item = Command, Error = ConnectionError>,
    T: Sink<SinkItem = Response, SinkError = ConnectionError>,
{
    pub fn new(stream: T, reader: Addr<Reader>, writer: Addr<Writer>) -> Self {
        let id = Uuid::new_v4();
        let logger = slog_scope::logger().new(slog_o!("connection" => format!("{}", id)));
        slog_info!(logger, "Opening connection");
        Connection {
            id,
            stream,
            logger,
            reader,
            writer,
        }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ConnectionError> {
        let Connection {
            stream,
            logger,
            id,
            reader,
            writer,
        } = self;
        let (tx, rx) = stream.split();

        rx.and_then(move |cmd| {
            slog_debug!(logger, "Processing command {:?}", cmd);

            let response: Box<Future<Item = Result<Response, StorageError>, Error = _>> =
                if cmd.writes() {
                    Box::new(writer.send(Operation::from(cmd)))
                } else {
                    Box::new(reader.send(Operation::from(cmd)))
                };

            response.map_err(ConnectionError::from).and_then(|x| Ok(x?))
        })
        .forward(tx)
        .map(move |_| info!("Closing connection"; "connection" => format_args!("{}", id)))
    }
}

pub fn accept<S, D>(
    stream: S,
    codec: D,
    reader: Addr<Reader>,
    writer: Addr<Writer>,
) -> impl IntoFuture<Item = (), Error = ConnectionError>
where
    S: AsyncRead + AsyncWrite,
    D: Decoder<Item = Command, Error = ConnectionError>,
    D: Encoder<Item = Response, Error = ConnectionError>,
{
    let framed = codec.framed(stream);
    let conn = Connection::new(framed, reader, writer);

    conn.run()
}
