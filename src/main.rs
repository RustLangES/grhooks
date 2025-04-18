use std::str::FromStr;
use std::sync::Arc;

use axum::{Router, routing::post};
use grhooks_config::Config;
use notify::event::{DataChange, ModifyKind};
use notify::{EventHandler, EventKind, Watcher};
use tokio::sync::RwLock;
use tracing::level_filters::LevelFilter;

mod errors;
mod handlers;
mod validator;

pub(crate) type GlobalConfig = Arc<RwLock<Config>>;

#[tokio::main]
async fn main() {
    let (config_path, config) = grhooks_config::get_config();
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::from_str(&config.verbose).unwrap_or(LevelFilter::INFO))
        .with_file(true)
        .with_line_number(true)
        .init();
    config.print_paths();

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();
    let state = Arc::new(RwLock::new(config));

    let mut manifest_watcher = notify::recommended_watcher(listen_config_changes(state.clone()))
        .expect("Cannot create watcher for manifest");
    manifest_watcher
        .watch(config_path.as_path(), notify::RecursiveMode::Recursive)
        .unwrap();

    let app = Router::new()
        .route("/{*path}", post(handlers::webhook_handler))
        .layer(axum::middleware::from_fn(validator::validate_headers))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            validator::validate_signature_middleware,
        ))
        .with_state(state);

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

fn listen_config_changes(state: GlobalConfig) -> impl EventHandler {
    move |res: notify::Result<notify::Event>| {
        let Ok(event) = res else {
            tracing::error!("Error watching config file: {}", res.unwrap_err());
            return;
        };
        if event.kind == EventKind::Modify(ModifyKind::Data(DataChange::Any)) {
            let mut config = state.blocking_write();
            config.webhooks = Vec::new(); // restore webhooks
            event.paths.iter().for_each(|path| {
                tracing::info!("Config file changed: {path:?}");
                config.merge(grhooks_config::parse_config(path));
            });
            config.print_paths();
        }
    }
}
