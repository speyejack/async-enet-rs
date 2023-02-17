pub mod config;
pub mod hostevents;

use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

use tokio::{
    net::ToSocketAddrs,
    select,
    sync::mpsc::{Receiver, Sender},
};

use crate::enet::protocol::DisconnectCommand;

use self::{
    config::HostConfig,
    hostevents::{HostPollEvent, HostRecvEvent, HostSendEvent},
};

use super::{
    channel::{Channel, ChannelID},
    net::{socket::ENetSocket, time::PacketTime},
    peer::{Packet, Peer, PeerID, PeerInfo, PeerRecvEvent},
    protocol::{
        AcknowledgeCommand, Command, CommandInfo, ConnectCommand, PacketFlags, PingCommand,
        ProtocolCommand, VerifyConnectCommand,
    },
    ChannelError, ENetError, Result,
};

pub struct Host {
    pub socket: ENetSocket,
    pub peers: HashMap<PeerID, PeerInfo>,
    pub config: HostConfig,

    // bandwidth_throttle_epoch: u32,
    // mtu: u32,
    pub random: random::Default,

    pub next_peer: u16,
    unack_packets: HashMap<u16, UnAckPacket>,

    pub receiver: Receiver<HostRecvEvent>,

    // Used for peer creation
    pub from_cli_tx: Sender<HostRecvEvent>,
}

#[derive(Debug, Clone)]
struct UnAckPacket {
    command: Command,
    last_sent: Duration,
    retries: usize,
    peer_id: PeerID,
}

impl UnAckPacket {
    pub fn new(command: Command) -> Self {
        Self {
            last_sent: command.info.sent_time,
            peer_id: command.info.peer_id,
            retries: 0,
            command,
        }
    }
}

impl Host {
    pub async fn create(config: HostConfig, addr: impl ToSocketAddrs) -> Result<Self> {
        let socket = tokio::net::UdpSocket::bind(addr).await?;
        let random = random::default(10);
        // TODO Set flags

        // TODO Set default host

        let peers = Default::default();
        // TODO Set default peers ... maybe
        let (from_cli_tx, from_cli_rx) = tokio::sync::mpsc::channel(100);

        Ok(Host {
            socket: ENetSocket::new(socket),
            peers,
            config,
            random,
            from_cli_tx,
            receiver: from_cli_rx,
            next_peer: 0,
            unack_packets: Default::default(),
        })
    }

