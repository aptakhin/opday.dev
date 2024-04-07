use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use bb8::{Pool, PooledConnection};
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;
use uuid::Uuid;

use crate::model::{
    HealthCheckModel, HealthCheckModelUpdateRequest, InsertHealthCheckModelRequest,
};

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

pub async fn get_health_check_by_id(id: Uuid, conn: DbConn) -> Option<HealthCheckModel> {
    let row = conn
        .query_one("SELECT * FROM health_check WHERE id = $1", &[&id])
        .await
        .ok()?;
    Some(HealthCheckModel {
        name: row.get::<&str, &str>("name").to_string(),
    })
}

pub async fn insert_health_check(
    organization_id: Uuid,
    model: InsertHealthCheckModelRequest,
    conn: DbConn,
) -> Uuid {
    let row = conn
        .query_one(
            "
            INSERT INTO health_check
                (organization_id, name, url, expected_status_code)
            VALUES
                ($1, $2, $3, $4)
            RETURNING (id)",
            &[&organization_id, &model.name, &model.url, &200],
        )
        .await
        .expect("REASON");
    row.get(0)
}

pub async fn update_health_check(
    id: Uuid,
    model: HealthCheckModelUpdateRequest,
    conn: DbConn,
) -> Uuid {
    let row = conn
        .query_one(
            "
            UPDATE health_check
            SET
                name = $2,
                url = $3
            WHERE id = $1
            RETURNING id",
            &[&id, &model.name, &model.url],
        )
        .await
        .expect("REASON");
    row.get(0)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use log::debug;
    use rstest::*;

    #[fixture]
    async fn pool() -> ConnectionPool {
        let database_dsn = std::env::var("DATABASE_DSN")
            .expect("Failed to parse DATABASE_DSN environment variable");

        let manager = PostgresConnectionManager::new_from_stringlike(&database_dsn, NoTls).unwrap();
        let pool = Pool::builder().build(manager).await.unwrap();
        pool
    }

    #[fixture]
    async fn conn(#[future] pool: ConnectionPool) -> DbConn {
        let pool_awaited = pool.await;
        let conn = pool_awaited
            .get_owned()
            .await
            .map_err(internal_error)
            .expect("REASON");
        conn
    }

    #[fixture]
    pub fn basic_health_check_model() -> InsertHealthCheckModelRequest {
        InsertHealthCheckModelRequest {
            name: "test".to_string(),
            url: "http://localhost:8080".to_string(),
        }
    }

    #[fixture]
    pub async fn basic_health_check(
        basic_health_check_model: InsertHealthCheckModelRequest,
        #[future] conn: DbConn,
    ) -> Uuid {
        let conn_awaited = conn.await;
        let organization_id = Uuid::new_v4();
        let id = insert_health_check(organization_id, basic_health_check_model, conn_awaited).await;
        id
    }

    #[tokio::main]
    #[rstest]
    async fn test_query_health_check(#[future] conn: DbConn, #[future] basic_health_check: Uuid) {
        env_logger::init();

        let conn_awaited = conn.await;
        let health_check_id = basic_health_check.await;
        let health_check = get_health_check_by_id(health_check_id, conn_awaited)
            .await
            .unwrap();
        debug!("Hellovalue {}", health_check.name);
    }
}
