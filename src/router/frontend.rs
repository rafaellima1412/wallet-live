use askama::Template;
use axum::{
    Form, Router,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use serde::Deserialize;
use tokio::try_join;

use crate::{
    app::AppState,
    auth::user::{UnauthenticatedUser, User},
    error::AppError,
    model::{Asset, OwnedAsset},
    quote::CoinGeckoClient,
    repository::Repository,
};
const QUOTES_VS_CURRENCY: &str = "usd";
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/login", get(login_page).post(login))
        .route("/logout", get(logout))
        .route("/assets", get(assets).post(purchase_asset))
        .route("/assets/refresh-prices", post(refresh_prices))
}
#[derive(Template)]
#[template(path = "assets.html")]
pub struct AssetsPage {
    owned_assets: Vec<OwnedAsset>,
    available_assets: Vec<Asset>,
    user: User,
}
pub async fn assets(repository: Repository, user: User) -> Result<Html<String>, AppError> {
    let (owned_assets, available_assets) = try_join!(
        repository.list_owned_assets(user.id()),
        repository.list_assets()
    )?;
    let html = AssetsPage {
        owned_assets,
        available_assets,
        user,
    }
    .render()?;
    Ok(Html(html))
}
#[derive(Deserialize)]
pub struct PurchaseAssetForm {
    asset_id: i64,
    unit_value: f64,
    quantity: f64,
}
pub async fn purchase_asset(
    repository: Repository,
    user: User,
    Form(request): Form<PurchaseAssetForm>,
) -> Result<Redirect, AppError> {
    repository
        .insert_owned_asset(
            user.id,
            request.asset_id,
            request.quantity,
            request.unit_value,
        )
        .await?;
    Ok(Redirect::to("/assets"))
}
#[tracing::instrument(skip_all)]
pub async fn refresh_prices(
    repository: Repository,
    quotes: CoinGeckoClient,
    _user: User,
) -> Result<Redirect, AppError> {
    let assets_to_refresh = repository.list_assets_with_coingecko_id().await?;

    let ids: Vec<String> = assets_to_refresh
        .iter()
        .filter_map(|asset| asset.coingecko_id.clone())
        .collect();

    let prices = quotes.fetch_prices(&ids, QUOTES_VS_CURRENCY).await?;

    for asset in assets_to_refresh {
        let Some(coingecko_id) = asset.coingecko_id.as_deref() else {
            continue;
        };
        if let Some(&price) = prices.get(coingecko_id) {
            repository.update_asset_price(asset.id, price).await?;
        }
    }

    Ok(Redirect::to("/assets"))
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage;

async fn login_page() -> Result<Html<String>, AppError> {
    let hmtl = LoginPage.render()?;
    Ok(Html(hmtl))
}
#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}
async fn login(
    repository: Repository,
    jar: CookieJar,
    Form(request): Form<LoginForm>,
) -> Result<impl IntoResponse, AppError> {
    let unauth_user = UnauthenticatedUser::new(request.username, request.password);
    let user = match unauth_user.authenticate(&repository).await {
        Ok(user) => user,
        Err(AppError::UserDoesNotExist) => unauth_user.register(&repository).await?,
        Err(other_err) => return Err(other_err),
    };
    let token = user.auth_token()?;
    let cookie = Cookie::build(("token", token)).http_only(true);
    Ok((jar.add(cookie), Redirect::to("/")))
}

pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    (jar.remove("token"), Redirect::to("/login"))
}

//#[axum::debug_handler]
async fn index(maybe_user: Option<User>) -> Result<Redirect, AppError> {
    match maybe_user {
        Some(_) => Ok(Redirect::to("/assets")),
        None => Ok(Redirect::to("/login")),
    }
}

pub mod filters {
    use askama;
    use time::{
        OffsetDateTime, format_description::StaticFormatDescription, macros::format_description,
    };
    #[askama::filter_fn]
    pub fn human_datetime(
        datetime: &OffsetDateTime,
        _env: &dyn askama::Values,
    ) -> askama::Result<String> {
        const HUMAN_READABLE_FORMAT: StaticFormatDescription =
            format_description!(version = 2, "[year]-[month]-[day] [hour]:[minute]");
        datetime
            .format(HUMAN_READABLE_FORMAT)
            .map_err(askama::Error::custom)
    }
}
