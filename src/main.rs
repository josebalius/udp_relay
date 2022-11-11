mod server;

use anyhow::{Context, Result};
use server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    match std::env::var("API_KEY") {
        Ok(api_key) => {
            let port = std::env::var("PORT").context("Failed to get PORT")?;
            let addr = format!("0.0.0.0:{}", port);

            println!("Running server on addr: {} ...", addr);
            Server::new(api_key, addr).listen().await
        }
        _ => {
            println!("API_KEY environment variable not set");
            Ok(())
        }
    }
}
