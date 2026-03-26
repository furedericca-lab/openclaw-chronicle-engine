use clap::Parser;
use chronicle_engine_rs::{build_app, config::AppConfig};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "chronicle-engine-rs")]
struct Args {
    #[arg(long, default_value = "/etc/chronicle-engine-backend/backend.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = AppConfig::load(&args.config)?;
    let bind = config.server.bind.clone();
    let app = build_app(config)?;

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    println!("chronicle-engine-rs listening on {bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
