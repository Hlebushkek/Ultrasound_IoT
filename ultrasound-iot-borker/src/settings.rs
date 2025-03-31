use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Broker {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub broker: Broker,
    pub server: Server,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let base_path = std::env::current_dir().expect("Failed to determine the current directory");
        let config_dir = base_path.join("config");

        let s = Config::builder()
            .add_source(File::from(config_dir.join("settings.toml")))
            .add_source(File::from(config_dir.join("local.toml")).required(false))
            .add_source(config::Environment::with_prefix("UIOT"))
            .build()?;

        s.try_deserialize()
    }
}
