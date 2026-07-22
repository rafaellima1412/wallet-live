use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::FromRequestParts;

use crate::{app::AppState, error::AppError};

const COINGECKO_BASE_URL: &str = "https://api.coingecko.com/api/v3";

#[derive(Clone)]
pub struct CoinGeckoClient {
    http: reqwest::Client,
}

impl CoinGeckoClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("wallet-live/0.1 (+https://github.com/rafaellima1412/wallet-live)")
            .build()
            .expect("o client HTTP deveria ser construído com configurações válidas");

        Self { http }
    }

    pub async fn fetch_prices(
        &self,
        ids: &[String],
        vs_currency: &str,
    ) -> Result<HashMap<String, f64>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        let joined_ids = ids.join(",");
        let response = self
            .http
            .get(format!("{COINGECKO_BASE_URL}/simple/price"))
            .query(&[("ids", joined_ids.as_str()), ("vs_currencies", vs_currency)])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!(
                %status,
                %body,
                ids = %joined_ids,
                "CoinGecko retornou um erro ao buscar cotações"
            );
            return Err(AppError::CoinGeckoRequestFailed(status.as_u16()));
        }

        let response = response
            .json::<HashMap<String, HashMap<String, f64>>>()
            .await?;

        let prices = response
            .into_iter()
            .filter_map(|(id, currencies_by_price)| {
                currencies_by_price
                    .get(vs_currency)
                    .copied()
                    .map(|price| (id, price))
            })
            .collect();

        Ok(prices)
    }
}

impl Default for CoinGeckoClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FromRequestParts<AppState> for CoinGeckoClient {
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(state.quotes.clone())
    }
}
