use crate::router::{self, api};
use axum::Router;
use sqlx::PgPool;

use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{
    Layer, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

#[derive(Clone)] //para a struct
pub struct AppState {
    pub db: PgPool,
}

impl AppState {
    async fn new() -> color_eyre::Result<Self> {
        // usar ele o color para tudo que start na a aplicação para exibir logo algo
        let database_url = std::env::var("DATABASE_URL")?;
        let db = PgPool::connect(&database_url).await?;
        Ok(Self { db })
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
