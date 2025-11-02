use crate::DbPool;
use hex::encode;

use uuid::Uuid;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::debug;

use crate::settings::Settings;
use axum::{
    async_trait,
    extract::{FromRef, FromRequest, Request},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use axum_extra::{
    headers::authorization::{Authorization, Bearer},
    TypedHeader,
};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Error, Debug, Clone)]
pub enum AuthError {
    #[error("Invalid authentication token")]
    InvalidToken,
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Password mismatch")]
    PasswordMismatch,
    #[error("Internal error: {0}")]
    InternalError(String),
}

#[derive(Debug)]
pub struct Token {
    token: String,
}

impl Token {
    pub fn new(user_id: String, _secret_key: &[u8]) -> Result<Self, AuthError> {
        let salt: [u8; 16] = rand::thread_rng().gen();
        let salt_hex = hex::encode(salt);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AuthError::InternalError(e.to_string()))?
            .as_secs()
            .to_string();

        let message = format!("{}{}{}", user_id, timestamp, salt_hex);

        Ok(Token {
            token: hash_password(message),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Authenticated(pub Uuid);

#[async_trait]
impl<S> FromRequest<S> for Authenticated
where
    DbPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers();
        let cookies = CookieJar::from_headers(headers);

        let token = extract_token(headers, &cookies).ok_or(StatusCode::UNAUTHORIZED)?;
        let pool = DbPool::from_ref(state);
        let api_auth_repository = ApiAuthRepository { pool: &pool };

        ApiAuth::from(token, &api_auth_repository)
            .await
            .map(|auth| Authenticated(auth.account_id))
            .map_err(|_| StatusCode::UNAUTHORIZED)
    }
}

fn extract_token(headers: &axum::http::HeaderMap, cookies: &CookieJar) -> Option<String> {
    let header_token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let cookie_token = cookies.get("_s").map(|c| c.value().to_string());

    header_token.or(cookie_token).map(|mut token| {
        if token.starts_with("Bearer ") {
            token = token.strip_prefix("Bearer ").unwrap_or(&token).to_string();
        }
        token
    })
}

pub async fn ensure_header_authentication(
    header: TypedHeader<Authorization<Bearer>>,
    pool: &DbPool,
) -> Result<Authenticated, AuthError> {
    let header_token = header.0.token().to_string();
    let api_auth_repository = ApiAuthRepository { pool };

    ApiAuth::from(header_token, &api_auth_repository)
        .await
        .map(|auth| Authenticated(auth.account_id))
        .map_err(|_| AuthError::InvalidToken)
}

#[derive(Debug, Clone)]
pub struct PushApiInfo {
    pub environment_id: Uuid,
}

pub async fn ensure_push_header(
    header_token: String,
    pool: &DbPool,
) -> Result<PushApiInfo, AuthError> {
    let push_api_repository = PushApiAuthRepository { pool: pool };
    let resp = PushApiAuth::from(header_token, &push_api_repository).await;
    match resp {
        Ok(auth) => Ok(PushApiInfo {
            environment_id: auth.environment_id,
        }),
        Err(_) => Err(AuthError::InvalidToken),
    }
}

pub async fn ensure_push_header_authentification(
    header: TypedHeader<Authorization<Bearer>>,
    pool: &DbPool,
) -> Result<PushApiInfo, AuthError> {
    let header_token = header.0.token().to_string();
    ensure_push_header(header_token, pool).await
}

pub enum AccountActionOnTenant {
    CanLinkAccount,
}

fn hash_password(password: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(Settings::read_settings().get_binary_secret_key());
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    encode(result)
}

pub struct AccountRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> AccountRepository<'a> {
    pub async fn signin(&self, email: String, hashed_password: String) -> Result<Uuid, AuthError> {
        let result = sqlx::query_as::<_, (bool, Uuid, String)>(
            "
            WITH ins AS (
                INSERT INTO account (email, password)
                VALUES ($1, $2)
                ON CONFLICT (email) DO NOTHING
                RETURNING id, password
            )
            SELECT true AS is_inserted, id AS account_id, password FROM ins
            UNION ALL
            SELECT false AS is_inserted, id AS account_id, password FROM account WHERE email = $1
            LIMIT 1
            ",
        )
        .bind(&email)
        .bind(&hashed_password)
        .fetch_one(self.pool)
        .await
        .map_err(|e| {
            debug!("Database query error: {}", e);
            AuthError::DatabaseError(e.to_string())
        })?;

        let (_is_inserted, account_id, db_hashed_password) = result;

        if hashed_password != db_hashed_password {
            return Err(AuthError::PasswordMismatch);
        }

        Ok(account_id)
    }

