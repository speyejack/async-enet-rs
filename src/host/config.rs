use std::time::{Duration, Instant};

use crate::error::Result;

#[derive(Debug)]
pub struct HostConfig {
    pub peer_count: usize,
    pub channel_limit: Option<usize>,
    pub incoming_bandwidth: Option<u32>,
    pub outgoing_bandwidth: Option<u32>,
    pub start_time: Instant,
    pub retry_count: usize,
    pub packet_timeout: Duration,
    pub ping_interval: Duration,
}

impl HostConfig {
    pub fn new(peer_count: usize) -> Result<Self> {
        Ok(HostConfig {
            peer_count,
            channel_limit: None,
            incoming_bandwidth: None,
            outgoing_bandwidth: None,
            start_time: Instant::now(),
            packet_timeout: Duration::from_secs(1),
            retry_count: 5,
            ping_interval: Duration::from_millis(500),
        })
    }
}