    pub fn handle_connect(
        &mut self,
        addr: SocketAddr,
        connect: &ConnectCommand,
    ) -> Result<(Peer, VerifyConnectCommand)> {
        // TODO Check channel count to consts
        // TODO Check MTU to consts

        let channel_count: usize = connect.channel_count.try_into()?;
        let channel_count = channel_count.min(self.config.peer_count);

        let mtu = connect.mtu;
        let window_size = connect.window_size;

        // TODO Hande repeat connects
        let peer_id = PeerID(self.next_peer);
        self.next_peer += 1;

        let (to_cli_tx, to_cli_rx) = tokio::sync::mpsc::channel(100);
        let peer = Peer {
            id: peer_id,
            out_channel: self.from_cli_tx.clone(),
            in_channel: to_cli_rx,
        };

        // Create all channels ahead of time
        let channels = (0..channel_count as u16)
            .map(|x| (x, Channel::default()))
            .collect();

        let mut peer_info = self.peers.entry(peer_id).or_insert(PeerInfo {
            outgoing_peer_id: connect.outgoing_peer_id.into(),
            incoming_peer_id: peer_id,
            connect_id: connect.connect_id,
            outgoing_session_id: 0xFF,
            incoming_session_id: 0xFF,
            address: addr,
            channels,
            incoming_bandwidth: connect.incoming_bandwidth,
            outgoing_bandwidth: connect.outgoing_bandwidth,
            packet_throttle_interval: connect.packet_throttle_interval,
            packet_throttle_acceleration: connect.packet_throttle_acceleration,
            packet_throttle_deceleration: connect.packet_throttle_deceleration,
            event_data: connect.data,
            sender: to_cli_tx,
            incoming_reliable_sequence_number: 1,
            outgoing_reliable_sequence_number: 0,
            window_size,
            mtu,
            last_msg_time: Instant::now(),
            round_trip_time: Duration::from_millis(500),
            round_trip_time_variance: Duration::ZERO,
        });

        // Handle incoming session id
        let mut incoming_session_id = if connect.incoming_session_id == 0xFF {
            peer_info.outgoing_peer_id
        } else {
            connect.incoming_session_id.into()
        };
        if incoming_session_id == peer_info.outgoing_peer_id {
            incoming_session_id = ((incoming_session_id.0 + 1) & 3).into();
        }
        peer_info.outgoing_session_id = incoming_session_id.into();

        // Handle outgoing session id
        let mut outgoing_session_id = if connect.outgoing_session_id == 0xFF {
            peer_info.incoming_peer_id
        } else {
            connect.outgoing_session_id.into()
        };
        if outgoing_session_id == peer_info.incoming_peer_id {
            outgoing_session_id = ((outgoing_session_id.0 + 1) & 3).into();
        }
        peer_info.incoming_session_id = outgoing_session_id.into();

        let incoming_session_id = 0;
        let outgoing_session_id = 0;

        let verify = VerifyConnectCommand {
            outgoing_peer_id: peer_info.incoming_peer_id.into(),
            incoming_session_id: incoming_session_id.try_into()?,
            outgoing_session_id: outgoing_session_id.try_into()?,
            mtu,
            window_size,
            channel_count: channel_count.try_into()?,
            incoming_bandwidth: self.config.incoming_bandwidth.unwrap_or(0),
            outgoing_bandwidth: self.config.outgoing_bandwidth.unwrap_or(0),
            packet_throttle_interval: peer_info.packet_throttle_interval,
            packet_throttle_acceleration: peer_info.packet_throttle_acceleration,
            packet_throttle_deceleration: peer_info.packet_throttle_deceleration,
            connect_id: peer_info.connect_id,
        };

        Ok((peer, verify))
    }

    fn handle_ack(&mut self, peer_id: PeerID, ack: &AcknowledgeCommand) -> Result<()> {
        self.unack_packets
            .remove(&ack.received_reliable_sequence_number);
        let rtt = ack
            .received_sent_time
            .to_duration(&self.config.start_time.elapsed())
            .ok_or(ENetError::InvalidPacket())?;

        let mut peer = self.get_peer_mut(peer_id)?;

        let diff = if rtt > peer.round_trip_time {
            rtt - peer.round_trip_time
        } else {
            peer.round_trip_time - rtt
        };

        peer.round_trip_time_variance -= peer.round_trip_time_variance / 4;

        peer.round_trip_time += diff / 8;
        peer.round_trip_time_variance += diff / 4;
        Ok(())
    }

    pub async fn connect(
        &mut self,
        _addr: SocketAddr,
        _channel_count: usize,
        _data: u32,
    ) -> Result<PeerID> {
        // TODO check channel count
        todo!();
        // let new_peer = PeerInfo::new(addr);
        // let peer = PeerID(self.next_peer);
        // self.next_peer += 1;
        // self.peers.insert(peer, new_peer);
    }

    pub async fn poll(&mut self) -> Result<HostPollEvent> {
        loop {
            let event = self.poll_for_event().await;
            match event {
                Ok(HostPollEvent::NoEvent) => {}
                Ok(event) => return Ok(event),
                Err(e) => tracing::warn!("Host err: {e}"),
            }
        }
    }

    pub async fn poll_for_event(&mut self) -> Result<HostPollEvent> {
        // Receive messages and pass them off
        // Send messages
        // Resend any messages that havent been resent again

        let timed_out = self.resend_missing_packets().await?;

        if let Some(id) = timed_out {
            self.disconnect_peer(id).await?;
        }

        self.send_pings().await?;
        select! {
            incoming_command = self.socket.recv() => {
                self.handle_incoming_command(&incoming_command?).await
            }
            outgoing_event = self.receiver.recv() => {
                match outgoing_event {
                    None => todo!("Impl Peer close"),
                    Some(event) => self.handle_outgoing_command(event).await
                }

            }
            _sleep = tokio::time::sleep(Duration::from_secs(1)) => {
                Ok(HostPollEvent::NoEvent)
            }
        }
    }

