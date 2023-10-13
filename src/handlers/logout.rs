use crate::{models::user::AuthContext, htmx::HXRedirect};

pub async fn get(mut auth: AuthContext) -> HXRedirect {
    auth.logout().await;
    HXRedirect::to("/").reload(true)
}
