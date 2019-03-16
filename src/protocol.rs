//! Types related to the Redis command/response protocol

use bytes::Bytes;

use actix_derive::Message;


#[derive(Debug)]
pub enum Synchronicity {
    Sync,
    Async
}


/// A Redis command
#[derive(Debug, Message)]
pub enum Command {
    /// Ping the database
    Ping(Option<Bytes>),
    /// Get a key's value
    Get(Bytes),
    /// Set a key's value
    Set(Bytes, Bytes),
    /// Delete a key
    Del(Vec<Bytes>),
    /// Check if a key exists
    Exists(Vec<Bytes>),

    /// Flush all databases
    FlushAll(Synchronicity),
    /// Flush current database
    FlushDB(Synchronicity),
}
impl Command {
    /// Whether this command should be executed asynchronously
    pub fn is_async(&self) -> bool {
        use Command::*;
        match self {
            FlushAll(Synchronicity::Async) | FlushDB(Synchronicity::Async) => true,
            _ => false
        }
    }

    /// Whether this command is a write operation
    pub fn writes(&self) -> bool {
        use Command::*;
        match self {
            Ping(_) | Get(_) | Exists(_) => false,
            _ => true,
        }
    }

    /// Whether this command is a read operation
    pub fn reads(&self) -> bool {
        !self.writes()
    }
}

/// An error response
#[derive(Debug)]
pub enum Error {
    WrongType,
}

/// A response
#[derive(Debug)]
pub enum Response {
    Ok,
    Error(Error),
    Nil,
    Pong,
    Integer(i64),
    Bulk(Bytes),
}
