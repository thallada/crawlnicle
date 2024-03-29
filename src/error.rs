use axum::extract::multipart::MultipartError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_with::DisplayFromStr;
use tracing::error;
use uuid::Uuid;
use validator::ValidationErrors;

/// An API-friendly error type.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A SQLx call returned an error.
    ///
    /// The exact error contents are not reported to the user in order to avoid leaking
    /// information about database internals.
    #[error("an internal database error occurred")]
    Sqlx(#[from] sqlx::Error),

    /// Similarly, we don't want to report random `anyhow` errors to the user.
    #[error("an internal server error occurred")]
    Anyhow(#[from] anyhow::Error),

    #[error("an internal server error occurred")]
    Reqwest(#[from] reqwest::Error),

    #[error("validation error in request body")]
    InvalidEntity(#[from] ValidationErrors),

    #[error("error with file upload: (0)")]
    Upload(#[from] MultipartError),

    #[error("no file uploaded")]
    NoFile,

    #[error("{0}: {1} not found")]
    NotFoundUuid(&'static str, Uuid),

    #[error("{0}: {1} not found")]
    NotFoundString(&'static str, String),

    #[error("referenced {0} not found")]
    RelationNotFound(&'static str),

    #[error("an internal server error occurred")]
    InternalServerError,

    #[error("unauthorized")]
    Unauthorized,

    #[error("bad request: {0}")]
    BadRequest(&'static str)
}

pub type Result<T, E = Error> = ::std::result::Result<T, E>;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        #[serde_with::serde_as]
        #[serde_with::skip_serializing_none]
        #[derive(serde::Serialize)]
        struct ErrorResponse<'a> {
            // Serialize the `Display` output as the error message
            #[serde_as(as = "DisplayFromStr")]
            message: &'a Error,

            errors: Option<&'a ValidationErrors>,
        }

        let errors = match &self {
            Error::InvalidEntity(errors) => Some(errors),
            _ => None,
        };

        error!("API error: {:?}", self);

        (
            self.status_code(),
            Json(ErrorResponse {
                message: &self,
                errors,
            }),
        )
            .into_response()
    }
}

impl Error {
    fn status_code(&self) -> StatusCode {
        use Error::*;

        match self {
            NotFoundUuid(_, _) | NotFoundString(_, _) => StatusCode::NOT_FOUND,
            Unauthorized => StatusCode::UNAUTHORIZED,
            BadRequest(_) => StatusCode::BAD_REQUEST,
            InternalServerError | Sqlx(_) | Anyhow(_) | Reqwest(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            InvalidEntity(_) | RelationNotFound(_) | NoFile => StatusCode::UNPROCESSABLE_ENTITY,
            Upload(err) => err.status(),
        }
    }
}
