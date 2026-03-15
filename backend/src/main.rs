use clap::Parser;
use memory_lancedb_pro_backend::{build_app, config::AppConfig};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "memory-lancedb-pro-backend")]
struct Args {
    #[arg(long, default_value = "/etc/memory-lancedb-pro-backend/backend.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = AppConfig::load(&args.config)?;
    let bind = config.server.bind.clone();
    let app = build_app(config)?;

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    println!("memory-lancedb-pro-backend listening on {bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
