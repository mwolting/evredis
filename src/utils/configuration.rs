//! Utilities related to configuration loading

use quick_error::quick_error;

use app_dirs::*;
use config::{Config, ConfigError, Environment, File};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};

use semver::{Version, VersionReq};

quick_error! {
    /// An error encountered during configuration loading
    #[derive(Debug)]
    pub enum ConfigurationError {
        /// Failed to load configuration
        LoadFailure(err: ConfigError) {
            display("Load failure: {}", err)
            from()
        }
        /// Failed to find configuration location
        MissingLocation(err: app_dirs::AppDirsError) {
            display("Missing location: {}", err)
            from()
        }
        /// Version mismatch between configuration and application
        IncompatibleVersion(actual: Version, expected: VersionReq) {
            display("Configuration of version {} is incompatible with requirement {}", actual, expected)
        }
    }
}

/// A configuration that can be loaded from multiple layers (files and environment)
pub trait Configuration: DeserializeOwned {
    /// A semver version requirement on the loaded configuration
    const VERSION_REQUIREMENT: &'static str = "*";

    /// Load a configuration from the environment only
    fn load_env() -> Result<Self, ConfigurationError> {
        let _ = dotenv::dotenv();

        let mut s = Config::new();
        s.merge(
            Environment::with_prefix("evredis")
                .separator("_")
                .ignore_empty(true),
        )?;

        Ok(s.try_into()?)
    }

    /// Load a configuration from the environment and several files
    ///
    /// Default locations include the system-wide and user-specific configuration dirs
    /// (different per OS), and (if the EVREDIS_DEBUG environment variable is set) the `config`
    /// directory in the current working dir.
    fn load() -> Result<Self, ConfigurationError> {
        let version_req =
            VersionReq::parse(Self::VERSION_REQUIREMENT).expect("Invalid version requirement");

        let _ = dotenv::dotenv();
        let (has_debug, is_debug) = std::env::var("EVREDIS_DEBUG")
            .map(|x| (true, str::parse::<bool>(&x).unwrap_or(true)))
            .unwrap_or((false, false));

        let mut s = Config::new();

        let mut root = get_data_root(AppDataType::SharedConfig)?;
        root.push("evredis");
        root.push("evredis");
        s.merge(File::from(root).required(false))?;

        root = get_data_root(AppDataType::UserConfig)?;
        root.push("evredis");
        root.push("evredis");
        s.merge(File::from(root).required(false))?;

        if has_debug {
            s.merge(File::with_name("config/evredis").required(true))?;

            if is_debug {
                s.merge(File::with_name("config/evredis-debug").required(true))?;
            }
        }

        s.merge(
            Environment::with_prefix("evredis")
                .separator("_")
                .ignore_empty(true),
        )?;

        #[derive(Default, Debug, Clone, Deserialize)]
        #[serde(default)]
        struct TestConfiguration {
            meta: MetaConfiguration,
        }
        let test_config: TestConfiguration = s.clone().try_into()?;
        if let Some(ref version) = test_config.meta.version {
            if !version_req.matches(version) {
                return Err(ConfigurationError::IncompatibleVersion(
                    version.clone(),
                    version_req,
                ));
            }
        } else if version_req != VersionReq::parse("*").unwrap() {
            eprintln!("WARN: No configuration version specified; assuming compatibility");
        }

        Ok(s.try_into()?)
    }
}

/// A configuration metadata section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MetaConfiguration {
    pub version: Option<Version>,
}
