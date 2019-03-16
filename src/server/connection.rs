//! The connection handler

use std::io;

use slog::{slog_debug, slog_error, slog_info, slog_o, Logger};
use slog_scope::error;

use quick_error::quick_error;
use uuid::Uuid;

use actix::prelude::*;
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
        /// An actor send error
        Send(err: SendError<Operation>) {
            display("Send error: {}", err)
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
    _client_id: Uuid,
    /// The command stream to listen on
    rx: Option<R>,
    /// The response sink to respond on
    tx: Option<T>,
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
        let client_id = Uuid::new_v4();
        let logger = slog_scope::logger().new(slog_o!("client_id" => format!("{}", client_id)));
        Connection {
            _client_id: client_id,
            rx: Some(rx),
            tx: Some(tx),
            logger,
            reader,
            writer,
        }
    }
}

impl<R, T> StreamHandler<Operation, ConnectionError> for Connection<R, T>
where
    R: Stream<Item = Command, Error = ConnectionError> + 'static,
    T: Sink<SinkItem = Response, SinkError = ConnectionError> + 'static,
{
    fn error(&mut self, err: ConnectionError, _ctx: &mut Self::Context) -> Running {
        slog_error!(self.logger, "Connection error: {}", err);

        Running::Stop
    }

    fn handle(&mut self, operation: Operation, ctx: &mut Self::Context) {
        let cmd = operation.command;
        slog_debug!(self.logger, "Processing command {:?}", cmd);

        let response: Box<Future<Item = Response, Error = ConnectionError>> = match cmd {
            _ if cmd.is_async() && cmd.writes() => Box::new(self.writer.try_send(Operation::from(cmd)).map(|()| Response::Ok).map_err(ConnectionError::from).into_future()),
            _ if cmd.is_async() => Box::new(self.reader.try_send(Operation::from(cmd)).map(|()| Response::Ok).map_err(ConnectionError::from).into_future()),
            _ if cmd.writes() => Box::new(self.writer.send(Operation::from(cmd)).then(|x| Ok(x??))),
            _ => Box::new(self.reader.send(Operation::from(cmd)).then(|x| Ok(x??))),
        };

        let tx = self.tx.take().expect("Sink not available");
        ctx.wait(
            response
                .and_then(|msg| tx.send(msg))
                .into_actor(self)
                .map(|sink, actor, _ctx| {
                    actor.tx = Some(sink);
                })
                .map_err(|err, _, _| error!("Error while executing command: {}", err)),
        );
    }
}

impl<R, T> Actor for Connection<R, T>
where
    R: Stream<Item = Command, Error = ConnectionError> + 'static,
    T: Sink<SinkItem = Response, SinkError = ConnectionError> + 'static,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        slog_info!(self.logger, "Opening connection");
        Self::add_stream(
            self.rx
                .take()
                .expect("Stream already consumed")
                .map(Operation::from),
            ctx,
        );
    }
}

/// Create and run a connection handler for the given bi-directional byte stream, codec, and reader/writer
pub fn accept<S: 'static, D: 'static>(
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

    conn.start();

    Ok(())
}
