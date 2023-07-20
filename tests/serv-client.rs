use std::{
    net::{Ipv4Addr, SocketAddr},
    ops::Not,
    time::Duration,
};

use anyhow::{bail, Context};
use enet::{
    host::{config::HostConfig, hostevents::HostPollEvent, Host},
    peer::{Packet, Peer, PeerRecvEvent},
    protocol::PacketFlags,
};
use once_cell::sync::OnceCell;
use orig_enet::*;
use quickcheck::Arbitrary;
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

#[derive(Debug, PartialEq, Eq, Clone)]
enum Data {
    Orig(Vec<u8>),
    Rewrite(Vec<u8>),
}

impl Data {
    pub fn data(&self) -> &[u8] {
        match self {
            Data::Orig(v) => v,
            Data::Rewrite(v) => v,
        }
    }
}

impl Not for Data {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Data::Orig(v) => Data::Rewrite(v),
            Data::Rewrite(v) => Data::Rewrite(v),
        }
    }
}

impl Arbitrary for Data {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let out = g.choose(&[0, 1]).unwrap();
        let data: Vec<u8> = Arbitrary::arbitrary(g);
        match out {
            0 => Data::Orig(data),
            1 => Data::Rewrite(data),
            _ => unreachable!(),
        }
    }
}

#[quickcheck_async::tokio]
async fn server_cli_quickcheck_packet(packets: Vec<Data>) -> Result<(), anyhow::Error> {
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

        match &e {
            Event::Connect(p) => {
                let new_p = p.clone();
                drop(p);
                break new_p;
                // break;
            }
            _ => {} // Event::Disconnect(p, r) => {
                    //     println!("connection NOT successful, peer: {:?}, reason: {}", p, r);
                    //     panic!("Connection failed");
                    // }
                    // Event::Receive { .. } => {
                    //     anyhow::bail!("unexpected Receive-event while waiting for connection")
                    // }
        };
        drop(e);
    };
    println!("Client connected");
    drop(cli_peer);

    // send a "hello"-like packet

    for data in packets {
        match &data {
            Data::Orig(v) => {
                cli_peer
                    .send_packet(
                        orig_enet::Packet::new(&v, PacketMode::ReliableSequenced).unwrap(),
                        1,
                    )
                    .context("sending packet failed")?;
            }
            Data::Rewrite(v) => {
                let packet = Packet {
                    data: v.clone(),
                    channel: 0,
                    flags: PacketFlags::default(),
                };

                serv_peer.send(packet);
            }
        }

        let recv = recv_packet(
            &mut serv_host,
            &mut serv_peer,
            &mut cli_host,
            // &mut cli_peer,
            dur,
        )
        .await?;

        let not_recv = !recv;
        if data != not_recv {
            bail!("{data:?} != {not_recv:?}")
        }
    }

    // for data in packets {
    //     match data {
    //         Data::Orig(v) => {
    //             cli_peer
    //                 .send_packet(
    //                     orig_enet::Packet::new(&v, PacketMode::ReliableSequenced).unwrap(),
    //                     1,
    //                 )
    //                 .context("sending packet failed")?;

    //             let recv = recv_packet(
    //                 &mut serv_host,
    //                 &mut serv_peer,
    //                 &mut cli_host,
    //                 &mut cli_peer,
    //                 dur,
    //             )
    //             .await?;
    //             match recv {
    //                 Data::Orig(nv) => {
    //                     bail!("Original enet recv packet when rewrite was expected");
    //                 }
    //                 Data::Rewrite(nv) => {
    //                     if v != nv {
    //                         bail!("Data difference: Orig {v:?} != Rewrite {nv:?}");
    //                     }
    //                 }
    //             }
    //         }
    //         Data::Rewrite(v) => {
    //             let packet = Packet {
    //                 data: v.clone(),
    //                 channel: 0,
    //                 flags: PacketFlags::default(),
    //             };

    //             serv_peer.send(packet);

    //             let recv = recv_packet(
    //                 &mut serv_host,
    //                 &mut serv_peer,
    //                 &mut cli_host,
    //                 &mut cli_peer,
    //                 dur,
    //             )
    //             .await?;
    //             match recv {
    //                 Data::Rewrite(nv) => {
    //                     bail!("Rewrite recv packet when original enet was expected");
    //                 }
    //                 Data::Orig(nv) => {
    //                     if v != nv {
    //                         bail!("Data difference: Rewrite {v:?} != Orig {nv:?}");
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }
    // // dbg!(&mut cli_host.service(100).unwrap());

    // serv_peer.send(packet).await?;

    println!("Sending packet");
    Ok(())
}

async fn recv_packet<'a>(
    serv_host: &mut Host,
    serv_peer: &mut Peer,
    cli_host: &'a mut orig_enet::Host<()>,
    // cli_peer: &mut orig_enet::Peer<'a, ()>,
    dur: Duration,
) -> Result<Data, anyhow::Error> {
    for _ in 0..10 {
        select! {
            e = serv_host.poll() => {
                println!("Host event: {e:?}");
            },
            e = serv_peer.poll() => {
                if let PeerRecvEvent::Recv(p) = &e {
                    return Ok(Data::Rewrite(p.data.clone()))
                }
                println!("Peer event: {e:?}");
            },
            _sleep = tokio::time::sleep(Duration::from_millis(1)) => {
            }
        }

        let cli_event = cli_host.service(dur.as_millis() as u32).ok().flatten();
        if let Some(e) = cli_event {
            match &e {
                Event::Receive {
                    sender,
                    channel_id,
                    packet,
                } => return Ok(Data::Orig(packet.data().to_vec())),
                _ => bail!("Unexpected orig enet packet"),
            }
        }
    }

    bail!("Didnt receive expected packet")
}
