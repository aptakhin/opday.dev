use clap::Parser;

use axum::{
    routing::{get, post},
    Json,
    Router,
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use serde::{Deserialize, Serialize};
use bb8::{Pool, PooledConnection};
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;
use std::{net::SocketAddr};
use tokio::net::TcpListener;
use uuid::Uuid;
use tokio_postgres::Error;

use log::debug;

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

// async fn create_user(
//     // this argument tells axum to parse the request body
//     // as JSON into a `CreateUser` type
//     Json(payload): Json<CreateUserRequest>,
// ) -> (StatusCode, Json<CreateUserResponse>) {
//     // insert your application logic here
//     let user = CreateUserResponse {
//         id: 1337,
//         username: payload.username,
//     };

//     // this will be converted into a JSON response
//     // with a status code of `201 Created`
//     (StatusCode::CREATED, Json(user))
// }

// #[derive(Deserialize)]
// struct CreateUserRequest {
//     username: String,
// }

// #[derive(Serialize)]
// struct CreateUserResponse {
//     id: u64,
//     username: String,
// }

// async fn query(db: DbClient) -> Result<(), Error> {
//     let db = db.lock().unwrap();
//     let rows = db.query("SELECT * FROM health_check", &[]).await?;

//     let value: &str = rows[0].get("name");

//     Ok(())
// }


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

type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

struct DatabaseConnection(PooledConnection<'static, PostgresConnectionManager<NoTls>>);

#[async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
where
    ConnectionPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = ConnectionPool::from_ref(state);

        let conn = pool.get_owned().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

async fn using_connection_extractor(
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<String, (StatusCode, String)> {
    let rows = conn
        .query("SELECT * FROM health_check", &[])
        .await
        .map_err(internal_error)?;
    let value: &str = rows[0].get("name");

    Ok(value.to_string())
}

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    env_logger::init();

    let database_dsn =
        std::env::var("DATABASE_DSN").expect("Failed to parse DATABASE_DSN environment variable");

    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));

    debug!("Listening on: {}", addr);

    let manager =
        PostgresConnectionManager::new_from_stringlike(&database_dsn, NoTls)
            .unwrap();
    let pool = Pool::builder().build(manager).await.unwrap();

    let app = Router::new()
        .route("/", get(root))
        .route("/hello", get(using_connection_extractor))
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

        let manager =
            PostgresConnectionManager::new_from_stringlike(&database_dsn, NoTls)
                .unwrap();
        let pool = Pool::builder().build(manager).await.unwrap();

        pool
    }

    #[fixture]
    async fn basic_health_check(#[future] db: ConnectionPool) -> Uuid {
        let db_awaited = db.await;
        let conn = db_awaited.get_owned().await.map_err(internal_error).expect("REASON");
        let rows = conn
            .query("INSERT INTO health_check
            (organization_id, name, url, expected_status_code) VALUES ($1, $2, $3, $4) RETURNING (id)", &[&Uuid::new_v4(), &"test", &"http://...", &200])
            .await.expect("REASON");
        assert_eq!(rows.len(), 1);
        let id_str: Uuid = rows[0].get(0);
        id_str
    }

    #[tokio::main]
    #[rstest]
    async fn test_query_health_check(
        #[future] db: ConnectionPool,
        #[future] basic_health_check: Uuid,
    ) -> Result<(), Error> {
        env_logger::init();

        let db_awaited = db.await;
        let conn = db_awaited.get_owned().await.map_err(internal_error).expect("REASON");
        let health_check_id = basic_health_check.await;
        let rows = conn
            .query(
                "SELECT * FROM health_check WHERE id=$1",
                &[&health_check_id],
            )
            .await?;
        assert_eq!(rows.len(), 1);
        let value: &str = rows[0].try_get("name").expect("Failed");
        debug!("Hellovalue {}", value);
        Ok(())
    }
}
