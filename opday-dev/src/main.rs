#![warn(unused_extern_crates)]

use tracing::{debug, info, warn};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;

use uuid::Uuid;

mod auth;
mod settings;
mod agent;

type DbPool = PgPool;

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

#[tokio::main]
async fn main() {
    init_logging();

    // let args: Vec<String> = env::args().collect();

    // if args.len() == 2 {
    //     if args[1] == "migrate" {
    //         let pool = make_db().await;
    //         info!("Starting migrations");
    //         let result = sqlx::migrate!("db/migrations")
    //             .run(&pool)
    //             .await
    //             .expect("Migrations panic!");
    //         info!("Migration result {:?}", result);
    //         return;
    //     }
    // }

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    info!("Listening on {}", listener.local_addr().unwrap());

    let app = agent::routes_app().await;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

#[cfg(test)]
pub mod test {
    use super::*;

}
