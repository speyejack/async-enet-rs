mod peer_id;
pub use peer_id::*;
use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

use tokio::sync::mpsc::Sender;

use super::{
    channel::{Channel, ChannelID},
    error::{ChannelError, ENetError, Result},
    host::hostevents::{HostRecvEvent, HostSendEvent},
    protocol::PacketFlags,
};

#[derive(Debug)]
pub struct PeerInfo {
    pub(crate) outgoing_peer_id: OutgoingPeerID,
    pub(crate) incoming_peer_id: PeerID,
    pub(crate) connect_id: u32, // Originally was u16
    pub(crate) outgoing_session_id: u16,
    pub(crate) incoming_session_id: u16,
    pub(crate) address: SocketAddr,

    pub(crate) channels: HashMap<ChannelID, Channel>,

    pub(crate) incoming_bandwidth: u32,
    pub(crate) outgoing_bandwidth: u32,

    pub(crate) packet_throttle_interval: u32,
    pub(crate) packet_throttle_acceleration: u32,
    pub(crate) packet_throttle_deceleration: u32,

    pub(crate) _mtu: u32,
    pub(crate) _window_size: u32,

    pub(crate) _event_data: u32,

    pub(crate) outgoing_reliable_sequence_number: u16,
    pub(crate) incoming_reliable_sequence_number: u16,
    pub(crate) sender: Sender<HostSendEvent>,

    pub(crate) last_msg_time: Instant,
    pub(crate) round_trip_time: Duration,
    pub(crate) round_trip_time_variance: Duration,
}

pub struct Peer {
    pub(crate) id: PeerID,
    pub(crate) address: SocketAddr,

    pub(crate) out_channel: tokio::sync::mpsc::Sender<HostRecvEvent>,
    pub(crate) in_channel: tokio::sync::mpsc::Receiver<HostSendEvent>,
}

impl std::fmt::Debug for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Peer")
            .field("id", &self.id)
            .field("address", &self.address)
            .finish()
    }
}

impl Peer {
    pub async fn send(&mut self, p: Packet) -> std::result::Result<(), ChannelError> {
        self.out_channel
            .send(HostRecvEvent {
                channel_id: p.channel,
                event: PeerSendEvent::Send(p),
                peer_id: self.id,
            })
            .await?;
        Ok(())
    }

    pub async fn broadcast(&mut self, p: Packet) -> std::result::Result<(), ChannelError> {
        self.out_channel
            .send(HostRecvEvent {
                channel_id: p.channel,
                event: PeerSendEvent::Broadcast(p),
                peer_id: self.id,
            })
            .await?;
        Ok(())
    }

    pub async fn poll(&mut self) -> PeerRecvEvent {
        let event = self.in_channel.recv().await;
        match event {
            None => PeerRecvEvent::Disconnect,
            Some(e) => e.event,
        }
    }

    pub async fn disconnect(self) {
        let _result = self
            .out_channel
            .send(HostRecvEvent {
                event: PeerSendEvent::Disconnect,
                peer_id: self.id,
                channel_id: 0xFF,
            })
            .await;
    }

    pub fn get_address(&self) -> SocketAddr {
        self.address
    }
}

impl PeerInfo {
    pub fn get_channel(&self, id: ChannelID) -> Result<&Channel> {
        let channel = self
            .channels
            .get(&id)
            .ok_or(ENetError::InvalidChannelId(id));
        channel
    }

    pub fn get_mut_channel(&mut self, id: ChannelID) -> Result<&mut Channel> {
        let channel = self
            .channels
            .get_mut(&id)
            .ok_or(ENetError::InvalidChannelId(id));
        channel
    }
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub data: Vec<u8>,
    pub channel: ChannelID,
    pub flags: PacketFlags,
}

#[derive(Debug, Clone)]
pub enum PeerSendEvent {
    Send(Packet),
    Broadcast(Packet),
    Ping,
    Disconnect,
}

#[derive(Debug)]
pub enum PeerRecvEvent {
    Recv(Packet),
    Disconnect,
}
