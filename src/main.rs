use enet::{host::Host, Result};

use crate::enet::host::config::HostConfig;

mod enet;

#[tokio::main]
async fn main() -> Result<()> {
    let config = HostConfig::new(10)?;
    let mut host = Host::create(config, "0.0.0.0:1027").await?;

    let packet = host.poll_until_event().await?;

    println!("first: {packet:#?}");
    let packet = host.poll_until_event().await?;
    println!("second: {packet:#?}");
    let packet = host.poll_until_event().await?;
    println!("third: {packet:#?}");

    loop {
        let command = host.poll().await?;

        println!("Got packet: {command:#x?}");
    }
}
