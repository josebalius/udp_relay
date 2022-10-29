mod client;
mod server;

use std::env;

use client::Client;
use server::Server;
use tokio::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    match args.get(1) {
        Some(arg) => match arg.as_str() {
            "client" => run_client().await,
            _ => run_server().await,
        },
        _ => run_server().await,
    }
}

async fn run_server() -> io::Result<()> {
    println!("Running server...");

    let addr = "0.0.0.0:8080".to_string();
    Server::new(addr).listen().await
}

async fn run_client() -> io::Result<()> {
    println!("Running client...");

    let key = "cyz";

    let remote_addr = "0.0.0.0:8080";
    let local_addr = "0.0.0.0:8081";

    let client = Client::new(
        remote_addr.to_string(),
        local_addr.to_string(),
        key.to_string(),
    );

    client.connect_and_relay().await
}
