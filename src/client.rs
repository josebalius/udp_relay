use std::sync::Arc;

use tokio::{
    io,
    net::UdpSocket,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

use crate::server;

pub struct Client {
    addr: String,
    peer_addr: String,
    key: String,
    local_mode: bool,

    sender: Sender<server::DataPacket>,
    receiver: Mutex<Receiver<server::DataPacket>>,
}

impl Client {
    pub fn new(
        addr: String,
        peer_addr: String,
        key: String,
        local_mode: bool,
        sender: Sender<server::DataPacket>,
        receiver: Receiver<server::DataPacket>,
    ) -> Self {
        Self {
            addr,
            peer_addr,
            key,
            local_mode,
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    pub async fn connect(&self) -> io::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let r = Arc::new(socket);
        let s = r.clone();

        let (receive, forward) = tokio::join!(self.receive(r), self.forward(s));
        receive.and(forward)
    }

    async fn receive(&self, socket: Arc<UdpSocket>) -> io::Result<()> {
        if !self.local_mode {
            self.connect_to_relay(&socket).await?;
        }

        let mut buf = [0; server::MAX_PACKET_LEN];
        loop {
            let (n, addr) = socket.recv_from(&mut buf).await?;
            println!("Received {} bytes from {}", n, addr);

            let data = std::str::from_utf8(&buf).unwrap();
            println!("Data: {:?}", data);

            match self
                .sender
                .send(server::DataPacket {
                    key: self.key.clone(),
                    data: buf[..n].to_vec(),
                })
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    println!("Error sending packet to server: {}", e);
                }
            }
        }
    }

    async fn connect_to_relay(&self, socket: &Arc<UdpSocket>) -> io::Result<()> {
        let data = format!("CONNECT {}", self.key);
        socket.send_to(data.as_bytes(), &self.peer_addr).await?;

        Ok(())
    }

    async fn forward(&self, socket: Arc<UdpSocket>) -> io::Result<()> {
        while let Some(packet) = self.receiver.lock().await.recv().await {
            self.forward_packet(packet, &socket).await?;
        }

        Ok(())
    }

    async fn forward_packet(
        &self,
        packet: server::DataPacket,
        socket: &Arc<UdpSocket>,
    ) -> io::Result<()> {
        socket.send_to(&packet.data, &self.peer_addr).await?;
        Ok(())
    }
}
