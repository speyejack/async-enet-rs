use super::protocol::Command;
use std::collections::HashMap;

pub type ChannelID = u16;

#[derive(Debug, Default)]
pub struct Channel {
    pub outgoing_reliable_sequence_number: u16,
    pub outgoing_unreliable_sequence_number: u16,

    pub incoming_reliable_sequence_number: u16,
    pub incoming_unreliable_sequence_number: u16,
    // May be replaced by msps channel
    // pub incoming_reliable_commands: HashMap<u16, Command>,
    // pub incoming_unreliable_commands: HashMap<u16, Command>,
}
