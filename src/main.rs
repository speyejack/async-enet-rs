use enet::{
    host::{hostevents::HostPollEvent, Host},
    Result,
};
use tracing_subscriber::EnvFilter;

use crate::enet::{host::config::HostConfig, peer::PeerRecvEvent};

mod enet;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let config = HostConfig::new(10)?;
    let mut host = Host::create(config, "0.0.0.0:1027").await?;

    let packet = host.poll_until_event().await?;
    let mut peer = match packet {
        HostPollEvent::Connect(p) => p,
        _ => panic!("First packet not connect"),
    };

    let join_handle = tokio::spawn(async move {
        loop {
            let peer_event = peer.poll().await;
            tracing::info!("Peer event: {peer_event:?}");
            if let PeerRecvEvent::Disconnect = peer_event {
                break;
            }
        }
    });

    loop {
        let event = host.poll().await?;

        println!("Got packet: {event:#x?}");
    }
}
