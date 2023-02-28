use std::{
    collections::HashMap,
    net::SocketAddr,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::{bail, ensure, Context, Result};
use tokio::{io, net::UdpSocket, sync::Mutex};

pub const MAX_PACKET_LEN: usize = 1500;

pub type Packet = Vec<u8>;

enum Command {
    Connect,
    Disconnect,
}

impl FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CONNECT" => Ok(Self::Connect),
            "DISCONNECT" => Ok(Self::Disconnect),
            _ => bail!("invalid command"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct ClientSession {
    addr: SocketAddr,
    last_seen: SystemTime,
}

pub struct Server {
    api_key: String,
    addr: String,
    clients: Mutex<HashMap<String, Vec<ClientSession>>>,
    connections: Mutex<HashMap<SocketAddr, String>>,
}

impl Server {
    pub fn new(api_key: String, addr: String) -> Arc<Self> {
        Arc::new(Self {
            api_key,
            addr,
            clients: Mutex::new(HashMap::new()),
            connections: Mutex::new(HashMap::new()),
        })
    }

    pub async fn listen(self: Arc<Self>) -> Result<()> {
        self.clone().run_gc();

        let socket = Arc::new(UdpSocket::bind(&self.addr).await?);
        let mut buf = [0; MAX_PACKET_LEN];
        loop {
            let (n, addr) = socket.recv_from(&mut buf).await?;

            let socket = socket.clone();
            let server = self.clone();
            tokio::task::spawn(async move {
                if let Err(e) = server
                    .handle_datagram(socket, &buf[..n].to_vec(), addr)
                    .await
                {
                    println!("Error receiving datagram: {}", e);
                }
            });
        }
    }

    async fn handle_datagram(
        &self,
        socket: Arc<UdpSocket>,
        buf: &Packet,
        addr: SocketAddr,
    ) -> Result<()> {
        let command = self
            .check_for_command(buf)
            .context("Failed to parse command")?;

        match command {
            Some(Command::Connect) => self.handle_connect_command(addr, buf).await,
            Some(Command::Disconnect) => self.handle_disconnect_command(addr, buf).await,
            _ => Ok(self.forward_packet(socket, buf, addr).await?),
        }
    }

    fn check_for_command(&self, buf: &Packet) -> Result<Option<Command>> {
        if let Ok(utf8_data) = std::str::from_utf8(&buf) {
            let parts = utf8_data.split_whitespace().collect::<Vec<_>>();
            ensure!(parts.len() >= 1, "Invalid command packet");

            if let Ok(command) = parts[0].parse::<Command>() {
                return Ok(Some(command));
            }
        }

        // We want to allow non-utf8 data to be sent to be forwarded
        Ok(None)
    }

    async fn handle_connect_command(&self, addr: SocketAddr, buf: &Packet) -> Result<()> {
        let utf8_data = std::str::from_utf8(&buf)?;
        let parts = utf8_data.split_whitespace().collect::<Vec<_>>();
        ensure!(parts.len() == 3, "Invalid CONNECT packet");

        let api_key = parts[1].to_string();
        let client_id = parts[2].to_string();

        ensure!(
            api_key == self.api_key,
            "Invalid API key for client: {}",
            client_id
        );

        println!("registering client: {:?} {:?}", addr, client_id);
        self.register_client(client_id, addr).await;
        Ok(())
    }

    async fn handle_disconnect_command(&self, addr: SocketAddr, buf: &Packet) -> Result<()> {
        let utf8_data = std::str::from_utf8(&buf)?;
        let parts = utf8_data.split_whitespace().collect::<Vec<_>>();
        ensure!(parts.len() == 2, "Invalid DISCONNECT packet");

        let client_id = parts[1].to_string();
        println!("deregistering client: {:?}", client_id);

        self.deregister_client(client_id, addr).await;
        Ok(())
    }

    async fn register_client(&self, key: String, addr: SocketAddr) {
        let mut conns = self.connections.lock().await;
        if let Some(_) = conns.get(&addr) {
            return; // already registered, no-op
        }

        let session = ClientSession {
            addr,
            last_seen: SystemTime::now(),
        };

        self.clients
            .lock()
            .await
            .entry(key.clone())
            .and_modify(|v| v.push(session))
            .or_insert(vec![session]);

        conns.insert(addr, key);
    }

    async fn deregister_client(&self, key: String, addr: SocketAddr) {
        let mut conns = self.connections.lock().await;
        if let None = conns.get(&addr) {
            return; // not registered, no-op
        }

        self.clients.lock().await.remove(&key);
        conns.remove(&addr);
    }

    async fn forward_packet(
        &self,
        socket: Arc<UdpSocket>,
        buf: &Packet,
        addr: SocketAddr,
    ) -> io::Result<()> {
        if let Some(key) = self.connections.lock().await.get(&addr) {
            if let Some(peers) = self.clients.lock().await.get_mut(key) {
                let send_results = peers.iter_mut().filter_map(|session| {
                    // Don't send to self, but update last_seen
                    if session.addr == addr {
                        session.last_seen = SystemTime::now();
                        return None;
                    }

                    Some(socket.send_to(&buf, session.addr))
                });

                for result in futures::future::join_all(send_results).await {
                    result?;
                }
            }
        }

        Ok(())
    }

    fn run_gc(self: Arc<Self>) {
        tokio::task::spawn(async move {
            self.gc_inactive_clients().await;
        });
    }

    async fn gc_inactive_clients(&self) {
        // 300 secs / 5 mins
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        interval.tick().await;

        loop {
            interval.tick().await;
            println!("Running GC");

            let mut clients = self.clients.lock().await;
            let mut conns = self.connections.lock().await;
            let mut clients_to_remove = vec![];
            let mut conns_to_remove = vec![];

            for (key, sessions) in clients.iter_mut() {
                sessions.retain(|session| {
                    let now = SystemTime::now();
                    let elapsed = now.duration_since(session.last_seen).unwrap();
                    let remove = elapsed.as_secs() > 43200; // no activity within 12 hours

                    if remove {
                        conns_to_remove.push(session.addr);
                    }

                    !remove
                });

                if sessions.is_empty() {
                    clients_to_remove.push(key.clone());
                }
            }

            println!("Removing {} clients", clients_to_remove.len());
            for key in clients_to_remove {
                clients.remove(&key);
            }

            println!("Removing {} connections", conns_to_remove.len());
            for addr in conns_to_remove {
                conns.remove(&addr);
            }
        }
    }
}
