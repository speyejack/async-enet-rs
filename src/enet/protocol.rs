use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ProtocolCommand {
    None,
    Ack,
    Connect(ConnectPacket),
    VerifyConnect,
    Disconnect,
    Ping,
    SendReliable,
    SendUnReliable,
    SendFragment,
    SendUnsequenced,
    BandwidthLimit,
    ThrottleConfigure,
    SendUnreliableFragment,
    Count,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PacketHeader {
    pub peer_id: u16,
    pub sent_time: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolCommandHeader {
    pub command: u8,
    pub channel_id: u8,
    pub reliable_sequence_number: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Protocol {
    pub packet_header: PacketHeader,
    pub command_header: ProtocolCommandHeader,
    pub command: ProtocolCommand,
}

pub enum ProtocolFlag {
    Ack,
    Usequenced,
    Compressed,
    SentTime,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AcknowledgePacket {
    pub received_reliable_sequence_number: u16,
    pub received_sent_time: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectPacket {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyConnectPacket {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct BandwidthLimitPacket {
    pub incoming_bandwidth: u32,
    pub outgoing_bandwidth: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ThrottleConfigurePacket {
    pub packet_throttle_interval: u32,
    pub packet_throttle_acceleration: u32,
    pub packet_throttle_deceleration: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DisconnectPacket {
    pub data: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PingPacket {}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendReliablePacket {
    pub data_length: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendUnreliablePacket {
    pub unreliable_sequence_number: u16,
    pub data_length: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendUnsequencedPacket {
    pub unsequenced_group: u16,
    pub data_length: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendFragmentPacket {
    pub start_sequence_number: u16,
    pub data_length: u16,
    pub fragment_count: u32,
    pub fragment_number: u32,
    pub total_length: u32,
    pub fragment_offset: u32,
}
