use config::File;
use config::FileFormat;

use poem::{http::StatusCode, Error, Result};
use serde::de::Deserialize;

#[cfg(test)]
use mockall::predicate::str;

fn get_value<'de, T: Deserialize<'de>>(key: &str) -> Result<T, config::ConfigError> {
    config::Config::builder()
        .set_default("bind", "0.0.0.0:3000")?
        .set_default("baseUrl", "http://0.0.0.0:3000")?
        .set_default("filesDir", "/var/uploadstr/files")?
        .set_default("pubkeyWhitelist", Vec::<String>::new())?
        .add_source(File::new("/etc/uploadstr/config", FileFormat::Json))
        .build()?
        .get::<T>(key)
}

#[cfg_attr(test, mockall::automock)]
pub trait Ops {
    fn get_config_values(&self, key: &str) -> Result<Vec<String>> {
        get_value(key).map_err(|_| {
            Error::from_string(
                "Could not read config...",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })
    }

    fn get_config_value(&self, key: &str) -> Result<String> {
        get_value(key).map_err(|_| {
            Error::from_string(
                "Could not read config...",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })
    }
}

pub struct Config {}
impl Config {
    pub fn new() -> Self {
        Self {}
    }
}
impl Ops for Config {}
