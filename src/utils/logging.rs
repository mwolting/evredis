use serde_derive::Deserialize;
use slog::{o, Drain};

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Full,
    Compact,
    Json,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct LoggingConfiguration {
    format: Format,
    level: Option<String>,
    filter: Option<String>,
    debug_info: bool,
}

impl Default for LoggingConfiguration {
    fn default() -> Self {
        LoggingConfiguration {
            format: Format::Full,
            level: Some("warn".into()),
            filter: None,
            debug_info: false,
        }
    }
}

impl LoggingConfiguration {
    fn build_format(&self) -> impl slog::Drain<Ok = (), Err = slog::Never> {
        let formatter: Box<slog::Drain<Ok = (), Err = slog::Never> + Send> = match self.format {
            Format::Full => {
                let decorator = slog_term::TermDecorator::new().stderr().build();

                Box::new(slog_term::FullFormat::new(decorator).build().fuse())
            }
            Format::Compact => {
                let decorator = slog_term::TermDecorator::new().stderr().build();

                Box::new(slog_term::CompactFormat::new(decorator).build().fuse())
            }
            Format::Json => Box::new(
                slog_json::Json::new(std::io::stderr())
                    .add_default_keys()
                    .build()
                    .fuse(),
            ),
        };

        let mut filter = slog_envlogger::LogBuilder::new(formatter);
        if let Some(ref level) = self.level {
            filter = filter.filter(
                None,
                str::parse::<slog::FilterLevel>(level).unwrap_or(slog::FilterLevel::Warning),
            );
        };
        if let Some(ref filter_expr) = self.filter {
            filter = filter.parse(&filter_expr);
        }

        slog_async::Async::new(filter.build().fuse()).build().fuse()
    }

    pub fn create_logger(&self) -> slog::Logger {
        if self.debug_info {
            slog::Logger::root(
                self.build_format(),
                o!(
                 "module" => slog::FnValue(move |info| {
                     info.module()
                 }),
                "file" =>
                 slog::FnValue(move |info| {
                     format!("{}:{}",
                             info.file(),
                             info.line()
                             )
                 }),
                ),
            )
        } else {
            slog::Logger::root(self.build_format(), o!())
        }
    }

    pub fn create_global_logger(&self) -> slog_scope::GlobalLoggerGuard {
        let logger = self.create_logger();
        slog_scope::set_global_logger(logger)
    }
}
