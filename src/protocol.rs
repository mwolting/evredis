//! Types related to the Redis command/response protocol

use bytes::Bytes;

use actix_derive::Message;

/// A Redis command
#[derive(Debug, Message)]
pub enum Command {
    /// Ping the database
    Ping(Option<Bytes>),
    /// Get a value
    Get(Bytes),
    /// Set a value
    Set(Bytes, Bytes),
}
impl Command {
    /// Whether this command is a write operation
    pub fn writes(&self) -> bool {
        use Command::*;
        match self {
            Ping(_) | Get(_) => false,
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
    Bulk(Bytes),
}
