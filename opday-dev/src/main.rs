#![warn(unused_extern_crates)]

use tracing::{debug, info, warn};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use clap::{Parser, Subcommand};
use std::fs;
use serde_derive::Deserialize;

mod settings;
mod agent;

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
    /// Optional name to operate on
    #[arg(short, long)]
    name: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE.toml")]
    config: Option<std::path::PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Deploy
    Deploy {
    },
    /// Manages agents
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
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
pub struct Config {
    #[serde(default)]
    pub just_hosts: Option<Vec<String>>,

    #[serde(default)]
    pub just_ssh_key_path: Option<String>,

    #[serde(default)]
    pub just_cr_credentials_path: Option<String>,
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

    let config: Config = toml::from_str(&config_str)
        .expect("Failed to parse TOML configuration");

    debug!("Debug {:#?}", config);

    match &cli.command {
        Commands::Deploy { } => {
            info!("Running deploy");
        }
        Commands::Agent { command } => {
            match command {
                AgentCommands::Serve { port, host, .. } => {
                    // info!("Creating agent {} of type {}", name, agent_type);
                    let addr = format!("{}:{}", host, port);
                    let listener = TcpListener::bind(&addr).await.unwrap();
                    info!("Listening on {}", listener.local_addr().unwrap());

                    let app = agent::routes_app().await;
                    axum::serve(
                        listener,
                        app.into_make_service_with_connect_info::<SocketAddr>(),
                    )
                    .await
                    .unwrap();
                }
            }
        }
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
        case::just_docker(vec!["", "deploy"]),
        case::just_sync(vec!["", "sync"]),
    )]
    fn test_cli_cases_parsed(args: Vec<&str>) {
        assert!(Cli::try_parse_from(args).is_ok());
    }

    #[tokio::test]
    async fn test_cli_serve_api() {
        let args = vec![
            "opday",
            "--verbose",
            "agent",
            "serve",
            "--port",
            "8080",
            "--host",
            "127.0.0.1",
        ];

        let cli = Cli::parse_from(args);

        assert_eq!(cli.verbose, 1);
        assert!(matches!(cli.command, Commands::Agent { command: AgentCommands::Serve { port, ref host, .. } }));
        if let Commands::Agent { command } = cli.command {
            if let AgentCommands::Serve { port, ref host, .. } = command {
                assert_eq!(port, 8080);
                assert_eq!(host, "127.0.0.1");
            }
        }
    }

    #[tokio::test]
    async fn test_config_parsing() {
        let toml_str = r#"
            just_hosts = ["host1", "host2"]
            just_ssh_key_path = "/path/to/ssh/key"
            just_cr_credentials_path = "/path/to/cr/credentials"
        "#;

        let config: Config = toml::from_str(toml_str).expect("Failed to parse TOML configuration");

        assert_eq!(config.just_hosts.unwrap(), vec!["host1", "host2"]);
        assert_eq!(config.just_ssh_key_path.unwrap(), "/path/to/ssh/key");
        assert_eq!(config.just_cr_credentials_path.unwrap(), "/path/to/cr/credentials");
    }
}
