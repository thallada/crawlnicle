use axum::response::Redirect;

use crate::models::user::AuthContext;

pub async fn get(mut auth: AuthContext) -> Redirect {
    auth.logout().await;
    Redirect::to("/")
}
