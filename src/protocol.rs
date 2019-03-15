use bytes::BytesMut;

use actix_derive::Message;

#[derive(Debug, Message)]
pub enum Command {
    Ping(Option<BytesMut>),
}
impl Command {
    pub fn writes(&self) -> bool {
        use Command::*;
        match self {
            Ping(_) => true,
        }
    }

    pub fn reads(&self) -> bool {
        !self.writes()
    }
}

#[derive(Debug)]
pub enum Error {}

#[derive(Debug)]
pub enum Response {
    Error(Error),
    Nil,
    Pong,
    Bulk(BytesMut),
}