    async fn disconnect_peer(&mut self, id: PeerID) -> Result<PeerInfo> {
        tracing::debug!("Disconnecting peer {id}");
        self.unack_packets.retain(|_k, v| v.peer_id != id);

        let info = self.new_command_info(id, 0xFF, PacketFlags::default())?;
        self.send(Command {
            info,
            command: DisconnectCommand { data: 0 }.into(),
        })
        .await?;

        let peer = self.peers.remove(&id);
        if let Some(peer) = peer {
            let _result = peer
                .sender
                .send(HostSendEvent {
                    event: PeerRecvEvent::Disconnect,
                    channel_id: 0xFF,
                })
                .await;
            return Ok(peer);
        }
        Err(ENetError::InvalidPeerId(id))
    }

    async fn handle_outgoing_command(&mut self, event: HostRecvEvent) -> Result<HostPollEvent> {
        let command = event.to_command(self).await?;
        self.send(command).await?;
        Ok(HostPollEvent::NoEvent)
    }

    async fn handle_incoming_command(&mut self, command: &Command) -> Result<HostPollEvent> {
        self.preprocess_packet(command).await?;

        match &command.command {
            ProtocolCommand::Connect(c) => {
                let (peer, verify_command) = self.handle_connect(command.info.addr, c)?;
                let verify_command = Command {
                    command: verify_command.into(),
                    info: self.new_command_info(peer.id, 0xFF, PacketFlags::reliable())?,
                };
                self.send(verify_command).await?;
                return Ok(HostPollEvent::Connect(peer));
            }
            ProtocolCommand::VerifyConnect(_) => todo!(),
            ProtocolCommand::Disconnect(_) => {
                self.disconnect_peer(command.info.peer_id).await?;
                return Ok(HostPollEvent::Disconnect(command.info.peer_id));
            }
            ProtocolCommand::SendReliable(_r) => self.forward_to_peer(command, true).await?,
            ProtocolCommand::SendUnreliable(_r) => self.forward_to_peer(command, false).await?,
            ProtocolCommand::Ack(r) => self.handle_ack(command.info.peer_id, r)?,

            _ => {}
        }
        Ok(HostPollEvent::NoEvent)
    }

