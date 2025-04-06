use std::collections::HashSet;
use std::path::PathBuf;

use clap::{Arg, Command};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct WebhookConfig {
    pub path: Option<String>,
    pub secret: Option<String>,
    pub events: HashSet<String>,
    pub shell: Option<Vec<String>>,
    pub command: Option<String>,
    pub script: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub port: u16,
    #[serde(skip)]
    pub verbose: String,
    pub webhooks: Vec<WebhookConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8080,
            verbose: "info".to_string(),
            webhooks: Vec::new(),
        }
    }
}

impl Config {
    pub fn merge(&mut self, other: Config) {
        // merge the two webhooks list without repeating the same path
        // in case of conflict, merge the events
        for other_webhook in other.webhooks {
            if let Some(index) = self
                .webhooks
                .iter()
                .position(|wh| wh.path == other_webhook.path)
            {
                let mut existing_webhook = self.webhooks.remove(index);
                existing_webhook.events.extend(other_webhook.events);
                self.webhooks.push(existing_webhook);
            } else {
                self.webhooks.push(other_webhook);
            }
        }
    }

    pub fn print_paths(&self) {
        for webhook in &self.webhooks {
            println!("Webhook path: {}", webhook.path);
            println!(
                "\tEvents: {}",
                webhook
                    .events
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}

pub fn parse_config(path: &PathBuf) -> Config {
    if path.is_dir() {
        let mut config = Config::default();
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_file() {
                let cfg_content = std::fs::read_to_string(entry.path()).unwrap();
                config.merge(
                    toml::from_str(&cfg_content)
                        .or_else(|_| serde_yaml::from_str(&cfg_content))
                        .or_else(|_| serde_json::from_str(&cfg_content))
                        .unwrap_or_default(),
                );
            }
        }
        config
    } else {
        let cfg_content = std::fs::read_to_string(path).unwrap();

        toml::from_str(&cfg_content)
            .or_else(|_| serde_yaml::from_str(&cfg_content))
            .or_else(|_| serde_json::from_str(&cfg_content))
            .unwrap_or_default()
    }
}

pub fn get_config() -> (PathBuf, Config) {
    let args = Command::new("grhooks")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("manifest-dir")
                .alias("config-dir")
                .env("GRHOOKS_MANIFEST_DIR")
                .required(true)
                .num_args(1)
                .help("Path to the configuration file")
                .value_parser(clap::builder::PathBufValueParser::new()),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .global(true)
                .action(clap::ArgAction::Count)
                .help("Enable verbose logging"),
        )
        .color(clap::ColorChoice::Always)
        .get_matches();

    let config_path = args
        .get_one::<PathBuf>("manifest-dir")
        .expect("No config file provided");

    let verbose =
        std::env::var("GRHOOKS_LOG").unwrap_or_else(|_| args.get_count("verbose").to_string());

    println!("Reading configs from path: {config_path:?}");
    let config = parse_config(config_path);

    (config_path.clone(), Config { verbose, ..config })
}
