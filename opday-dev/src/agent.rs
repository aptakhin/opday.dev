#![warn(unused_extern_crates)]

use axum::{
    async_trait,
    body::Bytes,
    extract::{Form, FromRequest, Request, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    routing::post,
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use axum_extra::{
    headers::authorization::{Authorization, Bearer},
    TypedHeader,
};
use std::env;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{debug, info, warn};

use minijinja::{context, path_loader, Environment};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use uuid::Uuid;

use crate::auth;
use crate::settings;

use crate::auth::{
    ensure_push_header_authentification,
    Account,
    AccountActionOnTenant,
    AccountRepository,
    ApiAuth,
    ApiAuthRepository,
    // Authentificated,
    PushApiAuth,
    PushApiAuthRepository,
    // ensure_header_authentification
};
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
    let mut env = Environment::new();
    env.set_loader(path_loader("static/templates"));
    let tmpl = env.get_template("index.html.j2").unwrap();
    Html(tmpl.render(context!(name => "John")).unwrap())
}

async fn get_signin() -> Html<String> {
    let mut env = Environment::new();
    env.set_loader(path_loader("static/templates"));
    let tmpl = env.get_template("signin.html.j2").unwrap();
    Html(tmpl.render(context!(name => "John")).unwrap())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Signin {
    pub email: String,
    pub password: String,
}

#[derive()]
pub enum HtmlOrRedirect {
    Html(Html<String>),
    Redirect(Redirect),
}

impl IntoResponse for HtmlOrRedirect {
    fn into_response(self) -> Response {
        match self {
            HtmlOrRedirect::Html(html) => html.into_response(),
            HtmlOrRedirect::Redirect(redirect) => redirect.into_response(),
        }
    }
}

async fn post_signin(
    jar: CookieJar,
    State(pool): State<DbPool>,
    Form(signin_form): Form<Signin>,
) -> Result<(CookieJar, Redirect), Html<String>> {
    let account_repository = AccountRepository { pool: &pool };
    // let signin_response = Account::new(&account_repository)
    //     .signin(signin_form.email, signin_form.password)
    //     .await;

    // debug!("Log1 {:?}", signin_response);

    // if signin_response.is_err() {
    //     let mut env = Environment::new();
    //     env.set_loader(path_loader("static/templates"));
    //     let tmpl = env.get_template("signin.html.j2").unwrap();
    //     return Err(Html(tmpl.render(context!(name => "John")).unwrap()));
    // }

    // let api_auth_repository = ApiAuthRepository { pool: &pool };
    // let auth_token = ApiAuth::create_new(signin_response.unwrap(), &api_auth_repository).await;

    // debug!("Log2 {:?}", auth_token);

    // if auth_token.is_err() {
    //     // Internal error
    //     let mut env = Environment::new();
    //     env.set_loader(path_loader("static/templates"));
    //     let tmpl = env.get_template("signin.html.j2").unwrap();
    //     return Err(Html(tmpl.render(context!(name => "John")).unwrap()));
    // }

    // let token = auth_token.unwrap().token;

    Ok((
        jar.add(Cookie::new("_s".to_string(), "aaa")),
        Redirect::to("/dashboard"),
    ))
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
    let pool = make_db().await;

    let router: Router<()> = Router::new()
        .route("/", get(get_root))
        .route("/api/signin", get(get_signin).post(post_signin))
        .with_state(pool);

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
