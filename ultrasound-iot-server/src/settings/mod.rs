use config::{Config, ConfigError, File};
use serde::Deserialize;

pub mod db;
use db::Database;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: Database,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let base_path = std::env::current_dir().expect("Failed to determine the current directory");
        let config_dir = base_path.join("ultrasound-iot-server/config");

        let s = Config::builder()
            .add_source(File::from(config_dir.join("settings.toml")))
            .add_source(File::from(config_dir.join("local.toml")))
            .add_source(config::Environment::with_prefix("app"))
            .build()?;

        s.try_deserialize()
    }
}
