//! The connection handler

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
    /// An error encountered during connection handling
    #[derive(Debug)]
    pub enum ConnectionError {
        /// An IO error
        Io(err: io::Error) {
            display("IO error: {}", err)
            from()
        }
        /// A response encoding error
        ResponseEncoding(err: EncodeError) {
            display("Failed to encode response: {}", err)
            from()
        }
        /// A command decoding error
        CommandDecoding(err: DecodeError) {
            display("Failed to decode command: {}", err)
            from()
        }
        /// A storage operation error
        Storage(err: StorageError) {
            display("Failed to execute storage operation: {}", err)
            from()
        }
        /// An actor mailbox capacity error
        Mailbox(err: actix::MailboxError) {
            display("Mailbox error: {}", err)
            from()
        }
    }
}

/// A connection handler
pub struct Connection<R, T>
where
    R: Stream<Item = Command, Error = ConnectionError>,
    T: Sink<SinkItem = Response, SinkError = ConnectionError>,
{
    /// The connection identifier (useful for log correlation)
    id: Uuid,
    /// The command stream to listen on
    rx: R,
    /// The response sink to respond on
    tx: T,
    /// A scoped logger
    logger: Logger,
    /// Address of the `Reader` actor to use
    reader: Addr<Reader>,
    /// Address of the `writer` actor to use
    writer: Addr<Writer>,
}

impl<R, T> Connection<R, T>
where
    R: Stream<Item = Command, Error = ConnectionError>,
    T: Sink<SinkItem = Response, SinkError = ConnectionError>,
{
    /// Create a new connection handler for the given input/output and reader/writer
    pub fn new(rx: R, tx: T, reader: Addr<Reader>, writer: Addr<Writer>) -> Self {
        let id = Uuid::new_v4();
        let logger = slog_scope::logger().new(slog_o!("connection" => format!("{}", id)));
        slog_info!(logger, "Opening connection");
        Connection {
            id,
            rx,
            tx,
            logger,
            reader,
            writer,
        }
    }

    /// Process the input stream's commands to completion
    pub fn run(self) -> impl Future<Item = (), Error = ConnectionError> {
        let Connection {
            rx,
            tx,
            logger,
            id,
            reader,
            writer,
        } = self;

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

/// Create and run a connection handler for the given bi-directional byte stream, codec, and reader/writer
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
    let (tx, rx) = codec.framed(stream).split();
    let conn = Connection::new(rx, tx, reader, writer);

    conn.run()
}
