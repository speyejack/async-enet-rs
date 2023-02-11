use std::time::Duration;

use bytes::{Buf, BytesMut};
use enet::{
    deserializer::EnetDeserializer,
    host::{Host, HostConfig},
    protocol::ConnectCommand,
    Result,
};
use serde::{Deserialize, Serialize};
use tokio::{io::BufWriter, net::UdpSocket};

use crate::enet::{
    host::HostEvent,
    protocol::{
        Command, CommandHeader, CommandInfo, PacketFlags, ProtocolCommand, ProtocolCommandHeader,
        VerifyConnectCommand,
    },
    serializer::EnetSerializer,
    sizer::EnetSizer,
};

mod enet;

#[tokio::main]
async fn main() -> Result<()> {
    let config = HostConfig::new(10)?;
    let mut host = Host::create(config, "0.0.0.0:1027").await?;

    let packet = host.poll().await?;
    println!("first: {packet:#?}");
    let packet = host.poll().await?;
    println!("second: {packet:#?}");
    let packet = host.poll().await?;
    println!("third: {packet:#?}");

    // let mut buf = [0; 100];
    // println!("Packet: {packet:#x?}");
    // let connect_id = match packet.command {
    //     enet::protocol::ProtocolCommand::Connect(p) => p.connect_id,
    //     _ => panic!("connect not first"),
    // };

    // let info = CommandInfo {
    //     addr: packet.metadata.addr,
    //     flags: PacketFlags::reliable(),
    //     peer_id: 0x8000_u16.into(),
    //     channel_id: 0xff,
    //     reliable_sequence_number: 0x1,
    //     sent_time: Duration::from_millis(0x17e0),
    // };

    // let packet = VerifyConnectCommand {
    //     outgoing_peer_id: 0x0000,
    //     incoming_session_id: 0x00,
    //     outgoing_session_id: 0x00,
    //     mtu: 0x578,
    //     window_size: 0x1000,
    //     channel_count: 0x2,
    //     incoming_bandwidth: 0x0,
    //     outgoing_bandwidth: 0x0,
    //     packet_throttle_interval: 0x1388,
    //     packet_throttle_acceleration: 0x02,
    //     packet_throttle_deceleration: 0x02,
    //     connect_id,
    // };

    // host.send(Command {
    //     command: packet.into(),
    //     metadata: info,
    // })
    // .await?;

    loop {
        let command = host.poll().await?;

        println!("Got packet: {command:#x?}");
    }
}
