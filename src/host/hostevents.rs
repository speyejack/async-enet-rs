use crate::{
    channel::ChannelID,
    peer::{Peer, PeerID, PeerRecvEvent, PeerSendEvent},
    protocol::{
        Command, DisconnectCommand, PacketFlags, PingCommand, ProtocolCommand, SendReliableCommand,
        SendUnreliableCommand,
    },
    error::{ENetError, Result,}
};

use super::Host;

#[derive(Debug)]
pub enum HostPollEvent {
    NoEvent,
    Connect(Peer),
    Disconnect(PeerID),
}

#[derive(Debug)]
pub struct HostSendEvent {
    pub(crate) event: PeerRecvEvent,
    pub(crate) channel_id: ChannelID,
}

#[derive(Debug, Clone)]
pub struct HostRecvEvent {
    pub(crate) event: PeerSendEvent,
    pub(crate) peer_id: PeerID,
    pub(crate) channel_id: ChannelID,
}

impl HostRecvEvent {
    pub async fn to_command(&self, host: &mut Host) -> Result<Command> {
        let peer = host.get_peer_mut(self.peer_id)?;

        let channel = peer.get_channel(self.channel_id)?;

        let (command, flags) = match &&self.event {
            PeerSendEvent::Send(p) if p.flags.reliable => (
                ProtocolCommand::SendReliable(SendReliableCommand {
                    // data_length: p.data.len().try_into()?,
                    data: p.data.clone(),
                }),
                p.flags.clone(),
            ),
            PeerSendEvent::Send(p) => (
                ProtocolCommand::SendUnreliable(SendUnreliableCommand {
                    unreliable_sequence_number: (channel.outgoing_unreliable_sequence_number + 1),
                    // data_length: p.data.len().try_into()?,
                    data: p.data.clone(),
                }),
                p.flags.clone(),
            ),
            PeerSendEvent::Ping => (
                ProtocolCommand::Ping(PingCommand {}),
                PacketFlags::reliable(),
            ),
            PeerSendEvent::Disconnect => (
                ProtocolCommand::Disconnect(DisconnectCommand { data: 0 }),
                PacketFlags::reliable(),
            ),
            PeerSendEvent::Broadcast(_) => {
                // TODO Handle broadcast - avoid recursion
                // host.broadcast(self.clone()).await?;
                // return Ok(None);
                return Err(ENetError::Other("Broadcast gave to to_command".to_string()));
            }
        };

        let info = host.new_command_info(self.peer_id, self.channel_id, flags)?;

        Ok(Command { command, info })
    }
}
