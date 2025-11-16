#![warn(unused_extern_crates)]

use tracing::{debug, info, warn, error};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use clap::{Parser, Subcommand};
use std::fs;
use serde_derive::Deserialize;
use toml::Table;

mod settings;
mod exec;

fn init_logging() {
    use crate::settings::Settings;
    let settings = Settings::read_settings();
    if settings.local_run {
        let pretty_format = tracing_subscriber::fmt::format()
            .with_level(true)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .pretty();
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer().event_format(pretty_format))
            .init();
    } else {
        let json_format = tracing_subscriber::fmt::format()
            .with_level(true)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .json();
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer().event_format(json_format))
            .init();
    }
}

#[derive(Parser, Debug)]
#[command(name = "opday")]
#[command(author = "Alex")]
#[command(version = "1.0")]
#[command(about = "Operations Day CLI Tool", long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "opday.toml")]
    config: Option<std::path::PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Sync {
    },
}

#[derive(Subcommand, Debug)]
enum AgentCommands {
    /// Serve agent API
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
        /// Host address
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Debug, Deserialize)]
pub struct OpdayDevJust {
    #[serde(default)]
    pub just_hosts: Option<Vec<String>>,

    #[serde(default)]
    pub just_ssh_key_path: Option<String>,

    #[serde(default)]
    pub just_cr_credentials_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpdayDevProject {
    #[serde(default)]
    pub name: Option<String>,
}

use std::sync::Arc;
use std::any::Any;

/// A dynamically extensible registry: map TOML group names to parser functions
/// that deserialize the toml::Value into a boxed `Any` (so callers can
/// downcast to concrete types they expect).
type ParsedType = Box<dyn Any + Send + Sync>;
type ParserFn = dyn Fn(toml::Value) -> Result<ParsedType, toml::de::Error> + Send + Sync;

/// Standard type identifiers: compile-time constants to avoid string duplication
pub mod standard_types {
    pub const JUST: &str = "opday.dev/just";
    pub const PROJECT: &str = "opday.dev/project";
}

pub struct Registry {
    parsers: std::collections::HashMap<String, Arc<ParserFn>>,
}

impl Registry {
    pub fn new() -> Self {
        Self { parsers: std::collections::HashMap::new() }
    }

    /// Register a parser for key -> T where T implements Deserialize.
    pub fn register<T>(&mut self, key: &str)
    where
        T: 'static + Send + Sync + for<'de> serde::Deserialize<'de>,
    {
        let key_s = key.to_string();
        let parser = move |v: toml::Value| -> Result<ParsedType, toml::de::Error> {
            let t: T = v.try_into()?;
            Ok(Box::new(t) as ParsedType)
        };
        self.parsers.insert(key_s, Arc::new(parser));
    }

    /// Parse all known keys in `table` and return vector of (key, parsed_box)
    pub fn parse_all(&self, table: &toml::value::Table) -> Vec<(String, ParsedType)> {
        let mut out = Vec::new();
        let mut had_some_root_keys = false;
        for (key, val) in table.iter() {
            match val {
                toml::Value::Table(_) => {}
                _ => { had_some_root_keys = true; }
            }
            if let Some(parser) = self.parsers.get(key) {
                match parser(val.clone()) {
                    Ok(parsed) => out.push((key.clone(), parsed)),
                    Err(e) => warn!("Failed to parse registered type {}: {}", key, e),
                }
            } else {
                debug!("No registered parser for key {}", key);
            }
        }
        if had_some_root_keys {
            if let Some(parser) = self.parsers.get(standard_types::JUST) {
                match parser(toml::Value::Table(table.clone())) {
                    Ok(parsed) => out.push((standard_types::JUST.to_string(), parsed)),
                    Err(e) => warn!("Failed to parse registered root type: {}", e),
                }
            }
        }

        out
    }
}

#[tokio::main]
async fn main() {
    init_logging();

    let cli = Cli::parse();

    match cli.verbose {
        0 => debug!("Debug mode is off"),
        1 => debug!("Debug mode is on"),
        2 => debug!("Debug mode is very verbose"),
        _ => debug!("Don't be crazy"),
    }

    let config_path = std::env::var("OPDAY_CONFIG_PATH")
        .unwrap_or_else(|_| String::from("opday.toml"));

    let config_str = fs::read_to_string(config_path)
        .expect("Failed to read configuration file");

    let config: OpdayDevJust = toml::from_str(&config_str)
        .expect("Failed to parse TOML configuration");

    debug!("Debug {:#?}", config);

    match &cli.command {
        Commands::Sync { } => {
            info!("Running sync");
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use rstest::rstest;

    #[rstest(
        args,
        case::just_sync(vec!["", "sync"]),
    )]
    fn test_cli_cases_parsed(args: Vec<&str>) {
        assert!(Cli::try_parse_from(args).is_ok());
    }

    #[tokio::test]
    async fn test_config_parsing() {
        let toml_str = r#"
            just_hosts = ["host1", "host2"]
            just_ssh_key_path = "/path/to/ssh/key"
            just_cr_credentials_path = "/path/to/cr/credentials"
            another_field = "some_value"
        "#;

        let config: OpdayDevJust = toml::from_str(toml_str).expect("Failed to parse TOML configuration");

        assert_eq!(config.just_hosts.unwrap(), vec!["host1", "host2"]);
        assert_eq!(config.just_ssh_key_path.unwrap(), "/path/to/ssh/key");
        assert_eq!(config.just_cr_credentials_path.unwrap(), "/path/to/cr/credentials");
    }

    #[tokio::test]
    async fn test_config_parsing2() {
        let toml_str = r#"
            just_ssh_key_path = "key"
            ["opday.dev/project"]
            name = "aaa"
        "#;
        let value = toml_str.parse::<Table>().unwrap();
        let mut registry = Registry::new();
        registry.register::<OpdayDevJust>(standard_types::JUST);
        registry.register::<OpdayDevProject>(standard_types::PROJECT);

        let parsed = registry.parse_all(&value);

        let mut found_project = false;
        let mut found_just = false;
        for (key, boxed) in parsed {
            if key == standard_types::JUST {
                if let Ok(b) = boxed.downcast::<OpdayDevJust>() {
                    let proj: OpdayDevJust = *b;
                    assert_eq!(proj.just_ssh_key_path, Some("key".to_string()));
                    found_just = true;
                }
            } else if key == standard_types::PROJECT {
                if let Ok(b) = boxed.downcast::<OpdayDevProject>() {
                    let proj: OpdayDevProject = *b;
                    assert_eq!(proj.name, Some("aaa".to_string()));
                    found_project = true;
                }
            }
        }

        assert!(found_just, "just was not parsed by registry");
        assert!(found_project, "opday.dev/project was not parsed by registry");
    }
}
