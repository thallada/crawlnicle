use axum::response::IntoResponse;

use crate::auth::AuthSession;

pub async fn get(auth: AuthSession) -> impl IntoResponse {
    match auth.user {
        Some(user) => {
            format!(
                "Logged in as: {}",
                user.name.unwrap_or_else(|| "No name".to_string())
            )
        }
        None => "Not logged in".to_string(),
    }
}
