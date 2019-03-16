//! Database reader actor

use super::*;

use slog::slog_info;
use slog_scope::info;

use evmap::ReadHandle;

use actix::prelude::*;

use crate::protocol::Response;

/// An actor that wraps a database reader handle
pub struct Reader {
    store: Option<ReadHandle<Key, Value>>,
}

impl Reader {
    /// Construct a new reader for the given handle
    pub fn new(store: ReadHandle<Key, Value>) -> Self {
        Reader { store: Some(store) }
    }
}
impl Default for Reader {
    fn default() -> Self {
        Reader { store: None }
    }
}
impl Supervised for Reader {}
impl ArbiterService for Reader {}
impl Actor for Reader {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        info!("Spawned reader");

        writer::Writer::from_registry()
            .send(writer::Subscribe)
            .into_actor(self)
            .map(|writer::Subscription(store), actor, _ctx| {
                actor.store = Some(store);
            })
            .map_err(|_, _, _| ())
            .wait(ctx);
    }
}
impl OperationProcessor for Reader {
    fn reader(&self) -> Option<&ReadHandle<Key, Value>> {
        self.store.as_ref()
    }
    fn writer(&mut self) -> Option<&mut WriteHandle<Key, Value>> {
        None
    }
}
impl Handler<Operation> for Reader {
    type Result = Result<Response, StorageError>;

    fn handle(&mut self, operation: Operation, _ctx: &mut Context<Self>) -> Self::Result {
        debug_assert!(operation.command.reads());

        self.process_operation(operation)
    }
}
