use bytes::BytesMut;

#[derive(Debug)]
pub enum Command {
    Ping(Option<BytesMut>),
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
