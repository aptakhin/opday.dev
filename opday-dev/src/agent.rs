#![warn(unused_extern_crates)]

use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use tracing::{warn};

use minijinja::{context, path_loader, Environment};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

use crate::settings::Settings;

type DbPool = PgPool;

async fn make_db() -> DbPool {
    let db_connection_str = Settings::read_settings().postgres_dsn;
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_connection_str)
        .await
        .expect("Can't connect to the database")
}

async fn get_root() -> Html<String> {
    // let mut env = Environment::new();
    // env.set_loader(path_loader("static/templates"));
    // let tmpl = env.get_template("index.html.j2").unwrap();
    // Html(tmpl.render(context!(name => "John")).unwrap())
    Html("<h1>Welcome to Opday Agent</h1>".to_string())
}


struct BufferRequestBody(Bytes);

#[async_trait]
impl<S> FromRequest<S> for BufferRequestBody
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let body = Bytes::from_request(req, state)
            .await
            .map_err(|err| err.into_response())?;

        Ok(Self(body))
    }
}

pub async fn routes_app() -> Router<()> {
    // let pool = make_db().await;

    let router: Router<()> = Router::new()
        .route("/", get(get_root));
        // .with_state(pool);

    router
}

#[cfg(test)]
pub mod test {
    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use hex::encode;
    use http_body_util::BodyExt;
    use rstest::{fixture, rstest};
    use serde_json::json;
    use tower::util::ServiceExt;
    use tracing_test::traced_test;

    #[fixture]
    pub async fn app() -> Router<()> {
        routes_app().await
    }

    #[fixture]
    pub async fn pool() -> DbPool {
        make_db().await
    }

    #[rstest]
    #[case("/")]
    #[tokio::test]
    #[traced_test]
    async fn get_root(#[case] s: impl AsRef<str>, #[future] app: Router<()>) {
        let response = app
            .await
            .oneshot(
                Request::builder()
                    .uri(s.as_ref())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(!body_str.is_empty(), "Response string should not be empty");
    }
}
