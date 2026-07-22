use axum::{extract::FromRequestParts, http::header::AUTHORIZATION};

use crate::app::AppState;

pub struct Admin;

impl FromRequestParts<AppState> for Admin {
    type Rejection = &'static str;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(auth) = parts.headers.get(AUTHORIZATION) else {
            return Err("Missing Authorization Header");
        };
        let auth = auth.to_str().map_err(|_| "Invalid Authorization Header")?;

        if auth == state.admin_secret_key {
            Ok(Admin)
        } else {
            Err("Invalid Credential")
        }
    }
}
