mod cli;
mod client;
mod server;

use clap::Parser;
use cli::Cli;
use client::Client;
use server::Server;
use tokio::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(cli::Commands::Client(client)) => run_client(client).await,
        Some(cli::Commands::Server(server)) => run_server(server).await,
        None => {
            // TODO: print help
            println!("A command is required, see --help");
            Ok(())
        }
    }
}

async fn run_server(server: cli::Server) -> io::Result<()> {
    println!("Running server...");
    Server::new(server.addr.to_string()).listen().await
}

async fn run_client(client: cli::Client) -> io::Result<()> {
    println!("Running client...");

    let client = Client::new(
        client.remote_addr.to_string(),
        client.local_addr.to_string(),
        client.key.to_string(),
    );

    client.connect_and_relay().await
}
