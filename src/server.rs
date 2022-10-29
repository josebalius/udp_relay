use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::{io, net::UdpSocket};

pub const MAX_PACKET_LEN: usize = 1024;

#[derive(Debug)]
pub struct DataPacket {
    pub key: String,
    pub data: Vec<u8>,
}

pub struct Server {
    addr: String,
    sender: Sender<DataPacket>,
    receiver: Mutex<Receiver<DataPacket>>,

    connected_clients: Mutex<HashMap<SocketAddr, Arc<Relay>>>,
    relay_clients: Mutex<HashMap<String, Arc<Relay>>>,
}

impl Server {
    pub fn new(addr: String, sender: Sender<DataPacket>, receiver: Receiver<DataPacket>) -> Self {
        Self {
            addr,
            sender,
            receiver: Mutex::new(receiver),
            connected_clients: Mutex::new(HashMap::new()),
            relay_clients: Mutex::new(HashMap::new()),
        }
    }

    pub async fn listen(&self) -> io::Result<()> {
        let socket = UdpSocket::bind(&self.addr).await?;
        let r = Arc::new(socket);
        let s = r.clone();
        let (receive, forward) = tokio::join!(self.receive(r), self.forward(&s));

        receive.and(forward)
    }

    async fn forward(&self, socket: &Arc<UdpSocket>) -> io::Result<()> {
        while let Some(packet) = self.receiver.lock().await.recv().await {
            self.forward_packet(packet, socket).await?;
        }
        Ok(())
    }

    async fn forward_packet(&self, packet: DataPacket, socket: &Arc<UdpSocket>) -> io::Result<()> {
        match self.relay_clients.lock().await.get(&packet.key) {
            Some(relay) => {
                socket.send_to(&packet.data, &relay.addr).await?;
            }
            _ => {
                println!("No relay found for {}", packet.key);
            }
        }
        Ok(())
    }

    async fn receive(&self, socket: Arc<UdpSocket>) -> io::Result<()> {
        let mut buf = [0; MAX_PACKET_LEN];
        loop {
            let (len, addr) = socket.recv_from(&mut buf).await?;
            self.receive_datagram(&buf[..len], addr).await?;
        }
    }

    async fn receive_datagram(&self, buf: &[u8], addr: SocketAddr) -> io::Result<()> {
        match self.handle_datagram(buf, addr) {
            Some(relay) => {
                let connected_client = relay.clone();
                let relay_client = relay.clone();

                self.connected_clients
                    .lock()
                    .await
                    .insert(addr, connected_client);
                self.relay_clients
                    .lock()
                    .await
                    .insert(relay_client.key.clone(), relay_client);
            }
            _ => match self.connected_clients.lock().await.get(&addr) {
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

    fn handle_datagram(&self, buf: &[u8], addr: SocketAddr) -> Option<Arc<Relay>> {
        let data = std::str::from_utf8(&buf).unwrap().trim_end_matches("\n");
        let parts = data.split_whitespace().collect::<Vec<&str>>();
        let cmd = parts[0];

        println!("command: {:?}", cmd);
        match cmd {
            "CONNECT" => {
                let key = parts[1];
                let relay = Relay::new(key.to_string(), addr, self.sender.clone());
                Some(Arc::new(relay))
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
