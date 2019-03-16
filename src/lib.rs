//! # evredis
//!
//! A Redis-compatible in-memory database, written in Rust and built on [actix](https://docs.rs/actix) and [evmap](https://docs.rs/evmap).

/// Various utilities
pub mod utils {
    pub mod configuration;
    pub mod logging;
}

pub mod codecs;
pub mod protocol;
pub mod server;
pub mod storage;