    async fn preprocess_packet(&mut self, command: &Command) -> Result<()> {
        if let ProtocolCommand::Connect(_) = command.command {
            return Ok(());
        }

        let peer = self.get_peer_mut(command.info.peer_id)?;
        peer.last_msg_time = Instant::now();

        match &command.command {
            p if command.info.flags.reliable => {
                self.send_ack_packet(command).await?;
                if let &ProtocolCommand::SendReliable(_) = p {
                    let peer = self.get_peer_mut(command.info.peer_id)?;
                    let channel = peer.get_mut_channel(command.info.channel_id.into())?;

                    if command.info.reliable_sequence_number
                        != channel.incoming_reliable_sequence_number.wrapping_add(1)
                    {
                        return Err(ENetError::InvalidPacket());
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn forward_to_peer(&mut self, command: &Command, is_reliable: bool) -> Result<()> {
        let peer = self.get_peer(command.info.peer_id)?;

        let data = match &command.command {
            ProtocolCommand::SendReliable(r) => r.data.clone(),
            ProtocolCommand::SendUnreliable(r) => r.data.clone(),
            _ => unreachable!("Invalid packet type forwarded to peer"),
        };

        let packet = Packet {
            data,
            channel: command.info.channel_id.into(),
            flags: command.info.flags.clone(),
        };

        let output = peer
            .sender
            .send(HostSendEvent {
                event: PeerRecvEvent::Recv(packet),
                channel_id: command.info.channel_id.into(),
            })
            .await;

        let output: std::result::Result<_, ChannelError> = output.map_err(|x| x.into());
        output?;
        Ok(())
    }

    fn new_command_info(
        &mut self,
        peer_id: PeerID,
        channel_id: ChannelID,
        flags: PacketFlags,
    ) -> Result<CommandInfo> {
        let peer = self.get_peer_mut(peer_id)?;

        let reliable_sequence_number = if channel_id == 0xFF {
            peer.outgoing_reliable_sequence_number =
                peer.outgoing_reliable_sequence_number.wrapping_add(1);
            peer.outgoing_reliable_sequence_number
        } else {
            let channel = peer.get_mut_channel(channel_id)?;

            if flags.reliable {
                channel.outgoing_unreliable_sequence_number = 0;
                channel.outgoing_reliable_sequence_number =
                    channel.outgoing_reliable_sequence_number.wrapping_add(1);
                channel.outgoing_reliable_sequence_number
            } else {
                channel.outgoing_unreliable_sequence_number =
                    channel.outgoing_unreliable_sequence_number.wrapping_add(1);
                channel.outgoing_reliable_sequence_number = 1;
                channel.outgoing_reliable_sequence_number
            }
        };

        let channel_id = channel_id.try_into()?;

        let info = CommandInfo {
            addr: peer.address,
            flags,
            peer_id: peer.outgoing_peer_id.into(),
            channel_id,
            reliable_sequence_number,
            sent_time: self.config.start_time.elapsed(),
            session_id: 0,
        };
        Ok(info)
    }

    async fn send_ack_packet(&mut self, command: &Command) -> Result<()> {
        let ack_command = AcknowledgeCommand {
            received_reliable_sequence_number: command.info.reliable_sequence_number,
            received_sent_time: PacketTime::from_duration(&command.info.sent_time),
        }
        .into();

        let flags = PacketFlags::default();
        let peer = self.get_peer(command.info.peer_id)?;

        let ack_info = CommandInfo {
            addr: peer.address,
            flags,
            peer_id: peer.outgoing_peer_id.into(),
            channel_id: command.info.channel_id,
            session_id: 0,
            reliable_sequence_number: command.info.reliable_sequence_number,
            sent_time: self.config.start_time.elapsed(),
        };

        self.socket
            .send(&Command {
                command: ack_command,
                info: ack_info,
            })
            .await
    }

    async fn resend_missing_packets(&mut self) -> Result<Option<PeerID>> {
        let resend_packets = self.unack_packets.iter_mut().filter(|x| {
            self.config.start_time.elapsed() - x.1.last_sent > self.config.packet_timeout
        });

        for (_k, p) in resend_packets {
            if p.retries >= self.config.retry_count {
                return Ok(Some(p.command.info.peer_id));
            }
            self.socket.send(&p.command).await?;
            p.retries += 1;
            p.last_sent = self.config.start_time.elapsed();
        }
        Ok(None)
    }

    async fn send_pings(&mut self) -> Result<()> {
        let update_peers: Vec<_> = self
            .peers
            .iter()
            .filter(|(_, v)| v.last_msg_time.elapsed() > self.config.ping_interval)
            .map(|(k, _)| *k)
            .collect();

        for peer_id in update_peers {
            let peer = self.get_peer_mut(peer_id)?;

            peer.last_msg_time = Instant::now();

            let info = self.new_command_info(peer_id, 0xFF, PacketFlags::reliable())?;

            let ping_command = PingCommand {}.into();
            let command = Command {
                command: ping_command,
                info,
            };

            self.send(command).await?;
        }
        Ok(())
    }

    pub async fn broadcast(&mut self, event: HostRecvEvent) -> Result<()> {
        let peers: Vec<_> = self.peers.keys().map(Clone::clone).collect();
        for _peer in peers {
            let event = event.clone();
            self.handle_outgoing_command(event).await?;
        }
        Ok(())
    }

    pub(crate) async fn send(&mut self, command: Command) -> Result<()> {
        self.socket.send(&command).await?;

        if command.info.flags.reliable {
            self.unack_packets.insert(
                command.info.reliable_sequence_number,
                UnAckPacket::new(command),
            );
        }

        Ok(())
    }

    pub(crate) fn get_peer_mut(&mut self, peer_id: PeerID) -> Result<&mut PeerInfo> {
        self.peers
            .get_mut(&peer_id)
            .ok_or(ENetError::InvalidPeerId(peer_id))
    }

    pub(crate) fn get_peer(&self, peer_id: PeerID) -> Result<&PeerInfo> {
        self.peers
            .get(&peer_id)
            .ok_or(ENetError::InvalidPeerId(peer_id))
    }
}
