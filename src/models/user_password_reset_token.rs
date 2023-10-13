use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPasswordResetToken {
    pub token_id: Uuid,
    pub user_id: Uuid,
    pub request_user_agent: Option<String>,
    pub request_ip: IpNetwork,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePasswordResetToken {
    pub token_id: Uuid,
    pub user_id: Uuid,
    pub request_user_agent: Option<String>,
    pub request_ip: IpNetwork,
    pub expires_at: DateTime<Utc>,
}

impl UserPasswordResetToken {
    pub fn expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub async fn get(
        pool: impl Executor<'_, Database = Postgres>,
        token_id: Uuid,
    ) -> Result<UserPasswordResetToken> {
        sqlx::query_as!(
            UserPasswordResetToken,
            r#"select
                *
            from user_password_reset_token
            where token_id = $1"#,
            token_id
        )
        .fetch_one(pool)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::RowNotFound = error {
                return Error::NotFoundUuid("user_password_reset_token", token_id);
            }
            Error::Sqlx(error)
        })
    }

    pub async fn create(
        pool: impl Executor<'_, Database = Postgres>,
        payload: CreatePasswordResetToken,
    ) -> Result<UserPasswordResetToken> {
        Ok(sqlx::query_as!(
            UserPasswordResetToken,
            r#"insert into user_password_reset_token (
                token_id, user_id, request_user_agent, request_ip, expires_at
            ) values (
                $1, $2, $3, $4, $5
            ) returning *"#,
            payload.token_id,
            payload.user_id,
            payload.request_user_agent,
            payload.request_ip,
            payload.expires_at
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn delete(
        pool: impl Executor<'_, Database = Postgres>,
        token_id: Uuid,
    ) -> Result<()> {
        sqlx::query!(
            r#"delete from user_password_reset_token
            where token_id = $1"#,
            token_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
