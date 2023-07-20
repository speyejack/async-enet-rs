// use enet::{
//     error::{ENetError, Result},
//     host::{hostevents::HostPollEvent, Host},
//     peer::{Packet, Peer},
//     protocol::PacketFlags,
// };
// use tracing_subscriber::EnvFilter;

// use enet::{host::config::HostConfig, peer::PeerRecvEvent};

// async fn poll_till_peer(host: &mut Host) -> Result<Peer> {
//     loop {
//         let packet = host.poll().await?;
//         if let HostPollEvent::Connect(mut peer) = packet {
//             peer.send(Packet {
//                 data: "Welcome!\0".as_bytes().to_vec(),
//                 channel: 0,
//                 flags: PacketFlags::reliable(),
//             })
//             .await?;
//             return Ok(peer);
//         };
//     }
// }

// async fn send_msg(peer: &mut Peer, mut p: Packet) {
//     let s = String::from_utf8(p.data).unwrap();
//     let trimmed = s.trim_end_matches('\0');
//     let msg = format!("{} has connected", trimmed);
//     tracing::info!("Msg: {msg}");
//     p.flags = PacketFlags::default();
//     p.data = msg.as_bytes().to_vec();
//     p.data.push(0);
//     peer.send(p).await.unwrap();
// }

// #[tokio::main]
// async fn main() -> Result<()> {
//     tracing_subscriber::fmt()
//         .with_level(true)
//         .without_time()
//         .with_env_filter(EnvFilter::from_default_env())
//         .init();
//     let config = HostConfig::new(10)?;
//     let mut host = Host::create(config, "0.0.0.0:1027").await?;

//     let mut peer_1 = poll_till_peer(&mut host).await?;
//     tracing::debug!("Got peer 1: {:?}", peer_1);
//     let mut peer_2 = poll_till_peer(&mut host).await?;
//     tracing::debug!("Got peer 2: {:?}", peer_2);

//     tokio::spawn(async move {
//         loop {
//             tokio::select! {
//                 packet = peer_1.poll() => {
//                     match packet {
//                         PeerRecvEvent::Recv(p) => {
//                             send_msg(&mut peer_1, p.clone()).await;
//                             send_msg(&mut peer_2, p).await;
//                         }
//                         PeerRecvEvent::Disconnect => return Ok::<(), ENetError>(()),
//                     }
//                 },
//                 packet = peer_2.poll() => {
//                     match packet {
//                         PeerRecvEvent::Recv(p) => {
//                             send_msg(&mut peer_1, p.clone()).await;
//                             send_msg(&mut peer_2, p).await;
//                         }
//                         PeerRecvEvent::Disconnect => return Ok::<(), ENetError>(()),
//                     }
//                 }
//             }
//         }
//     });

//     // let mut other_peers = Vec::new();
//     loop {
//         let event = host.poll().await?;
//         if let HostPollEvent::Connect(_p) = event {
//             tracing::info!("Added peer");
//             // other_peers.push(p);
//         } else {
//             tracing::info!("Got packet: {event:#x?}");
//         }
//     }
// }

fn main() {}
