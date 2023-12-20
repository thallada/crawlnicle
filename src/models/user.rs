use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::{Executor, FromRow, Postgres};
use uuid::Uuid;
use validator::Validate;

use crate::auth::generate_hash;
use crate::error::{Error, Result};

#[derive(Debug, Default, Clone, FromRow)]
pub struct User {
    pub user_id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub password_hash: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Default, Validate)]
pub struct CreateUser {
    #[validate(email(message = "email must be a valid email address"))]
    pub email: String,
    #[validate(length(
        min = 8,
        max = 255,
        message = "password must be between 8 and 255 characters long"
    ))]
    pub password: String,
    #[validate(length(max = 255, message = "name must be less than 255 characters long"))]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Default, Validate)]
pub struct UpdateUserPassword {
    #[validate(length(
        min = 8,
        max = 255,
        message = "password must be between 8 and 255 characters long"
    ))]
    pub password: String,
}

impl User {
    pub async fn get(db: impl Executor<'_, Database = Postgres>, user_id: Uuid) -> Result<User> {
        sqlx::query_as!(
            User,
            r#"select
                *
            from users
            where user_id = $1"#,
            user_id
        )
        .fetch_one(db)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::RowNotFound = error {
                return Error::NotFoundUuid("user", user_id);
            }
            Error::Sqlx(error)
        })
    }

    pub async fn get_by_email(
        db: impl Executor<'_, Database = Postgres>,
        email: String,
    ) -> Result<User> {
        sqlx::query_as!(
            User,
            r#"select
                *
            from users
            where email = $1"#,
            email
        )
        .fetch_one(db)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::RowNotFound = error {
                return Error::NotFoundString("user", email);
            }
            Error::Sqlx(error)
        })
    }

    pub async fn create(
        db: impl Executor<'_, Database = Postgres>,
        payload: CreateUser,
    ) -> Result<User> {
        payload.validate()?;
        let password_hash = generate_hash(payload.password).await?;

        Ok(sqlx::query_as!(
            User,
            r#"insert into users (
                email, password_hash, name
            ) values (
                $1, $2, $3
            ) returning
                user_id,
                email,
                email_verified,
                password_hash,
                name,
                created_at,
                updated_at,
                deleted_at
            "#,
            payload.email,
            password_hash,
            payload.name
        )
        .fetch_one(db)
        .await?)
    }

    pub async fn verify_email(
        db: impl Executor<'_, Database = Postgres>,
        user_id: Uuid,
    ) -> Result<User> {
        sqlx::query_as!(
            User,
            r#"update users set
                email_verified = true
            where user_id = $1
            returning *
            "#,
            user_id
        )
        .fetch_one(db)
        .await
        .map_err(|error| {
            if let sqlx::error::Error::RowNotFound = error {
                return Error::NotFoundUuid("user", user_id);
            }
            Error::Sqlx(error)
        })
    }

    pub async fn update_password(
        &self,
        db: impl Executor<'_, Database = Postgres>,
        payload: UpdateUserPassword,
    ) -> Result<User> {
        payload.validate()?;
        let password_hash = generate_hash(payload.password).await?;

        Ok(sqlx::query_as!(
            User,
            r#"update users set
                password_hash = $2
            where
                user_id = $1
            returning
                user_id,
                email,
                email_verified,
                password_hash,
                name,
                created_at,
                updated_at,
                deleted_at
            "#,
            self.user_id,
            password_hash,
        )
        .fetch_one(db)
        .await?)
    }
}
