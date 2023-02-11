use std::{collections::HashSet, fmt::Display, net::SocketAddr};

use super::{
    channel::{Channel, ChannelID},
    protocol::{Command, PacketFlags},
    ChannelError, ENetError,
};

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct PeerID(pub u16);

impl Display for PeerID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<u16> for PeerID {
    fn from(value: u16) -> Self {
        PeerID(value)
    }
}

impl From<u8> for PeerID {
    fn from(value: u8) -> Self {
        let val: u16 = value.into();
        val.into()
    }
}

impl From<PeerID> for u16 {
    fn from(value: PeerID) -> Self {
        value.0
    }
}

impl TryFrom<PeerID> for u8 {
    type Error = ENetError;

    fn try_from(value: PeerID) -> Result<Self, Self::Error> {
        let val: u16 = value.0.into();
        Ok(val.try_into()?)
    }
}

pub struct PeerInfo {
    pub(crate) outgoing_peer_id: PeerID,
    pub(crate) incoming_peer_id: PeerID,
    pub(crate) connect_id: u32, // Originally was u16
    pub(crate) outgoing_session_id: u16,
    pub(crate) incoming_session_id: u16,
    pub(crate) address: SocketAddr,

    pub(crate) channels: Vec<Channel>,

    pub(crate) incoming_bandwidth: u32,
    pub(crate) outgoing_bandwidth: u32,

    pub(crate) packet_throttle_interval: u32,
    pub(crate) packet_throttle_acceleration: u32,
    pub(crate) packet_throttle_deceleration: u32,

    pub(crate) mtu: u32,
    pub(crate) window_size: u32,

    pub(crate) event_data: u32,
}

#[derive(Debug)]
pub struct Peer {
    id: PeerID,

    out_channel: tokio::sync::mpsc::Sender<(PeerSendEvent, PeerID, ChannelID)>,
    in_channel: tokio::sync::mpsc::Receiver<PeerEvent>,
}

impl Peer {
    async fn send(
        &mut self,
        p: Packet,
        channel: ChannelID,
    ) -> std::result::Result<(), ChannelError> {
        self.out_channel
            .send((PeerSendEvent::Send(p), self.id, channel))
            .await?;
        Ok(())
    }

    async fn poll(&mut self) -> Option<PeerEvent> {
        self.in_channel.recv().await
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
    Ping,
    Disconnect,
}

#[derive(Debug)]
pub enum PeerEvent {
    Recv(Packet),
    Disconnect,
}
