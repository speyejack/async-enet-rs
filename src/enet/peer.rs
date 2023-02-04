use super::ENetAddress;

pub struct Peer {
    outgoing_peer_id: u16,
    incoming_peer_id: u16,
    connect_id: u16,
    outgoing_session_id: u16,
    incoming_session_id: u16,
    address: ENetAddress,
}

pub struct Channel {
    outgoing_reliable_sequence_number: u16,
    outgoing_unreliable_sequence_number: u16,

    incoming_reliable_sequence_number: u16,
    incoming_unreliable_sequence_number: u16,
}

impl Peer {
    pub fn new(address: ENetAddress) -> Peer {
        todo!()
        // Peer {
        //     outgoing_peer_id: todo!(),
        //     incoming_peer_id: todo!(),
        //     connect_id: todo!(),
        //     outgoing_session_id: todo!(),
        //     incoming_session_id: todo!(),
        //     address,
        // }
    }
}
