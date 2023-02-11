use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{ToSocketAddrs, UdpSocket},
    select,
    sync::mpsc::{Receiver, Sender},
};

use super::{
    channel::{Channel, ChannelID},
    deserializer::EnetDeserializer,
    peer::{Packet, Peer, PeerEvent, PeerID, PeerInfo, PeerSendEvent},
    protocol::{
        AcknowledgeCommand, BandwidthLimitCommand, Command, CommandHeader, CommandInfo,
        ConnectCommand, DisconnectCommand, PacketFlags, PingCommand, ProtocolCommand,
        ProtocolCommandHeader, SendFragmentCommand, SendReliableCommand, SendUnreliableCommand,
        SendUnsequencedCommand, ThrottleConfigureCommand, VerifyConnectCommand,
    },
    serializer::EnetSerializer,
    ENetError, Result,
};

pub struct HostConfig {
    pub peer_count: usize,
    pub channel_limit: Option<usize>,
    pub incoming_bandwidth: Option<u32>,
    pub outgoing_bandwidth: Option<u32>,
}

impl HostConfig {
    pub fn new(peer_count: usize) -> Result<Self> {
        Ok(HostConfig {
            peer_count,
            channel_limit: None,
            incoming_bandwidth: None,
            outgoing_bandwidth: None,
        })
    }
}

pub struct Host {
    pub socket: UdpSocket,
    pub peers: HashMap<PeerID, PeerInfo>,
    pub config: HostConfig,
    pub start_time: Instant,

    // bandwidth_throttle_epoch: u32,
    // mtu: u32,
    pub random: random::Default,

    pub next_peer: u16,
    pub unack_packets: HashMap<u16, Command>,

    pub receiver: Receiver<(PeerSendEvent, PeerID, ChannelID)>,
    pub sender: Sender<(PeerSendEvent, PeerID, ChannelID)>,
}

#[derive(Debug)]
pub enum HostEvent {
    Event(PeerEvent),
    NewConnection(Peer),
}

impl Host {
    pub async fn create(config: HostConfig, addr: impl ToSocketAddrs) -> Result<Self> {
        let socket = tokio::net::UdpSocket::bind(addr).await?;
        let random = random::default(10);
        // TODO Set flags

        // TODO Set default host

        let peers = Default::default();
        // TODO Set default peers ... maybe
        let (sender, receiver) = tokio::sync::mpsc::channel(100);

        Ok(Host {
            socket,
            peers,
            config,
            random,
            sender,
            receiver,
            next_peer: 0,
            start_time: Instant::now(),
            unack_packets: Default::default(),
        })
    }

    pub fn handle_connect(
        &mut self,
        addr: SocketAddr,
        connect: ConnectCommand,
    ) -> Result<(PeerID, VerifyConnectCommand)> {
        // TODO Check channel count to consts
        // TODO Check MTU to consts

        let channel_count: usize = connect.channel_count.try_into()?;
        let channel_count = channel_count.min(self.config.peer_count);

        let mtu = connect.mtu;
        let window_size = connect.window_size;

        // TODO Hande repeat connects
        let peer_id = PeerID(self.next_peer);
        self.next_peer += 1;

        let mut peer = self.peers.entry(peer_id).or_insert(PeerInfo {
            outgoing_peer_id: connect.outgoing_peer_id.into(),
            incoming_peer_id: self.next_peer.into(),
            connect_id: connect.connect_id,
            outgoing_session_id: 0xFF,
            incoming_session_id: 0xFF,
            address: addr,
            channels: Default::default(),
            incoming_bandwidth: connect.incoming_bandwidth,
            outgoing_bandwidth: connect.outgoing_bandwidth,
            packet_throttle_interval: connect.packet_throttle_interval,
            packet_throttle_acceleration: connect.packet_throttle_acceleration,
            packet_throttle_deceleration: connect.packet_throttle_deceleration,
            event_data: connect.data,
            window_size,
            mtu,
        });

        // Handle incoming session id
        let mut incoming_session_id = if connect.incoming_session_id == 0xFF {
            peer.outgoing_peer_id
        } else {
            connect.incoming_session_id.into()
        };
        if incoming_session_id == peer.outgoing_peer_id {
            incoming_session_id = ((incoming_session_id.0 + 1) & 3).into();
        }
        peer.outgoing_session_id = incoming_session_id.into();

        // Handle outgoing session id
        let mut outgoing_session_id = if connect.outgoing_session_id == 0xFF {
            peer.incoming_peer_id
        } else {
            connect.outgoing_session_id.into()
        };
        if outgoing_session_id == peer.incoming_peer_id {
            outgoing_session_id = ((outgoing_session_id.0 + 1) & 3).into();
        }
        peer.incoming_session_id = outgoing_session_id.into();

        let verify = VerifyConnectCommand {
            outgoing_peer_id: peer.incoming_peer_id.into(),
            incoming_session_id: incoming_session_id.try_into()?,
            outgoing_session_id: outgoing_session_id.try_into()?,
            mtu,
            window_size,
            channel_count: channel_count.try_into()?,
            incoming_bandwidth: self.config.incoming_bandwidth.unwrap_or(0),
            outgoing_bandwidth: self.config.outgoing_bandwidth.unwrap_or(0),
            packet_throttle_interval: peer.packet_throttle_interval,
            packet_throttle_acceleration: peer.packet_throttle_acceleration,
            packet_throttle_deceleration: peer.packet_throttle_deceleration,
            connect_id: peer.connect_id,
        };

        Ok((peer_id, verify))
    }

