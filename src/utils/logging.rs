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
    forward_stdlog: bool,
    stdlog_level: Option<String>,
    with_module: bool,
    with_filename: bool,
}

impl Default for LoggingConfiguration {
    fn default() -> Self {
        LoggingConfiguration {
            format: Format::Full,
            level: Some("warn".into()),
            filter: None,
            forward_stdlog: true,
            stdlog_level: Some("info".into()),
            with_module: true,
            with_filename: false,
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
                level
                    .parse::<slog::FilterLevel>()
                    .unwrap_or(slog::FilterLevel::Warning),
            );
        };
        if let Some(ref filter_expr) = self.filter {
            filter = filter.parse(&filter_expr);
        }

        slog_async::Async::new(filter.build().fuse()).build().fuse()
    }

    pub fn create_logger(&self) -> slog::Logger {
        let module = slog::FnValue(move |info| info.module());
        let filename = slog::FnValue(move |info| format!("{}:{}", info.file(), info.line()));

        match (self.with_filename, self.with_module) {
            (false, false) => slog::Logger::root(self.build_format(), o!()),
            (false, true) => slog::Logger::root(self.build_format(), o!("module" => module)),
            (true, false) => slog::Logger::root(self.build_format(), o!("file" => filename)),
            (true, true) => slog::Logger::root(
                self.build_format(),
                o!("module" => module, "file" => filename),
            ),
        }
    }

    pub fn create_global_logger(
        &self,
    ) -> Result<slog_scope::GlobalLoggerGuard, log::SetLoggerError> {
        let logger = self.create_logger();
        let guard = slog_scope::set_global_logger(logger);
        if self.forward_stdlog {
            if let Some(ref level) = self.stdlog_level {
                slog_stdlog::init_with_level(
                    level
                        .parse::<log::LogLevel>()
                        .unwrap_or(log::LogLevel::Info),
                )?;
            } else {
                slog_stdlog::init()?;
            }
        }

        Ok(guard)
    }
}
