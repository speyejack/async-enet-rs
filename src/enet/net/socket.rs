use std::{collections::VecDeque, net::SocketAddr};

use bytes::{Buf, Bytes, BytesMut};
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

use super::{deserializer::EnetDeserializer, serializer::EnetSerializer, time::PacketTime};

#[derive(Debug)]
pub struct ENetSocket {
    pub socket: UdpSocket,
    buf: [u8; 100],
    incoming_queue: VecDeque<Command>,
}

impl ENetSocket {
    pub fn new(socket: UdpSocket) -> Self {
        ENetSocket {
            socket,
            buf: [0; 100],
            incoming_queue: Default::default(),
        }
    }

    pub async fn recv(&mut self) -> Result<Command> {
        if let Some(c) = self.incoming_queue.pop_front() {
            return Ok(c);
        }
        let (len, addr) = self.socket.recv_from(&mut self.buf).await?;
        self.deserialize_command(addr, len)
    }

    fn deserialize_command(&mut self, addr: SocketAddr, len: usize) -> Result<Command> {
        let buf = &self.buf[..];
        let mut deser = EnetDeserializer {
            input: buf,
            consumed: 0,
        };

        // let header = ProtocolHeader::deserialize(&mut deser)?;
        let peer_id = u16::deserialize(&mut deser)?;

        let is_compressed = ((peer_id >> 14) & 1) == 1;
        let send_time = (peer_id >> 15) > 0;
        let session_id = (peer_id >> 12) & 3;
        let peer_id = peer_id & 0xFFF;

        let sent_time = if send_time {
            u16::deserialize(&mut deser)?
        } else {
            0
        };

        let header = ProtocolHeader { peer_id, sent_time };

        while deser.consumed < len {
            let header = header.clone();

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
                is_compressed,
                send_time,
            };

            let info = CommandInfo {
                addr,
                flags,
                peer_id: header.peer_id.into(),
                channel_id: packet_type.channel_id,
                reliable_sequence_number: packet_type.reliable_sequence_number,
                sent_time: Duration::from_millis(header.sent_time.into()),
                session_id,
            };
            self.incoming_queue.push_back(Command {
                command: packet,
                info,
            });
        }

        Ok(self.incoming_queue.pop_front().unwrap())
    }

    pub async fn send(&mut self, command: &Command) -> Result<()> {
        let addr = command.info.addr;
        let (bytes, size) = self.serialize_command(command)?;

        let bytes = &bytes[0..size];
        self.socket.send_to(bytes, addr).await?;
        Ok(())
    }

    fn serialize_command(&self, p: &Command) -> Result<(Bytes, usize)> {
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
        let id_flags: u16 = p.info.session_id;
        let id_flags = id_flags << 12
            | if flags.send_time { 1 << 15 } else { 0 }
            | if flags.is_compressed { 1 << 14 } else { 0 };

        let peer_id: u16 = p.info.peer_id.into();
        let peer_id = peer_id | id_flags;

        let command_flags =
            if flags.reliable { 1 << 7 } else { 0 } | if flags.unsequenced { 1 << 6 } else { 0 };

        let command = command | command_flags;

        let command_header = ProtocolCommandHeader {
            command,
            channel_id: p.info.channel_id,
            reliable_sequence_number: p.info.reliable_sequence_number,
        };

        let sent_time = PacketTime::from_duration(&p.info.sent_time);
        let packet_header = ProtocolHeader {
            peer_id,
            sent_time: sent_time.into(),
        };

        packet_header.serialize(&mut ser)?;
        command_header.serialize(&mut ser)?;

        let _val = match &p.command {
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
