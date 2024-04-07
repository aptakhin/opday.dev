use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
// use serde_derive::Deserialize;
use bb8::{Pool, PooledConnection};
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;

use crate::model::HealthCheckModel;

pub type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

pub type DbConn = PooledConnection<'static, PostgresConnectionManager<NoTls>>;

pub struct DatabaseConnection(pub DbConn);

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

pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

pub async fn get_health_check_by_id(conn: DbConn) -> Option<HealthCheckModel> {
    let row = conn
        .query_one("SELECT * FROM health_check", &[])
        .await
        .ok()?;
    let value: &str = row.get("name");

    Some(HealthCheckModel {
        name: value.to_string(),
    })
}
