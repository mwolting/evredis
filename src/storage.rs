//! Underlying key/value storage

use std::collections::{BTreeSet, HashMap, HashSet};

use slog::slog_info;
use slog_scope::info;

use quick_error::quick_error;

use bytes::Bytes;
use evmap::shallow_copy::ShallowCopy;
use evmap::{ReadHandle, WriteHandle};

use actix_derive::Message;

use crate::protocol::{Command, Error, Response};

pub mod reader;
pub mod writer;

quick_error! {
    /// An error encountered during storage operations
    #[derive(Debug)]
    pub enum StorageError {
        /// The storage actor doesn't have write permissions
        NoWriteAccess {
            display("No write access")
        }
        /// The storage actor doesn't have read permissions
        NoReadAccess {
            display("No read access")
        }
    }
}

/// A storage key
pub type Key = Bytes;

/// A storage value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(Bytes),
    List(Vec<Bytes>),
    Set(Box<HashSet<Bytes>>),
    OrderedSet(Box<BTreeSet<Bytes>>),
    Hash(Box<HashMap<Bytes, Bytes>>),
}
impl ShallowCopy for Value {
    unsafe fn shallow_copy(&mut self) -> Self {
        match self {
            Value::String(data) => {
                let inner = std::slice::from_raw_parts(data.as_ptr(), data.len());
                Value::String(Bytes::from_static(inner))
            }
            Value::List(ref mut values) => Value::List(values.shallow_copy()),
            Value::Set(ref mut values) => Value::Set(values.shallow_copy()),
            Value::OrderedSet(ref mut values) => Value::OrderedSet(values.shallow_copy()),
            Value::Hash(ref mut values) => Value::Hash(values.shallow_copy()),
        }
    }
}

/// A storage operation that can be processed by storage actors
#[derive(Debug, Message)]
#[rtype(result = "Result<Response, StorageError>")]
pub struct Operation {
    pub command: Command,
}
impl From<Command> for Operation {
    fn from(command: Command) -> Self {
        Operation { command }
    }
}

mod ops {
    use super::*;
    pub fn get_string_as_bulk(values: &[Value]) -> Response {
        match values[0] {
            Value::String(ref data) => Response::Bulk(data.clone()),
            _ => Response::Error(Error::WrongType),
        }
    }
}

trait OperationProcessor {
    fn reader(&self) -> Option<&ReadHandle<Key, Value>>;
    fn writer(&mut self) -> Option<&mut WriteHandle<Key, Value>>;

    fn process_operation(&mut self, operation: Operation) -> Result<Response, StorageError> {
        use self::ops::*;

        let command = operation.command;

        if command.writes() {
            let writer = self.writer().ok_or(StorageError::NoWriteAccess)?;

            let response = Ok(match command {
                Command::Set(key, value) => {
                    writer.update(key, Value::String(value));
                    Response::Ok
                }
                Command::Del(keys) => {
                    let mut updated = 0;
                    for key in keys {
                        if writer.contains_key(&key) {
                            updated += 1;
                            writer.empty(key);
                        }
                    }
                    Response::Integer(updated)
                }
                Command::FlushAll(_) | Command::FlushDB(_) => {
                    info!("Flushing the database");
                    writer.purge();
                    Response::Ok
                }
                _ => unreachable!(),
            });

            writer.refresh();
            response
        } else {
            let reader = self.reader().ok_or(StorageError::NoReadAccess)?;

            Ok(match command {
                Command::Ping(None) => Response::Pong,
                Command::Ping(Some(msg)) => Response::Bulk(msg),
                Command::Get(key) => reader
                    .get_and(&key, get_string_as_bulk)
                    .unwrap_or(Response::Nil),
                Command::Exists(keys) => Response::Integer(
                    keys.into_iter().filter(|k| reader.contains_key(k)).count() as i64,
                ),
                _ => unreachable!(),
            })
        }
    }
}