    pub async fn connect(
        &mut self,
        addr: SocketAddr,
        channel_count: usize,
        data: u32,
    ) -> Result<PeerID> {
        // TODO check channel count
        todo!();
        // let new_peer = PeerInfo::new(addr);
        // let peer = PeerID(self.next_peer);
        // self.next_peer += 1;
        // self.peers.insert(peer, new_peer);
    }

    pub async fn poll(&mut self) -> Result<Option<HostEvent>> {
        // Receive messages and pass them off
        // Send messages
        // Resend any messages that havent been resent again

        self.resend_missing_packets().await?;
        let data = self.retrieve_data().await?;
        let ret = match data {
            RetrievedData::SocketIncoming((addr, buffer)) => {
                let data = self.recv(buffer, addr).await?;
                data.map(|x| HostEvent::Event())
            }
            RetrievedData::PeerOutgoing(p) => {
                // TODO Handle peer disconnect
                if let Some(p) = p {
                    self.send_event(p.0, p.1, p.2).await?;
                }
                Ok(None)
            }
            RetrievedData::TimeOut => Ok(None),
        };

        ret
    }

    pub fn cleanup_peer(&mut self) {
        todo!("Make a cleanup method")
    }

    async fn retrieve_data(&mut self) -> Result<RetrievedData> {
        let mut buf = [0; 100];
        let ret = select! {
           val = self.socket.recv_from(&mut buf) => {
               let (_, addr) = val?;
               Ok(RetrievedData::SocketIncoming((addr, buf)))
           }
           send_msg = self.receiver.recv() => {
               Ok(RetrievedData::PeerOutgoing(send_msg))
           }
           // sleep = tokio::time::sleep(Duration::from_secs(1)) => {
           //     Ok(RetrievedData::TimeOut)
           // }
        };
        ret
    }

    pub async fn recv(&mut self, buff: [u8; 100], addr: SocketAddr) -> Result<Option<HostEvent>> {
        let p = self.recv_packet(buff, addr).await?;
        match p.command {
            ProtocolCommand::Connect(c) => {
                self.handle_connect(p.metadata.addr, c)?;
                Ok(None)
            }
            // ProtocolCommand::VerifyConnect(_) => todo!(),
            ProtocolCommand::Disconnect(_) => Ok(Some((PeerEvent::Disconnect, p.metadata))),
            // ProtocolCommand::Ping(_) => todo!(),
            ProtocolCommand::SendReliable(r) => Ok(Some((
                PeerEvent::Recv(Packet {
                    data: r.data,
                    channel: p.metadata.channel_id.into(),
                    flags: p.metadata.flags.clone(),
                }),
                p.metadata,
            ))),
            ProtocolCommand::SendUnreliable(r) => Ok(Some((
                PeerEvent::Recv(Packet {
                    data: r.data,
                    channel: p.metadata.channel_id.into(),
                    flags: p.metadata.flags.clone(),
                }),
                p.metadata,
            ))),
            // ProtocolCommand::SendFragment(_) => todo!(),
            // ProtocolCommand::SendUnsequenced(_) => todo!(),
            // ProtocolCommand::BandwidthLimit(_) => todo!(),
            // ProtocolCommand::ThrottleConfigure(_) => todo!(),
            // ProtocolCommand::SendUnreliableFragment(_) => todo!(),
            // ProtocolCommand::Count => todo!(),
            _ => Ok(None),
        }
    }

    pub(crate) async fn recv_packet(
        &mut self,
        buff: [u8; 100],
        addr: SocketAddr,
    ) -> Result<Command> {
        let (packet, info) = self.process_packet(&buff, addr)?;

        if info.flags.reliable {
            let command = AcknowledgeCommand {
                received_reliable_sequence_number: info.reliable_sequence_number,
                received_sent_time: info.sent_time.as_millis() as u16,
            }
            .into();

            let info = CommandInfo {
                addr,
                flags: Default::default(),
                peer_id: info.peer_id,
                channel_id: info.channel_id,
                reliable_sequence_number: info.reliable_sequence_number,
                sent_time: self.start_time.elapsed(),
            };

            self.send(Command {
                command,
                metadata: info,
            })
            .await?;
        }

        Ok(Command {
            command: packet,
            metadata: info,
        })
    }

    pub(crate) fn process_packet(
        &mut self,
        buf: &[u8],
        addr: SocketAddr,
    ) -> Result<(ProtocolCommand, CommandInfo)> {
        let mut deser = EnetDeserializer { input: buf };

        let header = CommandHeader::deserialize(&mut deser)?;
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

        Ok((packet, info))
    }

