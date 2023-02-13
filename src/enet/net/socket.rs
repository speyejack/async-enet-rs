use std::net::SocketAddr;

use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tokio::{net::UdpSocket, time::Duration};

use crate::enet::{
    protocol::{
        AcknowledgeCommand, BandwidthLimitCommand, Command, CommandInfo, ConnectCommand,
        DisconnectCommand, PacketFlags, PingCommand, ProtocolCommand, ProtocolCommandHeader,
        ProtocolHeader, SendFragmentCommand, SendReliableCommand, SendUnreliableCommand,
        SendUnsequencedCommand, ThrottleConfigureCommand, VerifyConnectCommand,
    },
    ENetError, Result,
};

use super::{deserializer::EnetDeserializer, serializer::EnetSerializer};

pub struct ENetSocket {
    pub socket: UdpSocket,
    buf: [u8; 100],
}

impl ENetSocket {
    pub fn new(socket: UdpSocket) -> Self {
        ENetSocket {
            socket,
            buf: [0; 100],
        }
    }

    pub async fn recv(&mut self) -> Result<Command> {
        let (len, addr) = self.socket.recv_from(&mut self.buf).await?;
        self.deserialize_command(addr)
    }

    fn deserialize_command(&mut self, addr: SocketAddr) -> Result<Command> {
        let buf = &self.buf[..];
        let mut deser = EnetDeserializer { input: buf };

        let header = ProtocolHeader::deserialize(&mut deser)?;
        let packet_type = ProtocolCommandHeader::deserialize(&mut deser)?;

        let packet: ProtocolCommand = match &packet_type.command & 0x0F {
            0 => ProtocolCommand::None,
            1 => AcknowledgeCommand::deserialize(&mut deser)?.into(),
            2 => ConnectCommand::deserialize(&mut deser)?.into(),
            3 => VerifyConnectCommand::deserialize(&mut deser)?.into(),
            4 => DisconnectCommand::deserialize(&mut deser)?.into(),
            5 => PingCommand::deserialize(&mut deser)?.into(),
            6 => SendReliableCommand::deserialize(&mut deser)?.into(),
            7 => SendUnreliableCommand::deserialize(&mut deser)?.into(),
            8 => SendFragmentCommand::deserialize(&mut deser)?.into(),
            9 => SendUnsequencedCommand::deserialize(&mut deser)?.into(),
            10 => BandwidthLimitCommand::deserialize(&mut deser)?.into(),
            11 => ThrottleConfigureCommand::deserialize(&mut deser)?.into(),
            12 => ProtocolCommand::SendUnreliableFragment(SendFragmentCommand::deserialize(
                &mut deser,
            )?),
            13 => ProtocolCommand::Count,
            _ => return Err(ENetError::Other("Invalid packet header".to_string())),
        };

        let flags = PacketFlags {
            reliable: ((&packet_type.command >> 7) & 1) == 1,
            unsequenced: ((&packet_type.command >> 6) & 1) == 1,

            // TODO impl these flags
            no_allocate: false,
            unreliable_fragment: false,
            sent: false,
        };

        let info = CommandInfo {
            addr,
            flags,
            peer_id: header.peer_id.into(),
            channel_id: packet_type.channel_id,
            reliable_sequence_number: packet_type.reliable_sequence_number,
            sent_time: Duration::from_millis(header.sent_time.into()),
        };

        Ok(Command {
            command: packet,
            info,
        })
    }

    pub async fn send(&mut self, command: &Command) -> Result<()> {
        let addr = command.info.addr;
        let (bytes, size) = self.serialize_command(&command)?;

        println!("Sending packet: {size} => {bytes:?}");
        self.socket.send_to(&bytes[0..size], addr).await?;
        Ok(())
    }

    fn serialize_command(&self, mut p: &Command) -> Result<(Bytes, usize)> {
        let mut buff = BytesMut::zeroed(100);
        let mut ser = EnetSerializer {
            output: &mut buff[..],
            size: 0,
        };

        let command = match p.command {
            ProtocolCommand::None => 0,
            ProtocolCommand::Ack(_) => 1,
            ProtocolCommand::Connect(_) => 2,
            ProtocolCommand::VerifyConnect(_) => 3,
            ProtocolCommand::Disconnect(_) => 4,
            ProtocolCommand::Ping(_) => 5,
            ProtocolCommand::SendReliable(_) => 6,
            ProtocolCommand::SendUnreliable(_) => 7,
            ProtocolCommand::SendFragment(_) => 8,
            ProtocolCommand::SendUnsequenced(_) => 9,
            ProtocolCommand::BandwidthLimit(_) => 10,
            ProtocolCommand::ThrottleConfigure(_) => 11,
            ProtocolCommand::SendUnreliableFragment(_) => 12,
            ProtocolCommand::Count => 13,
        };

        let flags = &p.info.flags;
        let command_flags =
            if flags.reliable { 1 << 7 } else { 0 } | if flags.unsequenced { 1 << 6 } else { 0 };

        let command = command | command_flags;

        let command_header = ProtocolCommandHeader {
            command,
            channel_id: p.info.channel_id,
            reliable_sequence_number: p.info.reliable_sequence_number,
        };

        let packet_header = ProtocolHeader {
            peer_id: p.info.peer_id.into(),
            sent_time: p.info.sent_time.as_millis() as u16,
        };

        packet_header.serialize(&mut ser)?;
        command_header.serialize(&mut ser)?;

        let val = match &p.command {
            ProtocolCommand::Ack(l) => l.serialize(&mut ser),
            ProtocolCommand::Connect(l) => l.serialize(&mut ser),
            ProtocolCommand::VerifyConnect(l) => l.serialize(&mut ser),
            ProtocolCommand::Disconnect(l) => l.serialize(&mut ser),
            ProtocolCommand::Ping(l) => l.serialize(&mut ser),
            ProtocolCommand::SendReliable(l) => l.serialize(&mut ser),
            ProtocolCommand::SendUnreliable(l) => l.serialize(&mut ser),
            ProtocolCommand::SendFragment(l) => l.serialize(&mut ser),
            ProtocolCommand::SendUnsequenced(l) => l.serialize(&mut ser),
            ProtocolCommand::BandwidthLimit(l) => l.serialize(&mut ser),
            ProtocolCommand::ThrottleConfigure(l) => l.serialize(&mut ser),
            ProtocolCommand::SendUnreliableFragment(l) => l.serialize(&mut ser),
            ProtocolCommand::Count => Ok(()),
            _ => Ok(()),
        }?;

        let size = ser.size;
        Ok((buff.freeze(), size))
    }
}