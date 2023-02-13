use std::time::Instant;

use crate::enet::Result;

pub struct HostConfig {
    pub peer_count: usize,
    pub channel_limit: Option<usize>,
    pub incoming_bandwidth: Option<u32>,
    pub outgoing_bandwidth: Option<u32>,
    pub start_time: Instant,
}

impl HostConfig {
    pub fn new(peer_count: usize) -> Result<Self> {
        Ok(HostConfig {
            peer_count,
            channel_limit: None,
            incoming_bandwidth: None,
            outgoing_bandwidth: None,
            start_time: Instant::now(),
        })
    }
}
