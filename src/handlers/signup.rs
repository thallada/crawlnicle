use axum::response::{IntoResponse, Redirect, Response};
use axum::{extract::State, Form};
use maud::html;
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;

use crate::error::{Error, Result};
use crate::models::user::{AuthContext, CreateUser, User};
use crate::partials::layout::Layout;
use crate::partials::signup_form::{signup_form, SignupFormProps};

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Signup {
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
    #[serde_as(as = "NoneAsEmptyString")]
    pub name: Option<String>,
}

pub async fn get(layout: Layout) -> Result<Response> {
    Ok(layout.with_subtitle("signup").render(html! {
        header {
            h2 { "Signup" }
        }
        (signup_form(SignupFormProps::default()))
    }))
}

pub async fn post(
    State(pool): State<PgPool>,
    mut auth: AuthContext,
    Form(signup): Form<Signup>,
) -> Result<Response> {
    if signup.password != signup.password_confirmation {
        // return Err(Error::BadRequest("passwords do not match"));
        return Ok(signup_form(SignupFormProps {
            email: Some(signup.email),
            name: signup.name,
            password_error: Some("passwords do not match".to_string()),
            ..Default::default()
        })
        .into_response());
    }
    let user = match User::create(
        &pool,
        CreateUser {
            email: signup.email.clone(),
            password: signup.password.clone(),
            name: signup.name.clone(),
        },
    )
    .await
    {
        Ok(user) => user,
        Err(err) => {
            if let Error::InvalidEntity(validation_errors) = err {
                let field_errors = validation_errors.field_errors();
                dbg!(&validation_errors);
                dbg!(&field_errors);
                return Ok(signup_form(SignupFormProps {
                    email: Some(signup.email),
                    name: signup.name,
                    email_error: field_errors.get("email").map(|&errors| {
                        errors
                            .iter()
                            .filter_map(|error| error.message.clone().map(|m| m.to_string()))
                            .collect::<Vec<String>>()
                            .join(", ")
                    }),
                    name_error: field_errors.get("name").map(|&errors| {
                        errors
                            .iter()
                            .filter_map(|error| error.message.clone().map(|m| m.to_string()))
                            .collect::<Vec<String>>()
                            .join(", ")
                    }),
                    password_error: field_errors.get("password").map(|&errors| {
                        errors
                            .iter()
                            .filter_map(|error| error.message.clone().map(|m| m.to_string()))
                            .collect::<Vec<String>>()
                            .join(", ")
                    }),
                    ..Default::default()
                })
                .into_response());
            }
            if let Error::Sqlx(sqlx::error::Error::Database(db_error)) = &err {
                if let Some(constraint) = db_error.constraint() {
                    if constraint == "users_email_idx" {
                        return Ok(signup_form(SignupFormProps {
                            email: Some(signup.email),
                            name: signup.name,
                            email_error: Some("email already exists".to_string()),
                            ..Default::default()
                        })
                        .into_response());
                    }
                }
            }
            return Err(err);
        }
    };
    auth.login(&user)
        .await
        .map_err(|_| Error::InternalServerError)?;
    Ok(Redirect::to("/").into_response())
}
