use clap::Parser;

use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use db::insert_health_check;
use model::HealthCheckModel;
use std::net::SocketAddr;
use std::str;
use tokio_postgres::NoTls;
use uuid::Uuid;

pub mod db;
pub mod model;

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

#[debug_handler]
async fn post_health_check(
    State(pool): State<ConnectionPool>,
    // DatabaseConnection(pool): DatabaseConnection,
    Json(payload): Json<HealthCheckModel>,
) -> Result<String, (StatusCode, String)> {
    let conn = pool.get_owned().await.map_err(internal_error)?;

    let id = insert_health_check(payload, conn).await;

    let id_response = model::IdResponse {
        success: true,
        id: Some(id),
        error: None,
    };
    Ok(serde_json::to_string(&id_response).unwrap())
}

async fn get_health_check(
    Path(id): Path<Uuid>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<String, (StatusCode, String)> {
    let health_check = get_health_check_by_id(id, conn).await.unwrap();
    Ok(serde_json::to_string(&health_check).unwrap())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    env_logger::init();

    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));

    debug!("Listening on: {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app().await).await.unwrap();
}

async fn app() -> Router {
    let database_dsn =
        std::env::var("DATABASE_DSN").expect("Failed to parse DATABASE_DSN environment variable");

    let manager = PostgresConnectionManager::new_from_stringlike(&database_dsn, NoTls).unwrap();
    let pool = Pool::builder().build(manager).await.unwrap();

    Router::new()
        .route("/", get(root))
        .route("/api/v1/alive", get(root))
        .route("/api/v1/health-check", post(post_health_check))
        .route("/api/v1/health-check/:id", get(get_health_check))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use rstest::*;
    use tower::ServiceExt;

    #[rstest(
        args,
        case::just_port(vec!["", "-p", "3000"]),
    )]
    fn test_config_for_any_order(args: Vec<&str>) {
        assert!(Cli::try_parse_from(args).is_ok());
    }

    #[tokio::test]
    async fn get_root() {
        let app = app().await;

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn get_alive() {
        let app = app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/alive")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn post_health_check() {
        let app = app().await;

        let model = HealthCheckModel {
            name: "test".to_string(),
        };

        let request = Request::post("/api/v1/health-check")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&model).unwrap()));

        let response = app.oneshot(request.unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        use serde_json::Value;

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let res = str::from_utf8(&body).unwrap();
        let v: Value = serde_json::from_str(&res).unwrap();

        assert_eq!(v["success"], true);
    }
}
