use super::*;

use slog::slog_info;
use slog_scope::info;

use evmap::{ReadHandle, WriteHandle};

use actix_derive::{Message, MessageResponse};

use actix::prelude::*;

use crate::protocol::{self, Command, Response};

pub struct Writer {
    reader: ReadHandle<Key, Value>,
    writer: WriteHandle<Key, Value>,
}

impl Writer {
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

    fn handle(&mut self, _: Subscribe, ctx: &mut Context<Self>) -> Self::Result {
        Subscription(self.reader.clone())
    }
}
impl Handler<Operation> for Writer {
    type Result = Result<Response, StorageError>;

    fn handle(&mut self, operation: Operation, ctx: &mut Context<Self>) -> Self::Result {
        Ok(match operation.command {
            Command::Ping(None) => Response::Pong,
            Command::Ping(Some(msg)) => Response::Bulk(msg),
        })
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Subscription")]
pub struct Subscribe;

#[derive(MessageResponse)]
pub struct Subscription(pub ReadHandle<Key, Value>);
