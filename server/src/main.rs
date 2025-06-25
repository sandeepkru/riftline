use clap::Parser;
use server::serve;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Address to bind the gRPC server
    #[arg(long, env = "LISTEN_ADDR", default_value = "127.0.0.1:50051")]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_from_env("RIFTLINE_LOG")
        .or_else(|_| EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .json()
        .flatten_event(true)
        .with_target(false)
        .init();

    let args = Args::parse();
    let addr: SocketAddr = args.addr.parse()?;
    info!(module = module_path!(), %addr, "Listening on {addr}");
    serve(addr, async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler")
    })
    .await?;
    Ok(())
}
