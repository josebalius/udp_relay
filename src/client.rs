use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::{
    io,
    net::UdpSocket,
    sync::mpsc::{Receiver, Sender},
};

use crate::server;

type Packet = Vec<u8>;

pub struct Client {
    remote_addr: String,
    local_addr: String,
    key: String,
}

impl Client {
    pub fn new(remote_addr: String, local_addr: String, key: String) -> Self {
        Self {
            remote_addr,
            local_addr,
            key,
        }
    }

    pub async fn connect_and_relay(&self) -> io::Result<()> {
        let (remote_sender, remote_receiver) = mpsc::channel(32);
        let (local_sender, local_receiver) = mpsc::channel(32);
        let (remote, local) = tokio::join!(
            self.connect_and_relay_socket(&self.remote_addr, true, local_receiver, remote_sender),
            self.connect_and_relay_socket(&self.local_addr, false, remote_receiver, local_sender)
        );
        remote.and(local)
    }

    async fn connect_and_relay_socket(
        &self,
        addr: &String,
        send_connect: bool,
        receiver: Receiver<Packet>,
        sender: Sender<Packet>,
    ) -> io::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(addr).await?;

        if send_connect {
            self.send_connect(&socket).await?;
        }

        let r = Arc::new(socket);
        let s = r.clone();

        let (receive, send) = tokio::join!(self.receive(r, sender), self.send(s, receiver));
        receive.and(send)
    }

    async fn send_connect(&self, socket: &UdpSocket) -> io::Result<usize> {
        let data = format!("{} {}", server::CONNECT_STR, self.key);
        socket.send(data.as_bytes()).await
    }

    async fn receive(&self, socket: Arc<UdpSocket>, sender: Sender<Packet>) -> io::Result<()> {
        let mut buf = [0; server::MAX_PACKET_LEN];
        loop {
            let (n, _) = socket.recv_from(&mut buf).await?;
            println!("Received {} bytes from remote", n);

            let packet = buf[..n].to_vec();
            if let Err(_) = sender.send(packet).await {
                println!("Failed to send packet from receive");
            }
        }
    }

    async fn send(&self, socket: Arc<UdpSocket>, mut receiver: Receiver<Packet>) -> io::Result<()> {
        while let Some(packet) = receiver.recv().await {
            println!("Sending {} bytes to remote", packet.len());
            if let Err(_) = socket.send(&packet).await {
                println!("Failed to send packet from send");
            }
        }
        Ok(())
    }
}
