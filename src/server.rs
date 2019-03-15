use std::io;
use std::net::{SocketAddr, ToSocketAddrs};

use slog::{slog_error, slog_info};
use slog_scope::{error, info};

use serde_derive::Deserialize;

use actix::Addr;
use actix_net::server::Server;
use actix_net::service::IntoNewService;
use futures::{Future, IntoFuture};

use crate::codecs::resp2;

pub mod connection;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfiguration {
    listen_on: Vec<SocketAddr>,
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
    pub fn start_server(&self) -> io::Result<Addr<Server>> {
        start(&self.listen_on[..])
    }
}

pub fn start(addr: impl ToSocketAddrs) -> io::Result<Addr<Server>> {
    Ok(Server::default()
        .bind("evredis", addr, move || {
            let codec = resp2::StreamCodec::default();

            info!("Spawning new worker");
            (move |stream: tokio_tcp::TcpStream| {
                info!("Accepting new connection");
                stream.set_nodelay(true).unwrap();
                connection::accept(stream, codec.clone())
                    .into_future()
                    .map_err(|err| error!("Connection error: {}", err))
            })
            .into_new_service()
        })?
        .start())
}
