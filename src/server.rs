//! The evredis server and its configuration

use std::io;
use std::net::{SocketAddr, ToSocketAddrs};

use slog::{slog_error, slog_info};
use slog_scope::{error, info};

use serde_derive::Deserialize;

use actix::prelude::*;
use actix_net::server::Server;
use actix_net::service::IntoNewService;
use futures::{Future, IntoFuture};

use crate::codecs::resp2;
use crate::storage::reader::Reader;
use crate::storage::writer::Writer;

pub mod connection;

/// Configuration for an evredis server
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfiguration {
    /// The interfaces to listen on
    pub listen_on: Vec<SocketAddr>,
}
impl Default for ServerConfiguration {
    fn default() -> Self {
        ServerConfiguration {
            listen_on: "localhost:6379"
                .to_socket_addrs()
                .expect("Invalid default address")
                .collect(),
        }
    }
}
impl ServerConfiguration {
    /// Spawn a server actor
    ///
    /// This may fail if the server cannot bind on the configured interfaces
    pub fn start_server(&self) -> io::Result<Addr<Server>> {
        start(&self.listen_on[..])
    }
}

/// Spawn a server actor on the given interfaces
pub fn start(addr: impl ToSocketAddrs) -> io::Result<Addr<Server>> {
    Ok(Server::default()
        .bind("evredis", addr, move || {
            info!("Spawning new worker");
            let codec = resp2::StreamCodec::default();

            (move |stream: tokio_tcp::TcpStream| {
                info!("Accepting new connection");
                stream.set_nodelay(true).unwrap();

                let reader = Reader::from_registry();
                let writer = Writer::from_registry();

                connection::accept(stream, codec.clone(), reader, writer)
                    .into_future()
                    .map_err(|err| error!("Connection error: {}", err))
            })
            .into_new_service()
        })?
        .start())
}