    pub async fn ensure_permissions_on_tenant(
        &self,
        account_id: Uuid,
        tenant_id: Uuid,
        _action: AccountActionOnTenant,
    ) -> Result<(), AuthError> {
        sqlx::query_as::<_, (Uuid, Uuid, Uuid)>(
            "
            SELECT id, account_id, tenant_id FROM tenant_and_account
            WHERE account_id = $1 AND tenant_id = $2
            ",
        )
        .bind(account_id)
        .bind(tenant_id)
        .fetch_one(self.pool)
        .await
        .map(|_| ())
        .map_err(|e| {
            debug!("Database query error: {}", e);
            AuthError::DatabaseError(e.to_string())
        })
    }
}

pub struct Account<'a> {
    repo: &'a AccountRepository<'a>,
}

impl<'a> Account<'a> {
    pub fn new(repo: &'a AccountRepository<'a>) -> Account<'a> {
        Account { repo }
    }

    // pub async fn signin(&self, email: String, password: String) -> Result<Uuid, String> {
    //     let hashed_password = hash_password(password);
    //     self.repo.signin(email, hashed_password).await
    // }

    // pub async fn ensure_permissions_on_tenant(
    //     &self,
    //     account_id: Uuid,
    //     tenant_id: Uuid,
    //     action: AccountActionOnTenant,
    // ) -> Result<(), ()> {
    //     self.repo
    //         .ensure_permissions_on_tenant(account_id, tenant_id, action)
    //         .await
    // }
}

#[derive(Debug)]
pub struct ApiAuthRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> ApiAuthRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn add_token(&self, token: String, account_id: Uuid) -> Result<Uuid, AuthError> {
        let device_id = "device_id".to_string();
        sqlx::query_as::<_, (bool, Uuid)>(
            "
            INSERT INTO auth_token
            (token, type, account_id, device_id)
            VALUES ($1, $2, $3, $4)
            RETURNING true, id
            ",
        )
        .bind(&token)
        .bind("auth")
        .bind(account_id)
        .bind(&device_id)
        .fetch_one(self.pool)
        .await
        .map(|(_, id)| id)
        .map_err(|e| {
            debug!("Database query error: {}", e);
            AuthError::DatabaseError(e.to_string())
        })
    }

    pub async fn check_token(&self, token: String) -> Result<Uuid, AuthError> {
        sqlx::query_as::<_, (Uuid,)>(
            "
            SELECT account_id
            FROM auth_token
            WHERE token = $1 AND type = $2
            LIMIT 1
            ",
        )
        .bind(&token)
        .bind("auth")
        .fetch_one(self.pool)
        .await
        .map(|(id,)| id)
        .map_err(|e| {
            debug!("Database query error: {}", e);
            AuthError::DatabaseError(e.to_string())
        })
    }
}

#[derive(Debug)]
pub struct ApiAuth<'a> {
    pub account_id: Uuid,
    pub token: String,
    auth_repo: &'a ApiAuthRepository<'a>,
}

impl<'a> ApiAuth<'a> {
    pub async fn from(
        token: String,
        auth_repo: &'a ApiAuthRepository<'a>,
    ) -> Result<Self, AuthError> {
        let account_id = auth_repo.check_token(token.clone()).await?;
        Ok(Self {
            account_id,
            token,
            auth_repo,
        })
    }

    pub async fn create_new(
        account_id: Uuid,
        auth_repo: &'a ApiAuthRepository<'a>,
    ) -> Result<Self, AuthError> {
        let token = Self::generate_token(account_id)?;
        auth_repo.add_token(token.clone(), account_id).await?;

        Ok(Self {
            account_id,
            token,
            auth_repo,
        })
    }

    fn generate_token(account_id: Uuid) -> Result<String, AuthError> {
        let secret_key = rand::thread_rng().gen::<[u8; 32]>();
        Token::new(encode(account_id), &secret_key).map(|t| t.token)
    }
}

#[derive(Debug)]
pub struct PushApiAuthRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> PushApiAuthRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn add_token(
        &self,
        token: String,
        environment_id: Uuid,
        by_account_id: Uuid,
    ) -> Result<Uuid, AuthError> {
        sqlx::query_as::<_, (bool, Uuid)>(
            "
            INSERT INTO push_token
            (token, environment_id, created_by_account_id)
            VALUES ($1, $2, $3)
            RETURNING true, id
            ",
        )
        .bind(&token)
        .bind(environment_id)
        .bind(by_account_id)
        .fetch_one(self.pool)
        .await
        .map(|(_, id)| id)
        .map_err(|e| {
            debug!("Database query error: {}", e);
            AuthError::DatabaseError(e.to_string())
        })
    }

    pub async fn check_token(&self, token: String) -> Result<Uuid, AuthError> {
        sqlx::query_as::<_, (Uuid,)>(
            "
            SELECT environment_id
            FROM push_token
            WHERE token = $1
            LIMIT 1
            ",
        )
        .bind(&token)
        .fetch_one(self.pool)
        .await
        .map(|(id,)| id)
        .map_err(|e| {
            debug!("Database query error: {}", e);
            AuthError::DatabaseError(e.to_string())
        })
    }

    pub fn generate_token(environment_id: Uuid) -> Result<String, AuthError> {
        let secret_key = rand::thread_rng().gen::<[u8; 32]>();
        Token::new(encode(environment_id), &secret_key).map(|t| format!("pt-{}", t.token))
    }
}

