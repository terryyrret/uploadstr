use config::Config;
use config::FileFormat;
use config::File;

use poem::{Result, http::StatusCode, Error};


pub fn get_config_values(key: &str) -> Result<Vec<String>> {
    values(key)
    .ok_or_else(|| Error::from_string(
        "Could not read config...",
        StatusCode::INTERNAL_SERVER_ERROR
    ))
}

pub fn values(key: &str) -> Option<Vec<String>> {
    Config::builder()
        .set_default("bind", "0.0.0.0:3000").ok()?
        .set_default("baseUrl", "http://0.0.0.0:3000").ok()?
        .set_default("filesDir", "/var/uploadstr/files").ok()?
        .set_default("pubkeyWhitelist", Vec::<String>::new()).ok()?
        .add_source(File::new("/etc/uploadstr/config", FileFormat::Json))
        .build().ok()?
        .get(key).ok()
}

pub fn get_config_value(key: &str) -> Result<String> {
    value(key)
    .ok_or_else(|| Error::from_string(
        "Could not read config...",
        StatusCode::INTERNAL_SERVER_ERROR
    ))
}


pub fn value(key: &str) -> Option<String> {
    Config::builder()
        .set_default("bind", "0.0.0.0:3000").ok()?
        .set_default("baseUrl", "http://0.0.0.0:3000").ok()?
        .set_default("filesDir", "/var/uploadstr/files").ok()?
        .set_default("pubkeyWhitelist", Vec::<String>::new()).ok()?
        .add_source(File::new("/etc/uploadstr/config", FileFormat::Json))
        .build().ok()?
        .get(key).ok()
}
