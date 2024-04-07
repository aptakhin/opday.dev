use clap::Parser;

use axum::{http::StatusCode, routing::get, Router};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use std::net::SocketAddr;
use tokio_postgres::NoTls;

#[allow(unused_imports)]
use uuid::Uuid;

pub mod db;
pub mod model;

#[allow(unused_imports)]
use crate::db::{get_health_check_by_id, internal_error, ConnectionPool, DatabaseConnection};

use log::debug;

async fn root() -> &'static str {
    "ok"
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Verbose level
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Bind port
    #[arg(short, long)]
    port: u16,
}

async fn get_health_check(
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<String, (StatusCode, String)> {
    let health_check = get_health_check_by_id(conn).await.unwrap();

    Ok(serde_json::to_string(&health_check).unwrap())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    env_logger::init();

    let database_dsn =
        std::env::var("DATABASE_DSN").expect("Failed to parse DATABASE_DSN environment variable");

    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));

    debug!("Listening on: {}", addr);

    let manager = PostgresConnectionManager::new_from_stringlike(&database_dsn, NoTls).unwrap();
    let pool = Pool::builder().build(manager).await.unwrap();

    let app = Router::new()
        .route("/", get(root))
        .route("/api/v1/alive", get(root))
        .route("/api/v1/health-check", get(get_health_check))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest(
        args,
        case::just_port(vec!["", "-p", "3000"]),
    )]
    fn test_config_for_any_order(args: Vec<&str>) {
        assert!(Cli::try_parse_from(args).is_ok());
    }

    #[fixture]
    async fn db() -> ConnectionPool {
        let database_dsn = std::env::var("DATABASE_DSN")
            .expect("Failed to parse DATABASE_DSN environment variable");

        let manager = PostgresConnectionManager::new_from_stringlike(&database_dsn, NoTls).unwrap();
        let pool = Pool::builder().build(manager).await.unwrap();

        pool
    }

    #[fixture]
    async fn basic_health_check(#[future] db: ConnectionPool) -> Uuid {
        let db_awaited = db.await;
        let conn = db_awaited
            .get_owned()
            .await
            .map_err(internal_error)
            .expect("REASON");
        let rows = conn
            .query_one("INSERT INTO health_check
            (organization_id, name, url, expected_status_code) VALUES ($1, $2, $3, $4) RETURNING (id)", &[&Uuid::new_v4(), &"test", &"http://...", &200])
            .await.expect("REASON");
        assert_eq!(rows.len(), 1);
        let id_str: Uuid = rows.get(0);
        id_str
    }

    #[tokio::main]
    #[rstest]
    async fn test_query_health_check(
        #[future] db: ConnectionPool,
        #[future] basic_health_check: Uuid,
    ) {
        env_logger::init();

        let db_awaited = db.await;
        let conn = db_awaited
            .get_owned()
            .await
            .map_err(internal_error)
            .expect("REASON");
        let health_check_id = basic_health_check.await;
        let rows = conn
            .query(
                "SELECT * FROM health_check WHERE id=$1",
                &[&health_check_id],
            )
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        let value: &str = rows[0].try_get("name").expect("Failed");
        debug!("Hellovalue {}", value);
    }
}
