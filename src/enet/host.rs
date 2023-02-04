use super::{peer::Peer, ENetAddress, Result};

pub struct HostConfig {
    pub peer_count: usize,
    pub channel_limit: Option<usize>,
    pub incoming_bandwidth: Option<u32>,
    pub outgoing_bandwidth: Option<u32>,
}

impl HostConfig {
    fn new(peer_count: usize) -> Result<Self> {
        Ok(HostConfig {
            peer_count,
            channel_limit: None,
            incoming_bandwidth: None,
            outgoing_bandwidth: None,
        })
    }
}

pub struct Host {
    peers: Vec<Peer>,
    config: HostConfig,

    // bandwidth_throttle_epoch: u32,
    // mtu: u32,
    random: random::Default,
}

impl Host {
    pub async fn create(config: HostConfig, addr: ENetAddress) -> Result<Self> {
        let socket = tokio::net::UdpSocket::bind(addr).await?;
        let random = random::default(10);
        // TODO Set flags

        // TODO Set default host

        let peers = Vec::new();
        // TODO Set default peers ... maybe

        Ok(Host {
            peers,
            config,
            random,
        })
    }

    pub async fn connect(
        &mut self,
        addr: ENetAddress,
        channel_count: usize,
        data: u32,
    ) -> Result<Peer> {
        // TODO check channel count
        let new_peer = Peer::new(addr);
        self.peers.push(new_peer);
        unimplemented!();
    }
}
