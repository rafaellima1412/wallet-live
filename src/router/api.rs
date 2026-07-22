use axum::{Json, Router, routing::get};
use serde::Deserialize;

use crate::{
    app::AppState, auth::admin::Admin, error::AppError, model::Asset, repository::Repository,
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/assets",
        get(list_assets).post(create_assets).patch(update_asset),
    )
}

//#[axum::debug_handler] //valida a assinatura da função
#[tracing::instrument(skip_all)] //--adiciona instrumentação para logs/tracing.
async fn list_assets(repository: Repository) -> Result<Json<Vec<Asset>>, AppError> {
    let assets = repository.list_assets().await?;
    Ok(Json(assets)) //tem que ser clonavel
}
#[derive(Deserialize)]
pub struct CreateAssetRequest {
    pub name: String,
    pub unit_value: f64,
    #[serde(default)]
    pub coingecko_id: Option<String>,
}

#[tracing::instrument(skip_all)] //--adiciona instrumentação para logs/tracing.
async fn create_assets(
    _: Admin,
    repository: Repository,
    Json(request): Json<CreateAssetRequest>,
) -> Result<Json<Asset>, AppError> {
    let new_asset = repository
        .create_asset(request.name, request.unit_value, None)
        .await?;

    Ok(Json(new_asset))
}
#[derive(Deserialize)]
struct UpdateAssetRequest {
    id: i64,
    name: Option<String>,
    unit_value: Option<f64>,
    #[serde(default)]
    coingecko_id: Option<String>,
}
#[tracing::instrument(skip_all)] //--adiciona instrumentação para logs/tracing.
async fn update_asset(
    _: Admin,
    repository: Repository,
    Json(request): Json<UpdateAssetRequest>,
) -> Result<Json<Asset>, AppError> {
    match repository
        .update_asset(
            request.id,
            request.name,
            request.unit_value,
            request.coingecko_id,
        )
        .await?
    {
        Some(update_asset) => Ok(Json(update_asset)),
        None => Err(AppError::AssetDoesNotExist),
    }
}
