use bytes::Bytes;

use actix_derive::Message;

#[derive(Debug, Message)]
pub enum Command {
    Ping(Option<Bytes>),
    Get(Bytes),
    Set(Bytes, Bytes),
}
impl Command {
    pub fn writes(&self) -> bool {
        use Command::*;
        match self {
            Ping(_) | Get(_) => false,
            _ => true,
        }
    }

    pub fn reads(&self) -> bool {
        !self.writes()
    }
}

#[derive(Debug)]
pub enum Error {
    WrongType,
}

#[derive(Debug)]
pub enum Response {
    Ok,
    Error(Error),
    Nil,
    Pong,
    Bulk(Bytes),
}
