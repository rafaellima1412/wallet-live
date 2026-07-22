use crate::app::App;
mod app;
mod auth;
pub mod error;
mod model;
pub mod quote;
pub mod repository;
pub mod router;
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    App::start().await
}
