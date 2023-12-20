use anyhow::Context;
use async_trait::async_trait;
use axum_login::{AuthUser, AuthnBackend, UserId};
use password_auth;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::Result, models::user::User};

pub async fn generate_hash(password: String) -> Result<String> {
    // Argon2 hashing is designed to be computationally intensive,
    tokio::task::spawn_blocking(move || -> String { password_auth::generate_hash(password) })
        .await
        .context("panic in generating password hash")
        .map_err(|e| e.into())
}

pub async fn verify_password(password: String, password_hash: String) -> Result<()> {
    tokio::task::spawn_blocking(move || -> Result<()> {
        password_auth::verify_password(password.as_bytes(), &password_hash)
            .map_err(|e| anyhow::anyhow!("failed to verify password hash: {}", e).into())
    })
    .await
    .context("panic in verifying password hash")?
}

impl AuthUser for User {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.user_id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct Backend {
    db: PgPool,
}

impl Backend {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = sqlx::Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = User::get_by_email(&self.db, creds.email).await.ok();

        if let Some(user) = user {
            if verify_password(creds.password, user.password_hash.clone())
                .await
                .ok()
                .is_some()
            {
                return Ok(Some(user));
            }
        }
        Ok(None)
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        sqlx::query_as!(
            User,
            r#"select
                *
            from users
            where user_id = $1"#,
            user_id
        )
        .fetch_optional(&self.db)
        .await
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;