    async fn resend_missing_packets(&mut self) -> Result<()> {
        // TODO Remove this clone
        let resend_packets: Vec<_> = self
            .unack_packets
            .iter()
            .filter(|x| self.start_time.elapsed() - x.1.metadata.sent_time > Duration::from_secs(1))
            .map(|x| x.1.clone())
            .collect();

        for p in resend_packets {
            self.send_command(&p).await?;
        }
        Ok(())
    }

    fn get_peer(&self, peer_id: PeerID) -> Result<&PeerInfo> {
        let outgoing_peer = self
            .peers
            .get(&peer_id)
            .ok_or(ENetError::InvalidPeerId(peer_id))?;
        Ok(outgoing_peer)
    }

    fn create_packet_header(&self, local_peer_id: PeerID) -> Result<CommandHeader> {
        let outgoing_peer = self
            .peers
            .get(&local_peer_id)
            .ok_or(ENetError::InvalidPeerId(local_peer_id))?;
        let outgoing_peer_id = outgoing_peer.outgoing_peer_id;
        let time = self.start_time.elapsed();

        Ok(CommandHeader {
            peer_id: outgoing_peer_id.into(),
            sent_time: time.as_millis() as u16,
        })
    }

    pub(crate) async fn send_event(
        &mut self,
        event: PeerSendEvent,
        peer_id: PeerID,
        channel_id: ChannelID,
    ) -> Result<()> {
        let command = self.create_command(event, peer_id, channel_id)?;
        self.send(command).await?;

        Ok(())
    }

    pub(crate) fn create_command(
        &self,
        event: PeerSendEvent,
        peer_id: PeerID,
        channel_id: ChannelID,
    ) -> Result<Command> {
        let peer = self.get_peer(peer_id)?;

        let channel: &Channel = peer
            .channels
            .get::<usize>(channel_id.into())
            .ok_or(ENetError::InvalidChannelId(channel_id))?;

        let (command, flags) = match event {
            PeerSendEvent::Send(p) if p.flags.reliable => (
                ProtocolCommand::SendReliable(SendReliableCommand {
                    // data_length: p.data.len().try_into()?,
                    data: p.data,
                }),
                p.flags,
            ),
            PeerSendEvent::Send(p) => (
                ProtocolCommand::SendUnreliable(SendUnreliableCommand {
                    unreliable_sequence_number: channel.outgoing_unreliable_sequence_number,
                    // data_length: p.data.len().try_into()?,
                    data: p.data,
                }),
                p.flags,
            ),
            PeerSendEvent::Ping => (
                ProtocolCommand::Ping(PingCommand {}),
                PacketFlags::reliable(),
            ),
            PeerSendEvent::Disconnect => (
                ProtocolCommand::Disconnect(DisconnectCommand { data: 0 }),
                PacketFlags::reliable(),
            ),
        };

        let info = CommandInfo {
            addr: peer.address,
            flags,
            peer_id: peer.outgoing_peer_id,
            channel_id: channel_id.try_into()?,
            reliable_sequence_number: channel.outgoing_reliable_sequence_number,
            sent_time: self.start_time.elapsed(),
        };

        Ok(Command {
            command,
            metadata: info,
        })
    }

    pub async fn broadcast(&mut self, event: PeerSendEvent, channel: ChannelID) -> Result<()> {
        let peers: Vec<_> = self.peers.keys().map(Clone::clone).collect();
        for peer in peers {
            let event = event.clone();
            self.send_event(event, peer, channel).await?;
        }
        Ok(())
    }

    pub(crate) async fn send(&mut self, command: Command) -> Result<()> {
        self.send_command(&command).await?;

        if command.metadata.flags.reliable {
            self.unack_packets
                .insert(command.metadata.reliable_sequence_number, command);
        }

        Ok(())
    }

    pub(crate) async fn send_command(&mut self, command: &Command) -> Result<()> {
        let addr = command.metadata.addr;
        let (bytes, size) = self.construct_packet(&command)?;

        println!("Sending packet: {size} => {bytes:?}");
        self.socket.send_to(&bytes[0..size], addr).await?;
        Ok(())
    }

    pub(crate) fn construct_packet(&mut self, mut p: &Command) -> Result<(Bytes, usize)> {
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

        let flags = &p.metadata.flags;
        let command_flags =
            if flags.reliable { 1 << 7 } else { 0 } | if flags.unsequenced { 1 << 6 } else { 0 };

        let command = command | command_flags;

        let command_header = ProtocolCommandHeader {
            command,
            channel_id: p.metadata.channel_id,
            reliable_sequence_number: p.metadata.reliable_sequence_number,
        };

        let packet_header = CommandHeader {
            peer_id: p.metadata.peer_id.into(),
            sent_time: p.metadata.sent_time.as_millis() as u16,
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

enum RetrievedData {
    SocketIncoming((SocketAddr, [u8; 100])),
    PeerOutgoing(Option<(PeerSendEvent, PeerID, ChannelID)>),
    TimeOut,
}
