use clap::Parser;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use std::net::SocketAddr;
use std::str;
use tokio_postgres::NoTls;
use uuid::Uuid;

pub mod db;
pub mod model;

use crate::db::{
    get_health_check_by_id, insert_health_check, internal_error, update_health_check,
    ConnectionPool, DatabaseConnection,
};
use crate::model::{
    HealthCheckModelGetResponse, HealthCheckModelUpdateRequest, IdResponse,
    InsertHealthCheckModelRequest, OpdayError,
};

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

async fn post_health_check(
    State(pool): State<ConnectionPool>,
    // DatabaseConnection(pool): DatabaseConnection,
    Json(insert_request): Json<InsertHealthCheckModelRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = pool.get_owned().await.map_err(internal_error)?;

    let organization_id = Uuid::new_v4();
    let id = insert_health_check(organization_id, insert_request, conn).await;

    let id_response = IdResponse {
        success: true,
        id: Some(id),
        error: None,
    };
    Ok((StatusCode::OK, Json(id_response)))
}

async fn post_update_health_check(
    Path(id): Path<Uuid>,
    State(pool): State<ConnectionPool>,
    // DatabaseConnection(pool): DatabaseConnection,
    Json(post_request): Json<HealthCheckModelUpdateRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = pool.get_owned().await.map_err(internal_error)?;

    let id = update_health_check(id, post_request, conn).await;

    let id_response = IdResponse {
        success: true,
        id: Some(id),
        error: None,
    };
    Ok((StatusCode::OK, Json(id_response)))
}

async fn get_health_check(
    Path(id): Path<Uuid>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let health_check = get_health_check_by_id(id, conn).await;

    let error = if health_check.is_none() {
        Some(OpdayError {
            error: "not_found".to_string(),
            error_loc: "Not found.".to_string(),
        })
    } else {
        None
    };

    let response = HealthCheckModelGetResponse {
        success: health_check.is_some(),
        model: health_check,
        error,
    };

    Ok((StatusCode::OK, Json(response)))
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
        .route("/api/v1/health-check/:id", post(post_update_health_check))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use db::tests::{basic_health_check, basic_health_check_model};
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

    #[rstest]
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

    #[rstest]
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

    #[rstest]
    #[tokio::test]
    async fn post_health_check(basic_health_check_model: InsertHealthCheckModelRequest) {
        let app = app().await;
        let model = basic_health_check_model;
        let request = Request::post("/api/v1/health-check")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&model).unwrap()));

        let response = app.oneshot(request.unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = str::from_utf8(&body).unwrap();
        let response: IdResponse = serde_json::from_str(&body_str).unwrap();
        assert_eq!(response.success, true);
    }

    async fn call_get_health_check_by_id(id: &Uuid, app: Router) -> HealthCheckModelGetResponse {
        let request =
            Request::get("/api/v1/health-check/".to_owned() + &id.to_string()).body(Body::from(""));
        let response = app.oneshot(request.unwrap()).await.unwrap();
        let response_status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = str::from_utf8(&body).unwrap();
        let id_response: HealthCheckModelGetResponse = serde_json::from_str(&body_str).unwrap();
        assert_eq!(response_status, StatusCode::OK, "raw: {}", &body_str);
        id_response
    }

    #[rstest]
    #[tokio::test]
    async fn test_update_health_check(
        #[future] basic_health_check: Uuid,
        basic_health_check_model: InsertHealthCheckModelRequest,
    ) {
        let app = app().await;
        let mut model = basic_health_check_model;
        model.name = "new_name".to_string();
        let basic_health_check_id = basic_health_check.await;
        let request =
            Request::post("/api/v1/health-check/".to_owned() + &basic_health_check_id.to_string())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&model).unwrap()));

        let response = app.clone().oneshot(request.unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = str::from_utf8(&body).unwrap();
        let response: IdResponse = serde_json::from_str(&body_str).unwrap();
        assert_eq!(response.success, true);
        let check_id_response =
            call_get_health_check_by_id(&basic_health_check_id, app.clone()).await;
        assert!(check_id_response.success, "raw: {:?}", &check_id_response);
        let model_w = check_id_response.model.clone().unwrap();
        assert_eq!(model_w.name, model.name, "raw: {:?}", &check_id_response);
    }

    #[rstest]
    #[tokio::test]
    async fn get_health_check_existing(#[future] basic_health_check: Uuid) {
        let app = app().await;
        let basic_health_check_id = basic_health_check.await;

        let response: HealthCheckModelGetResponse =
            call_get_health_check_by_id(&basic_health_check_id, app.clone()).await;

        assert!(response.success, "raw: {:?}", &response);
        assert!(
            response.model.clone().unwrap().name.len() > 0,
            "raw: {:?}",
            &response
        );
        assert!(response.error.is_none(), "raw: {:?}", &response);
    }

    #[fixture]
    fn not_existing_basic_health_check() -> Uuid {
        Uuid::default()
    }

    #[rstest]
    #[tokio::test]
    async fn get_health_check_not_existing(not_existing_basic_health_check: Uuid) {
        let app = app().await;

        let response: HealthCheckModelGetResponse =
            call_get_health_check_by_id(&not_existing_basic_health_check, app.clone()).await;

        assert!(!response.success, "raw: {:?}", &response);
        assert!(response.model.clone().is_none(), "raw: {:?}", &response);
        assert!(response.error.is_some(), "raw: {:?}", &response);
    }
}
