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
// #[ignore = "Using multi packet"]
async fn server_cli_single_packet() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::try_init();
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
    let dur = Duration::from_millis(1);

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
    for i in 0..200 {
        println!("Trial {i}");
        if i % 10 == 0 {
            let packet = Packet {
                data: b"hello".to_vec(),
                channel: 0,
                flags: PacketFlags::reliable(),
            };

            serv_peer.send(packet).await?;
        } else {
            let mut cli_peer = cli_host.peers().next().unwrap();
            cli_peer
                .send_packet(
                    orig_enet::Packet::new(b"hello", PacketMode::ReliableSequenced).unwrap(),
                    1,
                )
                .context("sending packet failed")?;
        }
        // println!("Sending packet");

        let mut got_data = false;
        'recv_loop: for _ in 0..10 {
            select! {
                e = serv_host.poll() => {
                    println!("Host event: {e:?}");
                },
                e = serv_peer.poll() => {
                    if let PeerRecvEvent::Recv(p) = &e {
                        if p.data == [104, 101, 108, 108, 111] {
                            // println!("Finished!");
                            got_data = true;
                            break 'recv_loop
                        }
                    }
                    // println!("Peer event: {e:?}");
                },
                _sleep = tokio::time::sleep(Duration::from_millis(1)) => {
                }
            }

            let cli_event = cli_host.service(dur.as_millis() as u32).ok().flatten();
            dbg!(&cli_event);
            drop(cli_event);
            let mut cli_peer = cli_host.peers().next().unwrap();
            let event = cli_peer.receive();
            dbg!(event);
            drop(cli_peer);
            // dbg!(&cli_event);
            // if let Some(Event::Receive {
            //     sender: _,
            //     channel_id: _,
            //     ref packet,
            // }) = cli_event
            // {
            //     if packet.data() == [104, 101, 108, 108, 111] {
            //         got_data = true;
            //         break 'recv_loop;
            //     }

            //     // test
            // }
        }
        if !got_data {
            bail!("Didnt receive expected packet")
        }
    }

    drop(cli_host);
    drop(cli_enet);
    Ok(())
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
            Data::Rewrite(v) => Data::Orig(v),
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

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        match *self {
            Data::Orig(ref x) => {
                let xs = x.shrink();
                let tagged = xs.map(Data::Orig);
                Box::new(tagged)
            }
            Data::Rewrite(ref x) => {
                let xs = x.shrink();
                let tagged = xs.map(Data::Orig);
                Box::new(tagged)
            }
        }
    }
}

// #[quickcheck_async::tokio]
// #[ignore = "Using single packet"]
// async fn server_cli_quickcheck_packet(packets: Vec<Data>) -> Result<(), anyhow::Error> {
//     println!("Starting test");
//     let _guard = TEST_MUTEX.get_or_init(|| Mutex::new(())).lock().await;
//     println!("Got guard");
//     // tracing_subscriber::fmt::try_init();
//     let serv_config = HostConfig::new(10)?;

//     let mut serv_host = Host::create_from_address(
//         serv_config,
//         SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 9001),
//     )
//     .await?;

//     let cli_enet = ENET.get_or_init(|| Enet::new().context("could not initialize ENet").unwrap());

//     let mut cli_host = cli_enet
//         .create_host::<()>(
//             None,
//             10,
//             ChannelLimit::Maximum,
//             BandwidthLimit::Unlimited,
//             BandwidthLimit::Unlimited,
//         )
//         .context("could not create host")?;

//     cli_host
//         .connect(&Address::new(Ipv4Addr::LOCALHOST, 9001), 10, 0)
//         .context("connect failed")?;

//     println!("Starting to poll");
//     cli_host.service(100).context("service failed")?;
//     let dur = Duration::from_millis(100);

//     let serv_peer = serv_host.poll_for_event(dur).await?;
//     let mut serv_peer = match serv_peer {
//         HostPollEvent::Connect(p) => p,
//         _ => panic!("Unexpected result"),
//     };

//     println!("Cli polling");

//     let mut cli_peer = loop {
//         let e = cli_host.service(100).context("service failed")?;

//         let e = match e {
//             Some(ev) => ev,
//             _ => continue,
//         };

//         println!("[client] event: {:#?}", e);

//         match e {
//             Event::Connect(ref p) => {
//                 // let new_p = p.clone();
//                 // drop(p);
//                 // break new_p;
//                 break p.clone();
//             }
//             _ => {}
//         };
//         drop(e);
//     };
//     drop(cli_peer);
//     println!("Client connected");

//     for data in packets {
//         match &data {
//             Data::Orig(v) => {
//                 let mut cli_peer = cli_host.peers().next().unwrap();
//                 cli_peer
//                     .send_packet(
//                         orig_enet::Packet::new(&v, PacketMode::ReliableSequenced).unwrap(),
//                         1,
//                     )
//                     .context("sending packet failed")?;
//                 drop(cli_peer);
//             }
//             Data::Rewrite(v) => {
//                 let packet = Packet {
//                     data: v.clone(),
//                     channel: 0,
//                     flags: PacketFlags::default(),
//                 };

//                 serv_peer.send(packet).await?;
//             }
//         }

//         let mut found_value = None;
//         'recv_loop: for _ in 0..10 {
//             select! {
//                 e = serv_host.poll() => {
//                     println!("Host event: {e:?}");
//                 },
//                 e = serv_peer.poll() => {
//                     if let PeerRecvEvent::Recv(p) = &e {
//                         found_value = Some(Data::Rewrite(p.data.clone()));
//                         break 'recv_loop
//                     }
//                     println!("Peer event: {e:?}");
//                 },
//                 _sleep = tokio::time::sleep(Duration::from_millis(1)) => {
//                 }
//             }

//             let cli_event = cli_host.service(dur.as_millis() as u32).ok().flatten();
//             if let Some(e) = cli_event {
//                 match &e {
//                     Event::Receive {
//                         sender: _,
//                         channel_id: _,
//                         packet,
//                     } => {
//                         found_value = Some(Data::Orig(packet.data().to_vec()));
//                         break 'recv_loop;
//                     }
//                     _ => bail!("Unexpected orig enet packet"),
//                 }
//             }
//         }

//         let expected = data.clone().not();
//         match found_value {
//             Some(recv) if expected == recv => {}
//             _ => {
//                 bail!("Test: {data:?}, {:?} != {found_value:?}", Some(expected))
//             }
//         }
//     }

//     Ok(())
// }
