#![no_main]

use anyhow::{bail, Context};
use enet::{
    host::{config::HostConfig, hostevents::HostPollEvent, Host},
    peer::{Packet, PeerRecvEvent},
    protocol::PacketFlags,
};
use libfuzzer_sys::fuzz_target;
use once_cell::sync::OnceCell;
use orig_enet::*;
use std::{
    net::{Ipv4Addr, SocketAddr},
    ops::Not,
    time::Duration,
};
use tokio::select;

static ENET: OnceCell<orig_enet::Enet> = OnceCell::new();

#[derive(Debug, PartialEq, Eq, Clone, arbitrary::Arbitrary)]
enum Data {
    Orig(Vec<u8>),
    Rewrite(Vec<u8>),
}

impl Not for Data {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Data::Orig(v) => Data::Rewrite(v),
            Data::Rewrite(v) => Data::Orig(v),
        }
    }
}

async fn server_cli_packets(packets: Vec<Data>) -> Result<(), anyhow::Error> {
    #[cfg(fuzzing_repro)]
    tracing_subscriber::fmt::try_init();

    let serv_config = HostConfig::new(10)?;

    let mut serv_host = Host::create_from_address(
        serv_config,
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 9001),
    )
    .await?;

    let cli_enet = ENET.get_or_init(|| Enet::new().context("could not initialize ENet").unwrap());
    // let cli_enet = Enet::new().context("count not initialize ENet").unwrap();

    let mut cli_host = cli_enet
        .create_host::<()>(
            None,
            10,
            ChannelLimit::Maximum,
            BandwidthLimit::Unlimited,
            BandwidthLimit::Unlimited,
        )
        .context("could not create host")?;

    cli_host
        .connect(&Address::new(Ipv4Addr::LOCALHOST, 9001), 10, 0)
        .context("connect failed")?;

    tracing::info!("Starting to poll");
    cli_host.service(100).context("service failed")?;
    let dur = Duration::from_millis(100);

    let serv_peer = serv_host.poll_for_event(dur).await?;
    let mut serv_peer = match serv_peer {
        HostPollEvent::Connect(p) => p,
        _ => panic!("Unexpected result"),
    };

    tracing::info!("Cli polling");

    let _cli_peer = loop {
        let e = cli_host.service(100).context("service failed")?;

        let e = match e {
            Some(ev) => ev,
            _ => continue,
        };

        tracing::info!("[client] event: {:#?}", e);

        match e {
            Event::Connect(ref p) => {
                // let new_p = p.clone();
                // drop(p);
                // break new_p;
                break p.clone();
            }
            _ => {}
        };
        drop(e);
    };
    tracing::info!("Client connected");

    for data in packets {
        match &data {
            Data::Orig(v) => {
                let mut cli_peer = cli_host.peers().next().unwrap();
                cli_peer
                    .send_packet(
                        orig_enet::Packet::new(&v, PacketMode::ReliableSequenced).unwrap(),
                        1,
                    )
                    .context("sending packet failed")?;
                drop(cli_peer);
            }
            Data::Rewrite(v) => {
                let packet = Packet {
                    data: v.clone(),
                    channel: 0,
                    flags: PacketFlags::reliable(),
                };

                serv_peer.send(packet).await?;
            }
        }

        let mut found_value = None;
        'recv_loop: for _ in 0..10 {
            select! {
                e = serv_host.poll() => {
                    tracing::info!("Host event: {e:?}");
                },
                e = serv_peer.poll() => {
                    if let PeerRecvEvent::Recv(p) = &e {
                        found_value = Some(Data::Rewrite(p.data.clone()));
                        break 'recv_loop
                    }
                    tracing::info!("Peer event: {e:?}");
                },
                _sleep = tokio::time::sleep(Duration::from_millis(1)) => {
                }
            }

            let cli_event = cli_host.service(dur.as_millis() as u32).ok().flatten();
            if let Some(e) = cli_event {
                match &e {
                    Event::Receive {
                        sender: _,
                        channel_id: _,
                        packet,
                    } => {
                        found_value = Some(Data::Orig(packet.data().to_vec()));
                        break 'recv_loop;
                    }
                    _ => bail!("Unexpected orig enet packet"),
                }
            }
        }

        let expected = data.clone().not();
        match found_value {
            Some(recv) if expected == recv => {}
            _ => {
                bail!("Test: {data:?}, {:?} != {found_value:?}", Some(expected))
            }
        }
    }

    Ok(())
}

fuzz_target!(|data: Vec<Data>| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { server_cli_packets(data).await.unwrap() });
    // fuzzed code goes here
});
