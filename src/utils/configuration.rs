use std::error;
use std::fmt;

use app_dirs::*;
use config::{Config, ConfigError, Environment, File};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};

use semver::{Version, VersionReq};

#[derive(Debug)]
pub enum ConfigurationError {
    LoadFailure(ConfigError),
    MissingLocation(app_dirs::AppDirsError),
    IncompatibleVersion(Version, VersionReq),
}
impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigurationError::LoadFailure(err) => write!(f, "Load failure: {}", err),
            ConfigurationError::MissingLocation(err) => write!(f, "Missing location: {}", err),
            ConfigurationError::IncompatibleVersion(actual, expected) => write!(
                f,
                "Configuration of version {} incompatible with requirement {}",
                actual, expected
            ),
        }
    }
}
impl error::Error for ConfigurationError {}
impl From<ConfigError> for ConfigurationError {
    fn from(err: ConfigError) -> Self {
        ConfigurationError::LoadFailure(err)
    }
}
impl From<app_dirs::AppDirsError> for ConfigurationError {
    fn from(err: app_dirs::AppDirsError) -> Self {
        ConfigurationError::MissingLocation(err)
    }
}

pub trait Configuration: DeserializeOwned {
    const VERSION_REQUIREMENT: &'static str = "*";

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MetaConfiguration {
    pub version: Option<Version>,
}