#[derive(Debug)]
pub struct PushApiAuth<'a> {
    pub environment_id: Uuid,
    pub token: String,
    api_auth_repo: &'a PushApiAuthRepository<'a>,
}

impl<'a> PushApiAuth<'a> {
    pub async fn from(
        token: String,
        api_auth_repo: &'a PushApiAuthRepository<'a>,
    ) -> Result<Self, AuthError> {
        let environment_id = api_auth_repo.check_token(token.clone()).await?;
        Ok(Self {
            environment_id,
            token,
            api_auth_repo,
        })
    }

    pub async fn create_new(
        environment_id: Uuid,
        api_auth_repo: &'a PushApiAuthRepository<'a>,
        by_account_id: Uuid,
    ) -> Result<Self, AuthError> {
        let token = PushApiAuthRepository::generate_token(environment_id)?;
        api_auth_repo
            .add_token(token.clone(), environment_id, by_account_id)
            .await?;

        Ok(Self {
            environment_id,
            token,
            api_auth_repo,
        })
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    // use crate::test::{add_random_email_account, app, create_tenant, pool};
    // use crate::ScopeRepository;
    // use axum::Router;
    // use rstest::rstest;
    // use tracing_test::traced_test;

    // #[rstest]
    // #[tokio::test]
    // #[traced_test]
    // async fn test_add_account(#[future] pool: DbPool) {
    //     let pool = pool.await;

    //     let account_inserted = add_random_email_account(&pool).await;

    //     assert!(account_inserted != Uuid::nil());
    // }

    // #[rstest]
    // #[tokio::test]
    // #[traced_test]
    // async fn test_auth_token_successful(#[future] pool: DbPool) {
    //     let pool = pool.await;
    //     let api_auth_repository = ApiAuthRepository { pool: &pool };
    //     let account = add_random_email_account(&pool).await;
    //     let auth = ApiAuth::create_new(account, &api_auth_repository).await;
    //     let token = auth.unwrap().token;
    //     assert!(!token.is_empty());

    //     let auth = ApiAuth::from(token, &api_auth_repository).await;

    //     assert!(auth.is_ok());
    //     assert_eq!(auth.unwrap().account_id, account);
    // }

    // #[rstest]
    // #[tokio::test]
    // #[traced_test]
    // async fn test_check_auth_token_failed_on_wrong_token(#[future] pool: DbPool) {
    //     let pool = pool.await;
    //     let api_auth_repository = ApiAuthRepository { pool: &pool };

    //     let check_auth = ApiAuth::from("wrong_token".to_string(), &api_auth_repository).await;

    //     assert_eq!(check_auth.is_err(), true);
    // }

    // #[rstest]
    // #[tokio::test]
    // #[traced_test]
    // async fn test_push_auth_token_successful(#[future] app: Router<()>, #[future] pool: DbPool) {
    //     let app = app.await;
    //     let pool = pool.await;
    //     let scope_repository = ScopeRepository { pool: &pool };
    //     let api_auth_repository = ApiAuthRepository { pool: &pool };
    //     let push_api_auth_repository = PushApiAuthRepository { pool: &pool };
    //     let account_id = add_random_email_account(&pool).await;
    //     let auth_token = ApiAuth::create_new(account_id, &api_auth_repository).await;
    //     let tenant_id =
    //         create_tenant("test-tenant".to_string(), auth_token.unwrap().token, &app).await;
    //     let tenant_id = tenant_id.id.unwrap();
    //     let storage_credential_id = scope_repository
    //         .create_storage_credential(
    //             tenant_id,
    //             "clickhouse".to_string(),
    //             "clickhouse://...".to_string(),
    //             account_id,
    //         )
    //         .await;
    //     let storage_credential_id = storage_credential_id.unwrap();
    //     let environment_id = scope_repository
    //         .create_environment(
    //             storage_credential_id,
    //             "testptile".to_string(),
    //             "testslug".to_string(),
    //             account_id,
    //         )
    //         .await;
    //     let environment_id = environment_id.unwrap();
    //     let auth =
    //         PushApiAuth::create_new(environment_id, &push_api_auth_repository, account_id).await;
    //     let token = auth.unwrap().token;
    //     assert!(!token.is_empty());

    //     let auth = PushApiAuth::from(token, &push_api_auth_repository).await;

    //     assert!(auth.is_ok());
    //     let auth_struct = auth.unwrap();
    //     assert_eq!(auth_struct.environment_id, environment_id);
    // }
}
