//! Underlying key/value storage

use std::collections::{BTreeSet, HashMap, HashSet};

use quick_error::quick_error;

use bytes::Bytes;
use evmap::shallow_copy::ShallowCopy;

use actix_derive::Message;

use crate::protocol::{Command, Response};

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
    command: Command,
}
impl From<Command> for Operation {
    fn from(command: Command) -> Self {
        Operation { command }
    }
}
