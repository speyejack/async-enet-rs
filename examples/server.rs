use anyhow::Context;
use enet::{
    host::{config::HostConfig, hostevents::HostPollEvent, Host},
    peer::PeerRecvEvent,
};
use std::net::{Ipv4Addr, SocketAddr};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let serv_config = HostConfig::new(2)?;
    let mut serv_host = Host::create_from_address(
        serv_config,
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 9001),
    )
    .await?;

    let event = serv_host.poll().await?;
    dbg!(&event);

    let mut serv_peer = match event {
        HostPollEvent::Connect(p) => p,
        _ => anyhow::bail!("Weird event {event:?}"),
    };

    tracing::info!("Got client, beginning loop");
    loop {
        tokio::select! {
            e = serv_peer.poll() => {
                tracing::info!("Got peer event: {e:?}");
                if let PeerRecvEvent::Disconnect = e {break}
            }
            e = serv_host.poll() => {
                tracing::info!("Got host event: {e:?}");
            }
        }
    }

    Ok(())
}
