use clap::Parser;
use server::serve;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Address to bind the gRPC server
    #[arg(long, env = "LISTEN_ADDR", default_value = "127.0.0.1:50051")]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    let addr: SocketAddr = args.addr.parse()?;
    println!("Listening on {}", addr);
    serve(addr, async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler")
    })
    .await?;
    Ok(())
}
