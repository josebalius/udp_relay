use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::{io, net::UdpSocket};

pub const MAX_PACKET_LEN: usize = 1024;

#[derive(Debug)]
pub struct DataPacket {
    key: String,
    data: Vec<u8>,
}

pub struct Server {
    addr: String,
    sender: Sender<DataPacket>,
    receiver: Receiver<DataPacket>,

    connected_clients: HashMap<SocketAddr, Relay>,
    relay_clients: HashMap<String, Relay>,
}

impl Server {
    pub fn new(addr: String, sender: Sender<DataPacket>, receiver: Receiver<DataPacket>) -> Self {
        Self {
            addr,
            sender,
            receiver,
            connected_clients: HashMap::new(),
            relay_clients: HashMap::new(),
        }
    }

    pub async fn listen(&mut self) -> io::Result<()> {
        let socket = UdpSocket::bind(&self.addr).await?;
        let r = Arc::new(socket);
        let s = r.clone();
        let mut buf = [0; MAX_PACKET_LEN];

        loop {
            tokio::select! {
                result = r.recv_from(&mut buf) => {
                    match result {
                        Ok((len, addr)) => {
                            self.receive_datagram(&buf[..len], addr).await?;
                        }
                        Err(e) => {
                            eprintln!("failed to receive a datagram: {}", e);
                        }
                    }
                }
                Some(packet) = self.receiver.recv() => {
                    self.forward_packet(packet, &s).await?;
                }
            }
        }
    }

    async fn forward_packet(
        &mut self,
        packet: DataPacket,
        socket: &Arc<UdpSocket>,
    ) -> io::Result<()> {
        match self.relay_clients.get(&packet.key) {
            Some(relay) => {
                socket.send_to(&packet.data, &relay.addr).await?;
            }
            _ => {
                println!("No relay found for {}", packet.key);
            }
        }
        Ok(())
    }

    async fn receive_datagram(&mut self, buf: &[u8], addr: SocketAddr) -> io::Result<()> {
        match self.handle_datagram(buf, addr) {
            Some(relay) => {
                let relay_copy = relay.clone();
                self.connected_clients.insert(addr, relay);
                self.relay_clients
                    .insert(relay_copy.key.clone(), relay_copy);
            }
            _ => match self.connected_clients.get(&addr) {
                Some(relay) => {
                    let data = buf.to_vec();
                    relay.send_data(data).await?;
                }
                _ => {
                    println!("No relay found for {}", addr);
                }
            },
        }
        Ok(())
    }

    fn handle_datagram(&mut self, buf: &[u8], addr: SocketAddr) -> Option<Relay> {
        let data = std::str::from_utf8(&buf).unwrap().trim_end_matches("\n");
        let parts = data.split_whitespace().collect::<Vec<&str>>();
        let cmd = parts[0];

        println!("command: {:?}", cmd);
        match cmd {
            "CONNECT" => {
                let key = parts[1];
                let relay = Relay::new(key.to_string(), addr, self.sender.clone());
                Some(relay)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct Relay {
    key: String,
    addr: SocketAddr,
    sender: Sender<DataPacket>,
}

impl Relay {
    fn new(key: String, addr: SocketAddr, sender: Sender<DataPacket>) -> Self {
        Self { key, addr, sender }
    }

    async fn send_data(&self, data: Vec<u8>) -> io::Result<()> {
        match self
            .sender
            .send(DataPacket {
                key: self.key.clone(),
                data,
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
}
