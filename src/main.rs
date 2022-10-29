mod client;
mod server;

use std::env;

use client::Client;
use server::Server;
use tokio::io;
use tokio::sync::mpsc;

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

    let (client_sender, client_receiver) = mpsc::channel(32);
    let (relay_sender, relay_receiver) = mpsc::channel(32);

    let client_addr = "0.0.0.0:8080".to_string();
    let client_server = Server::new(client_addr, relay_sender, client_receiver);
    let client_listen = client_server.listen();

    let relay_addr = "0.0.0.0:8081".to_string();
    let relay_server = Server::new(relay_addr, client_sender, relay_receiver);
    let relay_listen = relay_server.listen();

    let (client, relay) = tokio::join!(client_listen, relay_listen);

    client.and(relay)
}

async fn run_client() -> io::Result<()> {
    println!("Running client...");

    let key = "cyz";

    let (remote_sender, remote_receiver) = mpsc::channel(32);
    let (local_sender, local_receiver) = mpsc::channel(32);

    let remote_addr = "0.0.0.0:8080";
    let local_addr = "0.0.0.0:8082";

    let remote_client = Client::new(
        remote_addr.to_string(),
        local_addr.to_string(),
        key.to_string(),
        false,
        local_sender,
        remote_receiver,
    );
    let remote_connect = remote_client.connect();

    let local_client = Client::new(
        local_addr.to_string(),
        "0.0.0.0:8083".to_string(),
        key.to_string(),
        true,
        remote_sender,
        local_receiver,
    );
    let local_connect = local_client.connect();

    let (remote, local) = tokio::join!(remote_connect, local_connect);

    remote.and(local)
}
