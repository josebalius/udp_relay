use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Server(Server),
    Client(Client),
}

#[derive(Args)]
pub struct Server {
    #[clap(long, default_value = "0.0.0.0:8080")]
    pub addr: String,
}

#[derive(Args)]
pub struct Client {
    #[clap(long, default_value = "0.0.0.0:8080")]
    pub remote_addr: String,

    #[clap(long, default_value = "0.0.0.0:8081")]
    pub local_addr: String,

    #[clap(long, default_value = "cyz")]
    pub key: String,
}
