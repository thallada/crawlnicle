use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserEmailVerificationToken {
    pub token_id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CreateUserEmailVerificationToken {
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

impl UserEmailVerificationToken {
    pub fn expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub async fn get(pool: &PgPool, token_id: Uuid) -> Result<UserEmailVerificationToken> {
        sqlx::query_as!(
            UserEmailVerificationToken,
            r#"select
                *
            from user_email_verification_token
            where token_id = $1"#,
            token_id
        )
        .fetch_one(pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::RowNotFound = error {
                return Error::NotFoundUuid("user_email_verification_token", token_id);
            }
            Error::Sqlx(error)
        })
    }

    pub async fn create(
        pool: &PgPool,
        payload: CreateUserEmailVerificationToken,
    ) -> Result<UserEmailVerificationToken> {
        Ok(sqlx::query_as!(
            UserEmailVerificationToken,
            r#"insert into user_email_verification_token (
                user_id, expires_at
            ) values (
                $1, $2
            ) returning *"#,
            payload.user_id,
            payload.expires_at
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn delete(
        pool: &PgPool,
        token_id: Uuid,
    ) -> Result<()> {
        sqlx::query!(
            r#"delete from user_email_verification_token
            where token_id = $1"#,
            token_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
