mod client;
mod server;

use std::env;

use client::Client;
use server::Server;
use tokio::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let default_cmd = "server".to_string();

    let mode = args.get(1).or(Some(&default_cmd));
    match mode {
        Some(m) => match m.as_str() {
            "server" => return run_server().await,
            "client" => return run_client().await,
            _ => {}
        },
        None => {}
    }

    return Err(io::Error::new(
        io::ErrorKind::Other,
        "Invalid mode. Must be 'server' or 'client'",
    ));
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
