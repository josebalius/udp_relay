mod deployment;
mod server;

use anyhow::{Context, Result};
use server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    match std::env::var("API_KEY") {
        Ok(api_key) => {
            let deployment = get_deployment().context("Failed to get deployment")?;
            let addr = get_addr(deployment).context("Failed to get addr")?;

            println!("Running server on addr: {} ...", addr);
            Server::new(api_key, addr).listen().await
        }
        _ => {
            println!("API_KEY environment variable not set");
            Ok(())
        }
    }
}

fn get_addr(deployment_env: deployment::Environment) -> Result<String> {
    let port = std::env::var("PORT").context("Failed to get PORT")?;
    let addr = match deployment_env {
        deployment::Environment::Production => format!("fly-global-services:{}", port),
        _ => format!("0.0.0.0:{}", port),
    };
    Ok(addr)
}

fn get_deployment() -> Result<deployment::Environment> {
    let deployment_env = std::env::var("DEPLOYMENT").context("Failed to get DEPLOYMENT env var")?;
    deployment_env.parse::<deployment::Environment>()
}
