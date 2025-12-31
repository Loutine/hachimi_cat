use std::str::FromStr;

use clap::{Parser, Subcommand};
use hacore::AudioEngine;
use iroh::{Endpoint, EndpointId};
use ringbuf::{HeapRb, traits::Split};

#[derive(Parser)]
#[command(name = "hacat")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Listen,
    Call { id: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let (local_prod, local_cons) = tokio::sync::mpsc::channel(100);
    let remote_buf = HeapRb::new(4);
    let (remote_prod, remote_cons) = remote_buf.split();

    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    let dht = iroh::discovery::pkarr::dht::DhtDiscovery::builder();

    let alpn = b"my-custom-protocol/1.0".to_vec();
    let alpns = vec![alpn.clone()];

    match cli.command {
        Commands::Listen => {
            let endpoint = Endpoint::builder()
                .discovery(mdns)
                .discovery(dht)
                .alpns(alpns)
                .bind()
                .await?;
            let myid = endpoint.id();
            println!("local id: {}", myid);

            let ae = AudioEngine::build(local_prod.clone(), remote_cons)?;

            while let Some(incoming) = endpoint.accept().await {
                let connecting = incoming.accept()?;
                let connection = connecting.await?;

                // let frame = connection.read_datagram().await?;
                //TODO
                // connection.send_datagram(Bytes::from(value)).await?;
            }
        }
        Commands::Call { id } => {
            let endpoint = Endpoint::builder()
                .discovery(mdns)
                .discovery(dht)
                .alpns(alpns)
                .bind()
                .await?;
            let r = endpoint.connect(EndpointId::from_str(&id)?, &alpn);

            let ae = AudioEngine::build(local_prod, remote_cons)?;
        }
    }

    tokio::signal::ctrl_c().await?;

    // Gracefully shut down the endpoint
    println!("Shutting down.");
    // router.shutdown().await?;
    Ok(())
}
