use std::{collections::HashMap, net::SocketAddr};

use tokio::{io, net::UdpSocket};

pub const MAX_PACKET_LEN: usize = 1024;
pub const CONNECT_STR: &str = "CONNECT";

pub struct Server {
    addr: String,
    clients: HashMap<String, Vec<SocketAddr>>,
    connections: HashMap<SocketAddr, String>,
}

impl Server {
    pub fn new(addr: String) -> Self {
        Self {
            addr,
            clients: HashMap::new(),
            connections: HashMap::new(),
        }
    }

    pub async fn listen(&mut self) -> io::Result<()> {
        let socket = UdpSocket::bind(&self.addr).await?;
        self.receive(socket).await
    }

    async fn receive(&mut self, socket: UdpSocket) -> io::Result<()> {
        let mut buf = [0; MAX_PACKET_LEN];
        loop {
            let (n, addr) = socket.recv_from(&mut buf).await?;
            println!("Received {} bytes from {}", n, addr);

            self.receive_datagram(&socket, &buf[..n], addr).await?;
        }
    }

    async fn receive_datagram(
        &mut self,
        socket: &UdpSocket,
        buf: &[u8],
        addr: SocketAddr,
    ) -> io::Result<()> {
        match self.handle_connect(buf) {
            Some(key) => self.register_client(key, addr),
            None => self.forward_packet(socket, buf, addr).await?,
        }
        Ok(())
    }

    fn handle_connect(&self, buf: &[u8]) -> Option<String> {
        let data = std::str::from_utf8(&buf).unwrap().trim_end_matches("\n");
        let parts = data.split_whitespace().collect::<Vec<&str>>();
        if parts.len() == 2 && parts[0] == CONNECT_STR {
            return Some(parts[1].to_string());
        }
        None
    }

    fn register_client(&mut self, key: String, addr: SocketAddr) {
        println!("registering client: {}", key);
        self.clients
            .entry(key.clone())
            .and_modify(|v| v.push(addr))
            .or_insert(vec![addr]);

        self.connections.insert(addr, key);
    }

    async fn forward_packet(
        &self,
        socket: &UdpSocket,
        buf: &[u8],
        addr: SocketAddr,
    ) -> io::Result<()> {
        match self
            .connections
            .get(&addr)
            .take()
            .map(|key| {
                self.clients.get(key).map(|addrs| {
                    addrs
                        .iter()
                        .filter(|a| a != &&addr)
                        .map(|a| socket.send_to(buf, a))
                        .collect::<Vec<_>>()
                })
            })
            .flatten()
        {
            Some(futures) => match futures::future::join_all(futures)
                .await
                .into_iter()
                .find(|f| f.is_err())
            {
                Some(Err(e)) => Err(e),
                _ => Ok(()),
            },

            _ => Ok(()),
        }
    }
}
