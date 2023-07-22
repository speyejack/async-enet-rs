#![no_main]

use anyhow::{bail, Context};
use enet::{
    host::{config::HostConfig, hostevents::HostPollEvent, Host},
    net::socket::{ENetSocket, Socket},
    peer::{Packet, PeerRecvEvent},
    protocol::{Command, CommandInfo, PacketFlags, ProtocolCommand},
};
use libfuzzer_sys::fuzz_target;
use std::{
    net::{Ipv4Addr, SocketAddr},
    ops::Not,
    time::Duration,
};
use tokio::{net::UdpSocket, select};

#[derive(Debug, PartialEq, Eq, Clone, arbitrary::Arbitrary)]
enum Data {
    Cli(ProtocolCommand),
    Server(ProtocolCommand),
}

impl Not for Data {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Data::Cli(v) => Data::Server(v),
            Data::Server(v) => Data::Cli(v),
        }
    }
}

fn assert_cmd_eq(a: &Command, b: &Command) {
    assert_eq!(a.info.flags, b.info.flags);
    assert_eq!(a.info.internal_peer_id, b.info.internal_peer_id);
    assert_eq!(a.info.peer_id, b.info.peer_id);
    assert_eq!(a.info.channel_id, b.info.channel_id);
    assert_eq!(a.info.session_id, b.info.session_id);
    assert_eq!(
        a.info.reliable_sequence_number,
        b.info.reliable_sequence_number
    );
    assert_eq!(a.info.sent_time, b.info.sent_time);

    assert_eq!(a.command, b.command);
}

async fn round_trip(packets: Vec<Data>) -> Result<(), anyhow::Error> {
    // #[cfg(fuzzing)]
    // tracing_subscriber::fmt::try_init();
    let mut cli_sock = ENetSocket::new(UdpSocket::bind("127.0.0.1:9001").await?);
    let mut serv_sock = ENetSocket::new(UdpSocket::bind("127.0.0.1:9002").await?);

    let common_info = CommandInfo {
        addr: "127.0.0.1:30000".parse().unwrap(),
        flags: PacketFlags::reliable(),
        internal_peer_id: 0_u16.into(),
        peer_id: 0_u16.into(),
        channel_id: 0,
        session_id: 0,
        reliable_sequence_number: 0,
        sent_time: Duration::ZERO,
    };

    for command in packets {
        match command {
            Data::Cli(c) => {
                let mut info = common_info.clone();
                info.addr = "127.0.0.1:9002".parse().unwrap();
                let cmd = Command { info, command: c };

                cli_sock.send(&cmd).await?;

                tokio::select! {
                    _ = cli_sock.recv() => {
                        panic!("Client got unexpected packet");
                    }
                    p = serv_sock.recv() => {
                        assert_cmd_eq(&cmd, &p?);
                    }
                }
            }
            Data::Server(c) => {
                let mut info = common_info.clone();
                info.addr = "127.0.0.1:9001".parse().unwrap();
                let cmd = Command { info, command: c };

                serv_sock.send(&cmd).await?;

                tokio::select! {
                    _ = serv_sock.recv() => {
                        panic!("Server got unexpected packet");
                    }
                    p = cli_sock.recv() => {
                        assert_cmd_eq(&cmd, &p?);
                    }
                }
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
        .block_on(async { round_trip(data).await.unwrap() });
});
