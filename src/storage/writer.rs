//! Database writer actor
//!
use super::*;

use slog::slog_info;
use slog_scope::info;

use evmap::{ReadHandle, WriteHandle};

use actix_derive::{Message, MessageResponse};

use actix::prelude::*;

use crate::protocol::Response;

/// An actor that wraps a database reader handle
pub struct Writer {
    reader: ReadHandle<Key, Value>,
    writer: WriteHandle<Key, Value>,
}

impl Writer {
    /// Construct a new writer for the given handle
    pub fn new(store: WriteHandle<Key, Value>) -> Self {
        Writer {
            reader: store.clone(),
            writer: store,
        }
    }
}
impl Default for Writer {
    fn default() -> Self {
        let (reader, writer) = evmap::new();
        Writer { reader, writer }
    }
}
impl Actor for Writer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("Spawned writer");
    }
}
impl Supervised for Writer {}
impl SystemService for Writer {}
impl Handler<Subscribe> for Writer {
    type Result = Subscription;

    fn handle(&mut self, _: Subscribe, _ctx: &mut Context<Self>) -> Self::Result {
        Subscription(self.reader.clone())
    }
}
impl OperationProcessor for Writer {
    fn reader(&self) -> Option<&ReadHandle<Key, Value>> {
        Some(&self.reader)
    }
    fn writer(&mut self) -> Option<&mut WriteHandle<Key, Value>> {
        Some(&mut self.writer)
    }
}
impl Handler<Operation> for Writer {
    type Result = Result<Response, StorageError>;

    fn handle(&mut self, operation: Operation, _ctx: &mut Context<Self>) -> Self::Result {
        debug_assert!(operation.command.writes());

        self.process_operation(operation)
    }
}

/// A subscription request to get a reader handle for a `Writer`'s dataset
#[derive(Debug, Message)]
#[rtype(result = "Subscription")]
pub struct Subscribe;

/// A reader handle for a `Writer`'s dataset
#[derive(MessageResponse)]
pub struct Subscription(pub ReadHandle<Key, Value>);
