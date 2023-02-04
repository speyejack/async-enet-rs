use bytes::{Buf, BytesMut};
use enet::{deserializer::EnetDeserializer, protocol::ConnectPacket, Result};
use serde::{Deserialize, Serialize};
use tokio::{io::BufWriter, net::UdpSocket};

use crate::enet::{
    protocol::{PacketHeader, ProtocolCommandHeader, VerifyConnectPacket},
    serializer::EnetSerializer,
    sizer::EnetSizer,
};

mod enet;

#[tokio::main]
async fn main() -> Result<()> {
    // let mut buf = BytesMut::with_capacity(100);
    let mut buf = [0; 100];
    let socket = UdpSocket::bind("0.0.0.0:1027").await?;

    // let socket = BufWriter::new(socket);
    println!("Buff: {:?}", buf.len());
    let (len, addr) = socket.recv_from(&mut buf[..]).await?;
    println!("{len:?}- {addr:?} - {buf:?}");
    let mut deser = EnetDeserializer { input: &buf[..] };

    let header = PacketHeader::deserialize(&mut deser)?;
    let packet_type = ProtocolCommandHeader::deserialize(&mut deser)?;
    let packet = ConnectPacket::deserialize(&mut deser)?;
    println!("Header: {header:x?}");
    println!("Type: {packet_type:x?}");
    println!("Packet: {packet:#x?}");
    let connect_id = packet.connect_id;

    let header = PacketHeader {
        peer_id: 0x8000,
        sent_time: 0x17e0,
    };

    let packet_type = ProtocolCommandHeader {
        command: 0x83,
        channel_id: 0xff,
        reliable_sequence_number: 0x01,
    };

    let packet = VerifyConnectPacket {
        outgoing_peer_id: 0x0000,
        incoming_session_id: 0x00,
        outgoing_session_id: 0x00,
        mtu: 0x578,
        window_size: 0x1000,
        channel_count: 0x2,
        incoming_bandwidth: 0x0,
        outgoing_bandwidth: 0x0,
        packet_throttle_interval: 0x1388,
        packet_throttle_acceleration: 0x02,
        packet_throttle_deceleration: 0x02,
        connect_id,
    };
    let mut sizer = EnetSizer { size: 0 };
    header.serialize(&mut sizer)?;
    packet_type.serialize(&mut sizer)?;
    packet.serialize(&mut sizer)?;

    let packet_size = sizer.size;

    let mut ser = EnetSerializer {
        output: &mut buf[..],
    };

    header.serialize(&mut ser)?;
    packet_type.serialize(&mut ser)?;
    packet.serialize(&mut ser)?;

    socket.send_to(&buf[..packet_size], addr).await?;

    Ok(())
}
