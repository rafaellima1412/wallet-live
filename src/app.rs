use crate::{
    quote::CoinGeckoClient,
    router::{self, api},
};
use axum::Router;
use sqlx::PgPool;

use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{
    Layer, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

// #[derive(Clone)]
// pub struct AppConfig {
//     pub admin_secret_key: String,
//     pub jwt_secret_key: String,
// }
#[derive(Clone)] //para a struct
pub struct AppState {
    pub db: PgPool,
    pub quotes: CoinGeckoClient,
    pub admin_secret_key: String,
    pub jwt_secret_key: String,
}

impl AppState {
    async fn new() -> color_eyre::Result<Self> {
        // usar ele o color para tudo que start na a aplicação para exibir logo algo
        let database_url = std::env::var("DATABASE_URL")?;
        let admin_secret_key = std::env::var("ADMIN_SECRET_KEY")?;
        let jwt_secret_key = std::env::var("SECRET_KEY")?;
        let db = PgPool::connect(&database_url).await?;
        sqlx::migrate!("./migrations").run(&db).await?;
        let quotes = CoinGeckoClient::new();
        Ok(Self {
            db,
            quotes,
            admin_secret_key,
            jwt_secret_key,
        })
    }
}
pub struct App;

impl App {
    pub async fn start() -> color_eyre::Result<()> {
        let layer = tracing_subscriber::fmt::layer()
            .with_span_events(FmtSpan::NEW)
            .boxed();
        tracing_subscriber::registry().with(layer).init();
        dotenvy::dotenv()?;
        let state = AppState::new().await?;
        let listener = TcpListener::bind("0.0.0.0:3000").await?;
        let router = Router::new()
            .nest("/api", api::router())
            .merge(router::frontend::router())
            .with_state(state);

        info!("start service");
        axum::serve(listener, router).await?;
        Ok(())
    }
}
