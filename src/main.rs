use enet::{
    host::{hostevents::HostPollEvent, Host},
    peer::{Peer, PeerSendEvent},
    ENetError, Result,
};
use tracing_subscriber::EnvFilter;

use crate::enet::{host::config::HostConfig, peer::PeerRecvEvent};

mod enet;

async fn poll_till_peer(host: &mut Host) -> Result<Peer> {
    loop {
        let packet = host.poll_until_event().await?;
        match packet {
            HostPollEvent::Connect(peer) => return Ok(peer),
            _ => {}
        };
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let config = HostConfig::new(10)?;
    let mut host = Host::create(config, "0.0.0.0:1027").await?;

    let mut peer_1 = poll_till_peer(&mut host).await?;
    let mut peer_2 = poll_till_peer(&mut host).await?;

    tokio::spawn(async move {
        loop {
            tokio::select! {
                packet = peer_1.poll() => {
                    match packet {
                        PeerRecvEvent::Recv(p) => peer_2.send(p).await?,
                        PeerRecvEvent::Disconnect => return Ok::<(), ENetError>(()),
                    }
                },
                packet = peer_2.poll() => {
                    match packet {
                        PeerRecvEvent::Recv(p) => peer_1.send(p).await?,
                        PeerRecvEvent::Disconnect => return Ok(()),
                    }
                }
            }
        }
    });

    loop {
        let event = host.poll_until_event().await?;

        tracing::info!("Got packet: {event:#x?}");
    }
}
