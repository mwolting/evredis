//! Database reader actor

use super::*;

use slog::slog_info;
use slog_scope::info;

use evmap::ReadHandle;

use actix::prelude::*;

use crate::protocol::Response;

/// An actor that wraps a database reader handle
pub struct Reader {
    store: Option<ReadHandle<Key, Item>>,
}

impl Reader {
    /// Construct a new reader for the given handle
    pub fn new(store: ReadHandle<Key, Item>) -> Self {
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

impl Handler<Operation> for Reader {
    type Result = Result<Response, StorageError>;

    fn handle(&mut self, operation: Operation, _ctx: &mut Context<Self>) -> Self::Result {
        use super::ops::*;

        debug_assert!(operation.command.reads());

        let reader = self.store.as_ref().ok_or(StorageError::NoReadAccess)?;

        Ok(match operation.command {
            Command::Ping(None) => Response::Pong,
            Command::Ping(Some(msg)) => Response::Bulk(msg),
            Command::Get(key) => reader
                .get_and(&key, get_string_as_bulk)
                .unwrap_or(Response::Nil),
            Command::Exists(keys) => {
                Response::Integer(keys.into_iter().filter(|k| reader.contains_key(k)).count() as i64)
            }
            _ => Err(StorageError::NoWriteAccess)?,
        })
    }
}
