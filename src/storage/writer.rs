//! Database writer actor
//!
use super::*;

use slog::{slog_debug, slog_info};
use slog_scope::{debug, info};

use evmap::{ReadHandle, WriteHandle};

use actix_derive::{Message, MessageResponse};

use actix::prelude::*;

use crate::protocol::Response;

/// An actor that wraps a database reader handle
pub struct Writer {
    reader: ReadHandle<Key, Item>,
    writer: WriteHandle<Key, Item>,
    operation_id: u64,
}

impl Writer {
    /// Construct a new writer for the given handle
    pub fn new(store: WriteHandle<Key, Item>) -> Self {
        Writer {
            reader: store.clone(),
            writer: store,
            operation_id: 0,
        }
    }
}
impl Default for Writer {
    fn default() -> Self {
        let (reader, writer) = evmap::new();
        Writer {
            reader,
            writer,
            operation_id: 0,
        }
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

impl Handler<Operation> for Writer {
    type Result = Result<Response, StorageError>;

    fn handle(&mut self, operation: Operation, ctx: &mut Context<Self>) -> Self::Result {
        use super::ops::*;
        debug_assert!(operation.command.writes());

        self.operation_id += 1;
        let operation_id = self.operation_id;

        let response = Ok(match operation.command {
            Command::Set(key, value, expiration, conditional) => conditional
                .when(self.writer.contains_key(&key), || {
                    let expires_at = expiration.map(|x| clock::now() + x);

                    self.writer.update(
                        key.clone(),
                        Item {
                            value: Value::String(value),
                            meta: Metadata {
                                expiration: expires_at,
                                operation_id,
                            },
                        },
                    );

                    if let Some(t) = expiration {
                        ctx.run_later(t, move |act, _ctx| {
                            debug!("Expiring key {:?}", key);
                            if act
                                .writer
                                .get_and(&key, get_metadata)
                                .map(|meta| meta.operation_id == operation_id)
                                .unwrap_or(false)
                            {
                                act.writer.empty(key);
                                act.writer.refresh();
                            }
                        });
                    }

                    Response::Ok
                })
                .unwrap_or(Response::Nil),
            Command::Del(keys) => {
                let mut updated = 0;
                for key in keys {
                    if self.writer.contains_key(&key) {
                        updated += 1;
                        self.writer.empty(key);
                    }
                }
                Response::Integer(updated)
            }
            Command::FlushAll(_) | Command::FlushDB(_) => {
                info!("Flushing the database");
                self.writer.purge();
                Response::Ok
            }
            _ => Err(StorageError::NoReadAccess)?,
        });

        self.writer.refresh();
        response
    }
}

/// A subscription request to get a reader handle for a `Writer`'s dataset
#[derive(Debug, Message)]
#[rtype(result = "Subscription")]
pub struct Subscribe;

/// A reader handle for a `Writer`'s dataset
#[derive(MessageResponse)]
pub struct Subscription(pub ReadHandle<Key, Item>);
