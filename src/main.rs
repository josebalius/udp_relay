mod server;

use server::Server;
use tokio::io;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> io::Result<Result<(), io::Error>> {
    let (client_sender, client_receiver) = mpsc::channel(32);
    let (relay_sender, relay_receiver) = mpsc::channel(32);

    let client_addr = "0.0.0.0:8080".to_string();
    let mut client_server = Server::new(client_addr, relay_sender, client_receiver);
    let client_listen = client_server.listen();

    let relay_addr = "0.0.0.0:8081".to_string();
    let mut relay_server = Server::new(relay_addr, client_sender, relay_receiver);
    let relay_listen = relay_server.listen();

    let (client, relay) = tokio::join!(client_listen, relay_listen);

    Ok(client.and(relay))
}
