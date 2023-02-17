

pub type ChannelID = u16;

#[derive(Debug, Default)]
pub struct Channel {
    pub outgoing_reliable_sequence_number: u16,
    pub outgoing_unreliable_sequence_number: u16,

    pub incoming_reliable_sequence_number: u16,
    pub incoming_unreliable_sequence_number: u16,
}
