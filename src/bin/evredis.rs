use serde_derive::Deserialize;

use slog::slog_info;
use slog_scope::info;

use actix::System;

use evredis::server::ServerConfiguration;
use evredis::utils::configuration::Configuration;
use evredis::utils::logging::LoggingConfiguration;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct RootConfiguration {
    logging: LoggingConfiguration,
    server: ServerConfiguration,
}
impl Configuration for RootConfiguration {
    const VERSION_REQUIREMENT: &'static str = "^0.1";
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<std::error::Error>> {
    let config = RootConfiguration::load()?;
    let handle = config.logging.create_global_logger();

    let system = System::new("evredis");

    ctrlc::set_handler(|| {
        System::current().stop();
    })
    .expect("Failed to set ctrl+c handler");

    info!("evredis v{}", VERSION);

    config.server.start_server()?;

    let code = system.run();

    info!("Shutting down...");
    drop(handle);
    std::process::exit(code);
}
