use anyhow::Context;
use axum::response::{IntoResponse, Response};

use crate::auth::AuthSession;
use crate::error::Result;
use crate::htmx::HXRedirect;

pub async fn get(mut auth: AuthSession) -> Result<Response> {
    auth.logout()
        .context("failed to logout user from session")?;
    Ok(HXRedirect::to("/").reload(true).into_response())
}
