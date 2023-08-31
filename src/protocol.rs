use std::{net::SocketAddr, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{error::ENetError, net::time::PacketTime, peer::PeerID};

/// A wrapper around a packet with enet metadata
#[derive(Debug, Clone)]
pub struct Command {
    pub info: CommandInfo,
    pub command: ProtocolCommand,
}

/// Enet Command header information
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub addr: SocketAddr,
    pub flags: PacketFlags,
    pub internal_peer_id: PeerID,
    pub peer_id: PeerID,
    pub channel_id: u8,
    pub session_id: u16,
    pub reliable_sequence_number: u16,
    pub sent_time: Duration,
}

/// Flags for packet behavior on enet
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketFlags {
    pub reliable: bool,
    pub unsequenced: bool,

    pub no_allocate: bool,
    pub unreliable_fragment: bool,
    pub sent: bool,

    pub send_time: bool,
    pub is_compressed: bool,
}

impl Default for PacketFlags {
    fn default() -> Self {
        Self {
            reliable: Default::default(),
            unsequenced: Default::default(),
            no_allocate: Default::default(),
            unreliable_fragment: Default::default(),
            sent: Default::default(),
            send_time: true,
            is_compressed: Default::default(),
        }
    }
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

/// Client header information before commands in udp packet
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtocolHeader {
    pub peer_id: u16,   // peer id from outgoing peer id | HEADER FLAGS
    pub sent_time: u16, // first 16 bits of current time (ms) (dont be zero)
}

/// Command header information in udp packet
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtocolCommandHeader {
    pub command: u8,                   // Command | Command flags
    pub channel_id: u8, // Separate from clients (can have multiple clients), I think specified by user unless special packet then 0xFF
    pub reliable_sequence_number: u16, // Per channel, outgoing and incoming saved per channel
}

/// Different types of commands used in enet
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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
    SendUnreliableFragment(SendUnreliableFragmentCommand),
    Count,
}

/// Command to acknowledge a previous reliable packet
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct AcknowledgeCommand {
    pub received_reliable_sequence_number: u16, // Current number (no inc)
    pub received_sent_time: PacketTime,         // Ack sent time
}

/// Command to begin a connection with another enet instance
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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

/// The reply command to a connect command from another enet instance
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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

/// Command to change the bandwidth to peer
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BandwidthLimitCommand {
    pub incoming_bandwidth: u32,
    pub outgoing_bandwidth: u32,
}

/// Command to change the throttling to peer
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ThrottleConfigureCommand {
    pub packet_throttle_interval: u32,
    pub packet_throttle_acceleration: u32,
    pub packet_throttle_deceleration: u32,
}

/// Command gracefully disconnect from peer
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct DisconnectCommand {
    pub data: u32,
}

/// Command to ping peer
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct PingCommand {}

/// Command send reliable data
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SendReliableCommand {
    // pub data_length: u16,
    pub data: Vec<u8>,
}

/// Command send unreliable data
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SendUnreliableCommand {
    pub unreliable_sequence_number: u16, // Used for fragmenting
    // pub data_length: u16,
    pub data: Vec<u8>,
}

/// Command send unsequenced data
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SendUnsequencedCommand {
    pub unsequenced_group: u16,
    // pub data_length: u16,
    pub data: Vec<u8>,
}

/// Command send a fragmented packet
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SendFragmentCommand {
    pub start_sequence_number: u16,
    pub data_length: u16,
    pub fragment_count: u32,
    pub fragment_number: u32,
    pub total_length: u32,
    pub fragment_offset: u32,
    pub data: Vec<u8>,
}

/// Command send a unreliable fragmented packet
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SendUnreliableFragmentCommand {
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
impl_packet_conv!(DisconnectCommand, ProtocolCommand::Disconnect);
impl_packet_conv!(PingCommand, ProtocolCommand::Ping);
impl_packet_conv!(SendReliableCommand, ProtocolCommand::SendReliable);
impl_packet_conv!(SendUnreliableCommand, ProtocolCommand::SendUnreliable);
impl_packet_conv!(SendFragmentCommand, ProtocolCommand::SendFragment);
impl_packet_conv!(SendUnsequencedCommand, ProtocolCommand::SendUnsequenced);
impl_packet_conv!(BandwidthLimitCommand, ProtocolCommand::BandwidthLimit);
impl_packet_conv!(ThrottleConfigureCommand, ProtocolCommand::ThrottleConfigure);
impl_packet_conv!(
    SendUnreliableFragmentCommand,
    ProtocolCommand::SendUnreliableFragment
);
