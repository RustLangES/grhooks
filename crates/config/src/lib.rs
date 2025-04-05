use std::path::PathBuf;

use clap::{Arg, Command};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct WebhookConfig {
    pub path: Option<String>,
    pub secret: Option<String>,
    pub events: Vec<String>,
    pub shell: Option<Vec<String>>,
    pub command: Option<String>,
    pub script: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub port: u16,
    pub webhooks: Vec<WebhookConfig>,
}

pub fn get_config() -> Config {
    let args = Command::new("grhooks")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("manifest")
                .alias("config")
                .env("GRHOOKS_MANIFEST")
                .required(true)
                .num_args(1)
                .help("Path to the configuration file")
                .value_parser(clap::builder::PathBufValueParser::new()),
        )
        .color(clap::ColorChoice::Always)
        .get_matches();

    let config_path = args
        .get_one::<PathBuf>("manifest")
        .expect("No config file provided");

    println!("Reading configs from path: {config_path:?}");

    let cfg_content = std::fs::read_to_string(&config_path).unwrap();

    toml::from_str(&cfg_content)
        .or_else(|_| serde_yaml::from_str(&cfg_content))
        .or_else(|_| serde_json::from_str(&cfg_content))
        .expect("Failed to parse config")
}
