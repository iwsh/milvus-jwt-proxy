mod auth;
mod config;
mod proxy;

use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Milvus JWT Proxy...");

    // Load configuration
    let config = config::Config::from_env();
    info!("Configuration loaded: {:?}", config);

    // Create and run proxy
    let proxy = proxy::Proxy::new(config);
    proxy.run().await?;

    Ok(())
}
