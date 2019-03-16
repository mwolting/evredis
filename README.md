# evredis

[![Build Status](https://cloud.drone.io/api/badges/mwolting/evredis/status.svg)](https://cloud.drone.io/mwolting/evredis)

A Redis-compatible in-memory database, written in Rust and built on [actix](https://github.com/actix/actix) and [evmap](https://github.com/jonhoo/rust-evmap).

## Current state

Commands implemented:

- GET
- SET
- DEL
- EXISTS
- PING
- FLUSHDB
- FLUSHALL

Currently, evredis only supports a single database. Multi-database operations will therefore operate only on this one, and any database switching commands
will fail.

## License

evredis is available under the GNU Affero GPLv3 license.
