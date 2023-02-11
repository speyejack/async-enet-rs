use std::{net::SocketAddr, time::Duration};

use serde::{Deserialize, Serialize};

use super::{peer::PeerID, ENetError};

#[derive(Debug, Clone)]
pub struct Command {
    pub metadata: CommandInfo,
    pub command: ProtocolCommand,
}

#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub addr: SocketAddr,
    pub flags: PacketFlags,
    pub peer_id: PeerID,
    pub channel_id: u8,
    pub reliable_sequence_number: u16,
    pub sent_time: Duration,
}

#[derive(Debug, Default, Clone)]
pub struct PacketFlags {
    pub reliable: bool,
    pub unsequenced: bool,

    pub no_allocate: bool,
    pub unreliable_fragment: bool,
    pub sent: bool,
}

impl PacketFlags {
    pub fn reliable() -> Self {
        let flags: Self = PacketFlags {
            reliable: true,
            ..Default::default()
        };
        flags
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandHeader {
    pub peer_id: u16,   // peer id from outgoing peer id | HEADER FLAGS
    pub sent_time: u16, // first 16 bits of current time (ms) (dont be zero)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolCommandHeader {
    pub command: u8,                   // Command | Command flags
    pub channel_id: u8, // Separate from clients (can have multiple clients), I think specified by user unless special packet then 0xFF
    pub reliable_sequence_number: u16, // Per channel, outgoing and incoming saved per channel
}

// Command types used for protocol
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProtocolCommand {
    None,
    Ack(AcknowledgeCommand),
    Connect(ConnectCommand),
    VerifyConnect(VerifyConnectCommand),
    Disconnect(DisconnectCommand),
    Ping(PingCommand),
    SendReliable(SendReliableCommand),
    SendUnreliable(SendUnreliableCommand),
    SendFragment(SendFragmentCommand),
    SendUnsequenced(SendUnsequencedCommand),
    BandwidthLimit(BandwidthLimitCommand),
    ThrottleConfigure(ThrottleConfigureCommand),
    SendUnreliableFragment(SendFragmentCommand),
    Count,
}

// Different command types
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AcknowledgeCommand {
    pub received_reliable_sequence_number: u16, // Current number (no inc)
    pub received_sent_time: u16,                // Ack sent time
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectCommand {
    pub outgoing_peer_id: u16,
    pub incoming_session_id: u8,
    pub outgoing_session_id: u8,
    pub mtu: u32,
    pub window_size: u32,
    pub channel_count: u32,
    pub incoming_bandwidth: u32,
    pub outgoing_bandwidth: u32,
    pub packet_throttle_interval: u32,
    pub packet_throttle_acceleration: u32,
    pub packet_throttle_deceleration: u32,
    pub connect_id: u32,
    pub data: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerifyConnectCommand {
    pub outgoing_peer_id: u16,
    pub incoming_session_id: u8,
    pub outgoing_session_id: u8,
    pub mtu: u32,
    pub window_size: u32,
    pub channel_count: u32,
    pub incoming_bandwidth: u32,
    pub outgoing_bandwidth: u32,
    pub packet_throttle_interval: u32,
    pub packet_throttle_acceleration: u32,
    pub packet_throttle_deceleration: u32,
    pub connect_id: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BandwidthLimitCommand {
    pub incoming_bandwidth: u32,
    pub outgoing_bandwidth: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThrottleConfigureCommand {
    pub packet_throttle_interval: u32,
    pub packet_throttle_acceleration: u32,
    pub packet_throttle_deceleration: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DisconnectCommand {
    pub data: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PingCommand {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendReliableCommand {
    // pub data_length: u16,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendUnreliableCommand {
    pub unreliable_sequence_number: u16, // Used for fragmenting
    // pub data_length: u16,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendUnsequencedCommand {
    pub unsequenced_group: u16,
    // pub data_length: u16,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendFragmentCommand {
    pub start_sequence_number: u16,
    pub data_length: u16,
    pub fragment_count: u32,
    pub fragment_number: u32,
    pub total_length: u32,
    pub fragment_offset: u32,
    pub data: Vec<u8>,
}

macro_rules! impl_packet_conv {
    ($p: ty, $wrap: path) => {
        impl From<$p> for ProtocolCommand {
            fn from(value: $p) -> Self {
                $wrap(value)
            }
        }

        impl TryFrom<ProtocolCommand> for $p {
            type Error = ENetError;

            fn try_from(value: ProtocolCommand) -> Result<Self, Self::Error> {
                match value {
                    $wrap(p) => Ok(p),
                    _ => Err(ENetError::UnexpectedPacketType),
                }
            }
        }
    };
}

impl_packet_conv!(AcknowledgeCommand, ProtocolCommand::Ack);
impl_packet_conv!(ConnectCommand, ProtocolCommand::Connect);
impl_packet_conv!(VerifyConnectCommand, ProtocolCommand::VerifyConnect);
impl_packet_conv!(BandwidthLimitCommand, ProtocolCommand::BandwidthLimit);
impl_packet_conv!(ThrottleConfigureCommand, ProtocolCommand::ThrottleConfigure);
impl_packet_conv!(DisconnectCommand, ProtocolCommand::Disconnect);
impl_packet_conv!(PingCommand, ProtocolCommand::Ping);
impl_packet_conv!(SendReliableCommand, ProtocolCommand::SendReliable);
impl_packet_conv!(SendUnreliableCommand, ProtocolCommand::SendUnreliable);
impl_packet_conv!(SendUnsequencedCommand, ProtocolCommand::SendUnsequenced);
impl_packet_conv!(SendFragmentCommand, ProtocolCommand::SendUnreliableFragment);
