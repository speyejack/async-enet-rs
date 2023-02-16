use enet::{
    host::{hostevents::HostPollEvent, Host},
    peer::{Packet, Peer, PeerSendEvent},
    protocol::PacketFlags,
    ENetError, Result,
};
use tracing_subscriber::EnvFilter;

use crate::enet::{host::config::HostConfig, peer::PeerRecvEvent};

mod enet;

async fn poll_till_peer(host: &mut Host) -> Result<Peer> {
    loop {
        let packet = host.poll_until_event().await?;
        if let HostPollEvent::Connect(mut peer) = packet {
            peer.send(Packet {
                data: "Welcome!\0".as_bytes().to_vec(),
                channel: 0,
                flags: PacketFlags::reliable(),
            })
            .await?;
            return Ok(peer);
        };
    }
}

async fn send_msg(peer: &mut Peer, mut p: Packet) {
    let s = String::from_utf8(p.data).unwrap();
    let trimmed = s.trim_end_matches('\0');
    let msg = format!("{} has connected", trimmed);
    tracing::info!("Msg: {msg}");
    p.flags = PacketFlags::default();
    p.data = msg.as_bytes().to_vec();
    p.data.push(0);
    peer.send(p).await.unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_level(true)
        .without_time()
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
                        PeerRecvEvent::Recv(mut p) => {
                            send_msg(&mut peer_1, p.clone()).await;
                            send_msg(&mut peer_2, p).await;
                        }
                        PeerRecvEvent::Disconnect => return Ok::<(), ENetError>(()),
                    }
                },
                packet = peer_2.poll() => {
                    match packet {
                        PeerRecvEvent::Recv(mut p) => {
                            send_msg(&mut peer_1, p.clone()).await;
                            send_msg(&mut peer_2, p).await;
                        }
                        PeerRecvEvent::Disconnect => return Ok::<(), ENetError>(()),
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
