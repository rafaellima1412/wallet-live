use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Missing Authorization Headers")]
    MissingAuthorization,
    #[error("Invalid Credentials")]
    InvalidCredentials,
    #[error("Asset Does Not Exist")]
    AssetDoesNotExist,
    #[error("User Name is already registered")]
    UserNameToken,
    #[error("User Does Not Exist")]
    UserDoesNotExist,
    #[error("transparent")]
    Database(#[from] sqlx::Error),
    #[error("transparent")]
    Template(#[from] askama::Error),
    #[error("transparent")]
    JwT(#[from] jwt_simple::Error),
}
#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let error_reponse = ErrorResponse {
            error: self.to_string(),
        };
        let status = match self {
            Self::UserNameToken | Self::MissingAuthorization => StatusCode::BAD_REQUEST,
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::AssetDoesNotExist | Self::UserDoesNotExist => StatusCode::NOT_FOUND,
            Self::Database(_) | Self::Template(_) | Self::JwT(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        (status, Json(error_reponse)).into_response()
    }
}
