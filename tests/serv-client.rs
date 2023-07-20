use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use anyhow::{bail, Context};
use enet::{
    host::{config::HostConfig, hostevents::HostPollEvent, Host},
    peer::{Packet, PeerRecvEvent},
    protocol::PacketFlags,
};
use once_cell::sync::OnceCell;
use orig_enet::*;
use quickcheck::quickcheck;
use tokio::{select, sync::Mutex};

static TEST_MUTEX: OnceCell<Mutex<()>> = OnceCell::new();
static ENET: OnceCell<orig_enet::Enet> = OnceCell::new();

#[tokio::test]
#[ignore = "Using multi packet"]
async fn server_cli_single_packet() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::try_init();
    let serv_config = HostConfig::new(10)?;

    let mut serv_host = Host::create_from_address(
        serv_config,
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 9001),
    )
    .await?;

    let cli_enet = Enet::new().context("could not initialize ENet")?;

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

    println!("Starting to poll");
    cli_host.service(100).context("service failed")?;
    let dur = Duration::from_millis(100);

    let serv_peer = serv_host.poll_for_event(dur).await?;
    let mut serv_peer = match serv_peer {
        HostPollEvent::Connect(p) => p,
        _ => panic!("Unexpected result"),
    };

    println!("Cli polling");

    let mut cli_peer = loop {
        let e = cli_host.service(100).context("service failed")?;

        let e = match e {
            Some(ev) => ev,
            _ => continue,
        };

        println!("[client] event: {:#?}", e);

        match e {
            Event::Connect(ref p) => {
                break p.clone();
                // break;
            }
            Event::Disconnect(ref p, r) => {
                println!("connection NOT successful, peer: {:?}, reason: {}", p, r);
                panic!("Connection failed");
            }
            Event::Receive { .. } => {
                anyhow::bail!("unexpected Receive-event while waiting for connection")
            }
        };
    };
    println!("Client connected");

    // send a "hello"-like packet
    cli_peer
        .send_packet(
            orig_enet::Packet::new(b"hello", PacketMode::ReliableSequenced).unwrap(),
            1,
        )
        .context("sending packet failed")?;

    // // dbg!(&mut cli_host.service(100).unwrap());
    // let mut packet = Packet {
    //     data: "hello".as_bytes().to_vec(),
    //     channel: 0,
    //     flags: PacketFlags::default(),
    // };
    // packet.data.push(0);

    // serv_peer.send(packet).await?;
    println!("Sending packet");

    for _ in 0..10 {
        select! {
            e = serv_host.poll() => {
                println!("Host event: {e:?}");
            },
            e = serv_peer.poll() => {
                if let PeerRecvEvent::Recv(p) = &e {
                    if p.data == [104, 101, 108, 108, 111] {
                        drop(cli_host);
                        drop(cli_enet);
                        return Ok(())
                    }
                }
                println!("Peer event: {e:?}");
            },
            _sleep = tokio::time::sleep(Duration::from_millis(1)) => {
            }
        }

        let cli_event = cli_host.service(dur.as_millis() as u32).ok().flatten();
        if let Some(e) = cli_event {
            dbg!(e);
        }
    }

    // disconnect after all outgoing packets have been sent.
    // cli_peer.disconnect_later(5);

    // loop {
    //     let e = cli_host.service(1000).context("service failed")?;
    //     println!("received event: {:#?}", e);
    // }
    bail!("Didnt receive expected packet")
}

#[quickcheck_async::tokio]
async fn server_cli_quickcheck_packet() -> Result<(), anyhow::Error> {
    println!("Starting test");
    let _guard = TEST_MUTEX.get_or_init(|| Mutex::new(())).lock().await;
    println!("Got guard");
    // tracing_subscriber::fmt::try_init();
    let serv_config = HostConfig::new(10)?;

    let mut serv_host = Host::create_from_address(
        serv_config,
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 9001),
    )
    .await?;

    let cli_enet = ENET.get_or_init(|| Enet::new().context("could not initialize ENet").unwrap());

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

    println!("Starting to poll");
    cli_host.service(100).context("service failed")?;
    let dur = Duration::from_millis(100);

    let serv_peer = serv_host.poll_for_event(dur).await?;
    let mut serv_peer = match serv_peer {
        HostPollEvent::Connect(p) => p,
        _ => panic!("Unexpected result"),
    };

    println!("Cli polling");

    let mut cli_peer = loop {
        let e = cli_host.service(100).context("service failed")?;

        let e = match e {
            Some(ev) => ev,
            _ => continue,
        };

        println!("[client] event: {:#?}", e);

        match e {
            Event::Connect(ref p) => {
                break p.clone();
                // break;
            }
            Event::Disconnect(ref p, r) => {
                println!("connection NOT successful, peer: {:?}, reason: {}", p, r);
                panic!("Connection failed");
            }
            Event::Receive { .. } => {
                anyhow::bail!("unexpected Receive-event while waiting for connection")
            }
        };
    };
    println!("Client connected");

    // send a "hello"-like packet
    cli_peer
        .send_packet(
            orig_enet::Packet::new(b"hello", PacketMode::ReliableSequenced).unwrap(),
            1,
        )
        .context("sending packet failed")?;

    // // dbg!(&mut cli_host.service(100).unwrap());
    // let mut packet = Packet {
    //     data: "hello".as_bytes().to_vec(),
    //     channel: 0,
    //     flags: PacketFlags::default(),
    // };
    // packet.data.push(0);

    // serv_peer.send(packet).await?;
    println!("Sending packet");

    for _ in 0..10 {
        select! {
            e = serv_host.poll() => {
                println!("Host event: {e:?}");
            },
            e = serv_peer.poll() => {
                if let PeerRecvEvent::Recv(p) = &e {
                    if p.data == [104, 101, 108, 108, 111] {
                        println!("Got expected data");
                        return Ok(())
                    }
                }
                println!("Peer event: {e:?}");
            },
            _sleep = tokio::time::sleep(Duration::from_millis(1)) => {
            }
        }

        let cli_event = cli_host.service(dur.as_millis() as u32).ok().flatten();
        if let Some(e) = cli_event {
            dbg!(e);
        }
    }

    // disconnect after all outgoing packets have been sent.
    // cli_peer.disconnect_later(5);

    // loop {
    //     let e = cli_host.service(1000).context("service failed")?;
    //     println!("received event: {:#?}", e);
    // }
    bail!("Didnt receive expected packet")
}
