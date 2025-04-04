use std::sync::Arc;

use axum::{Router, routing::post};
use grhooks_config::Config;
use tokio::sync::RwLock;
use tracing::level_filters::LevelFilter;

mod handlers;
mod validator;

pub(crate) type GlobalConfig = Arc<RwLock<Config>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_file(true)
        .with_line_number(true)
        .init();

    let config = grhooks_config::get_config();
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();
    let state = Arc::new(RwLock::new(config));

    let app = Router::new()
        .route("/{*path}", post(handlers::webhook_handler))
        .layer(axum::middleware::from_fn(validator::validate_headers))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            validator::validate_signature_middleware,
        ))
        .with_state(state);

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap()
}
